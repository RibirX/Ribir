use crate::{tessellator::Tessellator, GlRender, GpuBackend, TriangleLists, Vertex};
use futures::executor::block_on;
use ribir_painter::DeviceSize;
use ribir_text::shaper::TextShaper;
use std::{error::Error, iter};
mod color_pass;
mod stencil_pass;
pub mod surface;

use surface::{Surface, TextureSurface, WindowSurface};
use wgpu::util::DeviceExt;

use zerocopy::AsBytes;
mod img_pass;
use self::{color_pass::ColorPass, img_pass::ImagePass, stencil_pass::StencilPass};

const TEXTURE_INIT_SIZE: (u16, u16) = (1024, 1024);
const TEXTURE_MAX_SIZE: (u16, u16) = (4096, 4096);

/// create wgpu backend with window
pub async fn wgpu_backend_with_wnd<W: raw_window_handle::HasRawWindowHandle>(
  window: &W,
  size: DeviceSize,
  tex_init_size: Option<(u16, u16)>,
  tex_max_size: Option<(u16, u16)>,
  shaper: TextShaper,
) -> GpuBackend<WgpuGl> {
  let init_size = tex_init_size.unwrap_or(TEXTURE_INIT_SIZE);
  let max_size = tex_max_size.unwrap_or(TEXTURE_MAX_SIZE);
  let tessellator = Tessellator::new(init_size, max_size, shaper);
  let gl = WgpuGl::from_wnd(window, size, AntiAliasing::Msaa4X).await;

  GpuBackend { tessellator, gl }
}

/// create wgpu backend windowless
pub async fn wgpu_backend_headless(
  size: DeviceSize,
  tex_init_size: Option<(u16, u16)>,
  tex_max_size: Option<(u16, u16)>,
  shaper: TextShaper,
) -> GpuBackend<WgpuGl<surface::TextureSurface>> {
  let init_size = tex_init_size.unwrap_or(TEXTURE_INIT_SIZE);
  let max_size = tex_max_size.unwrap_or(TEXTURE_MAX_SIZE);
  let tessellator = Tessellator::new(init_size, max_size, shaper);
  let gl = WgpuGl::headless(size).await;
  GpuBackend { tessellator, gl }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AntiAliasing {
  None = 1,
  Msaa2X = 2,
  Msaa4X = 4,
  Msaa8X = 8,
  Msaa16X = 16,
}

pub struct WgpuGl<S: Surface = WindowSurface> {
  device: wgpu::Device,
  queue: wgpu::Queue,
  surface: S,
  color_pass: ColorPass,
  img_pass: ImagePass,
  stencil_pass: StencilPass,
  coordinate_matrix: wgpu::Buffer,
  primitives_layout: wgpu::BindGroupLayout,
  vertex_buffers: Option<VertexBuffers>,
  anti_aliasing: AntiAliasing,
  multisample_framebuffer: Option<wgpu::TextureView>,
  size: DeviceSize,
  /// if the frame already draw something.
  empty_frame: bool,
  stencil_cnt: u32,
}
struct VertexBuffers {
  vertices: wgpu::Buffer,
  vertex_size: usize,
  indices: wgpu::Buffer,
  index_size: usize,
}

impl WgpuGl<WindowSurface> {
  /// Create a canvas and bind to a native window, its size is `width` and
  /// `height`. If you want to create a headless window, use
  /// [`headless_render`](WgpuRender::headless_render).
  pub async fn from_wnd<W: raw_window_handle::HasRawWindowHandle>(
    window: &W,
    size: DeviceSize,
    anti_aliasing: AntiAliasing,
  ) -> Self {
    let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);

    let w_surface = unsafe { instance.create_surface(window) };

    let adapter = instance
      .request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: Some(&w_surface),
        force_fallback_adapter: false,
      })
      .await
      .unwrap();

    Self::new(
      size,
      &adapter,
      |device| WindowSurface::new(w_surface, device, size),
      anti_aliasing,
    )
    .await
  }
}

impl WgpuGl<TextureSurface> {
  /// Create a headless wgpu render, if you want to bind to a window, use
  /// [`wnd_render`](WgpuRender::wnd_render).
  pub async fn headless(size: DeviceSize) -> Self {
    let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);

    let adapter = instance
      .request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: None,
        force_fallback_adapter: false,
      })
      .await
      .unwrap();

    WgpuGl::new(
      size,
      &adapter,
      |device| {
        TextureSurface::new(
          device,
          size,
          wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        )
      },
      AntiAliasing::None,
    )
    .await
  }
}

impl<S: Surface> GlRender for WgpuGl<S> {
  fn begin_frame(&mut self) { self.empty_frame = true; }

  fn add_texture(&mut self, texture: crate::Texture) {
    self
      .img_pass
      .add_texture(texture, &self.device, &self.queue)
  }

  fn draw_triangles(&mut self, data: TriangleLists) {
    self.write_vertex_buffer(data.vertices, data.indices);
    let vertex_buffers = self.vertex_buffers.as_ref().unwrap();

    let mut encoder = self.create_command_encoder();
    let prim_bind_group = self.create_primitives_bind_group(data.primitives);

    let sample_count = self.multi_sample_count();
    let Self {
      device,
      coordinate_matrix,
      color_pass,
      img_pass,
      stencil_pass,
      ..
    } = self;

    let uniforms = data
      .commands
      .iter()
      .filter_map(|cmd| match cmd {
        crate::DrawTriangles::Texture { texture_id, .. } => {
          let uniform = img_pass.create_texture_uniform(device, *texture_id, coordinate_matrix);
          Some((texture_id, uniform))
        }
        _ => None,
      })
      .collect::<std::collections::HashMap<_, _>>();

    let view = self
      .surface
      .current_texture()
      .create_view(&wgpu::TextureViewDescriptor::default());

    let size_extend = wgpu::Extent3d {
      width: self.size.width,
      height: self.size.height,
      depth_or_array_layers: 1,
    };

    let stencil_texture = device.create_texture(&wgpu::TextureDescriptor {
      label: Some("stencil"),
      size: size_extend,
      mip_level_count: 1,
      sample_count,
      dimension: wgpu::TextureDimension::D2,
      format: wgpu::TextureFormat::Depth24PlusStencil8,
      usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
    });

    {
      let (view, resolve_target, store) = self.multisample_framebuffer.as_ref().map_or_else(
        || (&view, None, true),
        |multi_sample| (multi_sample, Some(&view), false),
      );
      let load = if self.empty_frame {
        wgpu::LoadOp::Clear(wgpu::Color::WHITE)
      } else {
        wgpu::LoadOp::Load
      };
      let ops = wgpu::Operations { load, store };
      let rpass_color_attachment = wgpu::RenderPassColorAttachment { view, resolve_target, ops };

      let load_stencil = if self.empty_frame {
        wgpu::LoadOp::Clear(0)
      } else {
        wgpu::LoadOp::Load
      };
      let stencil_view = stencil_texture.create_view(&wgpu::TextureViewDescriptor::default());
      let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Triangles render pass"),
        color_attachments: &[rpass_color_attachment],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
          view: &stencil_view,
          depth_ops: None,
          stencil_ops: Some(wgpu::Operations { load: load_stencil, store: true }),
        }),
      });
      render_pass.set_vertex_buffer(0, vertex_buffers.vertices.slice(..));
      render_pass.set_index_buffer(vertex_buffers.indices.slice(..), wgpu::IndexFormat::Uint32);
      render_pass.set_bind_group(1, &prim_bind_group, &[]);
      data.commands.iter().for_each(|cmd| {
        let stencil_reference = self.stencil_cnt;
        match cmd {
          crate::DrawTriangles::Color(rg) => {
            render_pass.set_pipeline(&color_pass.pipeline);
            render_pass.set_bind_group(0, &color_pass.uniform, &[]);
            render_pass.set_stencil_reference(stencil_reference);
            render_pass.draw_indexed(rg.clone(), 0, 0..1);
          }
          crate::DrawTriangles::Texture { rg, texture_id } => {
            render_pass.set_pipeline(&img_pass.pipeline);
            render_pass.set_bind_group(0, uniforms.get(texture_id).unwrap(), &[]);
            render_pass.set_stencil_reference(stencil_reference);
            render_pass.draw_indexed(rg.clone(), 0, 0..1);
          }
          crate::DrawTriangles::PushStencil(rg) => {
            render_pass.set_pipeline(&stencil_pass.push_stencil_pipeline);
            render_pass.set_bind_group(0, &stencil_pass.uniform, &[]);
            render_pass.set_stencil_reference(stencil_reference);
            render_pass.draw_indexed(rg.clone(), 0, 0..1);
            self.stencil_cnt += 1;
          }
          crate::DrawTriangles::PopStencil(rg) => {
            render_pass.set_pipeline(&stencil_pass.pop_stencil_pipeline);
            render_pass.set_bind_group(0, &stencil_pass.uniform, &[]);
            render_pass.set_stencil_reference(stencil_reference);
            render_pass.draw_indexed(rg.clone(), 0, 0..1);
            self.stencil_cnt -= 1;
          }
        }
      });
    }
    self.empty_frame = false;

    self.queue.submit(iter::once(encoder.finish()));
  }

  fn end_frame<'a>(&mut self, cancel: bool) {
    if !cancel {
      self.surface.present();
    }
    self.img_pass.end_frame();
  }

  fn resize(&mut self, size: DeviceSize) {
    self.size = size;
    self.surface.resize(&self.device, &self.queue, size);
    self.coordinate_matrix = coordinate_matrix_buffer_2d(&self.device, size.width, size.height);
    self
      .color_pass
      .resize(&self.coordinate_matrix, &self.device);
    self
      .stencil_pass
      .resize(&self.coordinate_matrix, &self.device);
    self.multisample_framebuffer = Self::multisample_framebuffer(
      &self.device,
      size,
      self.surface.format(),
      self.multi_sample_count(),
    );
  }

  fn capture(&self, capture: ribir_painter::CaptureCallback) -> Result<(), Box<dyn Error>> {
    let mut encoder = self.create_command_encoder();
    let buffer = self.surface.copy_as_rgba_buffer(&self.device, &mut encoder);
    self.queue.submit(iter::once(encoder.finish()));

    let buffer_slice = buffer.slice(..);
    let buffer_future = buffer_slice.map_async(wgpu::MapMode::Read);

    // Poll the device in a blocking manner so that our future resolves.
    self.device.poll(wgpu::Maintain::Wait);
    block_on(buffer_future)?;

    let size = self.surface.view_size();
    let slice = buffer_slice.get_mapped_range();
    let buffer_bytes_per_row = slice.len() as u32 / size.height;
    let img_bytes_pre_row = (size.width * 4) as usize;
    let rows = (0..size.height).map(|i| {
      let offset = (i * buffer_bytes_per_row) as usize;
      &slice.as_ref()[offset..offset + img_bytes_pre_row]
    });

    capture(size, Box::new(rows));
    Ok(())
  }
}

impl<S: Surface> WgpuGl<S> {
  async fn new<C>(
    size: DeviceSize,
    adapter: &wgpu::Adapter,
    surface_ctor: C,
    anti_aliasing: AntiAliasing,
  ) -> WgpuGl<S>
  where
    C: FnOnce(&wgpu::Device) -> S,
  {
    let (device, queue) = adapter
      .request_device(
        &wgpu::DeviceDescriptor {
          label: Some("Request device"),
          features: wgpu::Features::empty(),
          limits: Default::default(),
        },
        None,
      )
      .await
      .unwrap();

    let surface = surface_ctor(&device);

    let primitive_layout = primitives_layout(&device);
    let coordinate_matrix = coordinate_matrix_buffer_2d(&device, size.width, size.height);

    let msaa_count = anti_aliasing as u32;
    let color_pass = ColorPass::new(
      &device,
      surface.format(),
      &coordinate_matrix,
      &primitive_layout,
      msaa_count,
    );
    let texture_pass = ImagePass::new(&device, surface.format(), &primitive_layout, msaa_count);
    let stencil_pass = StencilPass::new(
      &device,
      surface.format(),
      &coordinate_matrix,
      &primitive_layout,
      msaa_count,
    );

    let multisample_framebuffer =
      Self::multisample_framebuffer(&device, size, surface.format(), msaa_count);
    WgpuGl {
      device,
      surface,
      queue,
      size,
      color_pass,
      img_pass: texture_pass,
      stencil_pass,
      coordinate_matrix,
      primitives_layout: primitive_layout,
      empty_frame: true,
      vertex_buffers: None,
      anti_aliasing,
      multisample_framebuffer,
      stencil_cnt: 0,
    }
  }

  #[inline]
  pub fn set_anti_aliasing(&mut self, anti_aliasing: AntiAliasing) {
    if self.anti_aliasing != anti_aliasing {
      let Self {
        color_pass,
        img_pass,
        primitives_layout,
        surface,
        device,
        stencil_pass,
        ..
      } = self;
      self.anti_aliasing = anti_aliasing;
      let msaa_count = anti_aliasing as u32;
      let format = surface.format();
      color_pass.set_anti_aliasing(msaa_count, primitives_layout, device, format);
      img_pass.set_anti_aliasing(msaa_count, primitives_layout, device, format);
      stencil_pass.set_anti_aliasing(msaa_count, primitives_layout, device, format);
    }
  }

  fn create_command_encoder(&self) -> wgpu::CommandEncoder {
    self
      .device
      .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Create Encoder") })
  }

  fn multisample_framebuffer(
    device: &wgpu::Device,
    size: DeviceSize,
    format: wgpu::TextureFormat,
    sample_count: u32,
  ) -> Option<wgpu::TextureView> {
    (sample_count > 1).then(|| {
      let multisampled_texture_extent = wgpu::Extent3d {
        width: size.width,
        height: size.height,
        depth_or_array_layers: 1,
      };

      let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
        size: multisampled_texture_extent,
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        label: None,
      };

      device
        .create_texture(multisampled_frame_descriptor)
        .create_view(&wgpu::TextureViewDescriptor::default())
    })
  }

  fn create_primitives_bind_group<T: AsBytes>(&self, primitives: &[T]) -> wgpu::BindGroup {
    let primitives_buffer = self
      .device
      .create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Primitive Buffer"),
        contents: primitives.as_bytes(),
        usage: wgpu::BufferUsages::STORAGE,
      });
    self.device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout: &self.primitives_layout,
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: wgpu::BindingResource::Buffer(primitives_buffer.as_entire_buffer_binding()),
      }],
      label: Some("Primitive buffer bind group"),
    })
  }

  fn multi_sample_count(&self) -> u32 { self.anti_aliasing as u32 }

  fn write_vertex_buffer(&mut self, vertices: &[Vertex], indices: &[u32]) {
    let Self { device, vertex_buffers, .. } = self;
    let new_vertex_buffer = || {
      device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertices buffer"),
        contents: vertices.as_bytes(),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
      })
    };
    let new_index_buffer = || {
      device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        contents: indices.as_bytes(),
        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        label: Some("Indices buffer"),
      })
    };

    if let Some(buffers) = vertex_buffers {
      if buffers.vertex_size >= vertices.len() {
        self
          .queue
          .write_buffer(&buffers.vertices, 0, vertices.as_bytes());
      } else {
        buffers.vertices = new_vertex_buffer();
      }
      buffers.vertex_size = vertices.len();
      if buffers.index_size >= indices.len() {
        self
          .queue
          .write_buffer(&buffers.indices, 0, indices.as_bytes())
      } else {
        buffers.indices = new_index_buffer();
      }
      buffers.index_size = indices.len();
    } else {
      *vertex_buffers = Some(VertexBuffers {
        vertices: new_vertex_buffer(),
        vertex_size: vertices.len(),
        indices: new_index_buffer(),
        index_size: indices.len(),
      });
    }
  }
}

fn primitives_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
  device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    entries: &[wgpu::BindGroupLayoutEntry {
      binding: 0,
      visibility: wgpu::ShaderStages::VERTEX,
      ty: wgpu::BindingType::Buffer {
        ty: wgpu::BufferBindingType::Storage { read_only: true },
        has_dynamic_offset: false,
        min_binding_size: None,
      },
      count: None,
    }],
    label: Some("Primitive layout (maybe changed every draw)"),
  })
}

fn coordinate_matrix_buffer_2d(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Buffer {
  device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
    contents: [
      [2. / width as f32, 0., 0., 0.],
      [0., -2. / height as f32, 0., 0.],
      [0., 0., 1., 0.],
      [-1., 1., 0., 1.],
    ]
    .as_bytes(),
    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    label: Some("2d coordinate transform buffer."),
  })
}

impl Vertex {
  fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
    use std::mem::size_of;
    wgpu::VertexBufferLayout {
      array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
      step_mode: wgpu::VertexStepMode::Vertex,
      attributes: &[
        wgpu::VertexAttribute {
          offset: 0,
          shader_location: 0,
          format: wgpu::VertexFormat::Float32x2,
        },
        wgpu::VertexAttribute {
          offset: (size_of::<[f32; 2]>()) as wgpu::BufferAddress,
          shader_location: 1,
          format: wgpu::VertexFormat::Uint32,
        },
      ],
    }
  }
}

#[cfg(test)]
mod test {
  use crate::wgpu_backend_headless;
  use ribir::prelude::AppContext;
  use ribir_painter::{
    Brush, CaptureCallback, ClipInstruct, Color, DeviceSize, PaintCommand, PaintInstruct,
    PaintPath, PainterBackend, Path, Point, Transform, Vector,
  };
  use std::{cell::RefCell, rc::Rc};

  fn compare_paint_result(
    size: DeviceSize,
    commands1: Vec<PaintCommand>,
    commands2: Vec<PaintCommand>,
  ) {
    let ctx = Rc::new(RefCell::new(AppContext::default()));
    let mut p_backend = futures::executor::block_on(wgpu_backend_headless(
      size,
      None,
      None,
      ctx.borrow().shaper.clone(),
    ));

    fn capture_buf() -> (Rc<RefCell<Vec<u8>>>, CaptureCallback<'static>) {
      let v = Rc::new(RefCell::new(Vec::default()));
      let data = v.clone();
      let receiver: CaptureCallback<'static> = Box::new(move |size, rows| {
        v.borrow_mut()
          .reserve((4 * size.width * size.height) as usize);
        rows.for_each(|r| v.borrow_mut().extend(r));
      });
      return (data, receiver);
    }

    let (buf1, receiver1) = capture_buf();
    p_backend.commands_to_image(commands1, receiver1).unwrap();

    let (buf2, receiver2) = capture_buf();
    p_backend.commands_to_image(commands2, receiver2).unwrap();

    assert_eq!(*buf1.borrow(), *buf2.borrow());
  }

  fn fill_path(path: &Path, transform: &Transform, color: ribir_painter::Color) -> PaintCommand {
    PaintCommand::Paint(PaintInstruct {
      path: PaintPath::Path(path.clone().into()),
      opacity: 1.,
      transform: transform.clone(),
      brush: Brush::Color(color),
    })
  }

  fn push_clip(path: &Path, transform: &Transform) -> PaintCommand {
    PaintCommand::PushClip(ClipInstruct {
      path: PaintPath::Path(path.clone().into()),
      transform: transform.clone(),
    })
  }

  fn pop_clip() -> PaintCommand { PaintCommand::PopClip }

  fn full_rect(width: f32, height: f32) -> Path {
    let mut builder = Path::builder();
    builder
      .begin_path(Point::zero())
      .line_to(Point::new(width, 0.))
      .line_to(Point::new(width, height))
      .line_to(Point::new(0., height))
      .end_path(true);
    builder.fill()
  }

  #[test]
  fn stencil_clip() {
    let size = DeviceSize::new(100, 100);
    let ident = Transform::default();
    let full_path = full_rect(100., 100.);

    // simple clip
    let commands1 = vec![
      push_clip(&full_path, &Transform::scale(0.5, 0.5)),
      fill_path(&full_path, &ident, Color::BLUE),
      pop_clip(),
    ];
    let commands2 = vec![fill_path(
      &full_path,
      &Transform::scale(0.5, 0.5),
      Color::BLUE,
    )];
    compare_paint_result(size, commands1, commands2);

    // clip embed
    let commands3 = vec![
      push_clip(&full_path, &Transform::scale(0.75, 0.75)),
      push_clip(
        &full_path,
        &Transform::scale(0.75, 0.75).then_translate(Vector::new(25., 25.)),
      ),
      fill_path(&full_path, &ident, Color::BLUE),
      pop_clip(),
      fill_path(&full_path, &Transform::scale(1., 0.15), Color::YELLOW),
      fill_path(
        &full_path,
        &Transform::scale(0.25, 0.25).then_translate(Vector::new(75., 75.)),
        Color::RED,
      ),
      pop_clip(),
    ];
    let commands4 = vec![
      fill_path(
        &full_path,
        &Transform::scale(0.5, 0.5).then_translate(Vector::new(25., 25.)),
        Color::BLUE,
      ),
      fill_path(&full_path, &Transform::scale(0.75, 0.15), Color::YELLOW),
    ];
    compare_paint_result(size, commands3, commands4);
  }
}
