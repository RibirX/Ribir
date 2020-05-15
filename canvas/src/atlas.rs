use super::{Color, PhysicPoint, PhysicSize};
use guillotiere::*;
mod color_palette;
use color_palette::ColorPalettes;

pub(crate) struct TextureAtlas {
  pub(crate) texture: wgpu::Texture,
  pub(crate) view: wgpu::TextureView,
  atlas_allocator: AtlasAllocator,
  color_palettes: ColorPalettes,
  size: PhysicSize,
}

const INIT_SIZE: u32 = 512;
const MAX_SIZE: u32 = 4096;

pub(crate) enum AtlasStoreErr {
  /// atlas is too full to store the texture, buf the texture is good for store
  /// in the atlas if it's not store too many others.
  SpaceNotEnough,
  /// The texture you want to store in the atlas is too large, you should not
  /// try to store it again.
  OverTheMaxLimit,
}

impl TextureAtlas {
  pub(crate) fn new(device: &wgpu::Device) -> Self {
    let texture = Self::new_texture(device, INIT_SIZE, INIT_SIZE);

    let mut atlas_allocator =
      AtlasAllocator::new(size2(INIT_SIZE as i32, INIT_SIZE as i32));
    TextureAtlas {
      view: texture.create_default_view(),
      texture,
      color_palettes: ColorPalettes::new(&mut atlas_allocator),
      atlas_allocator,
      size: PhysicSize::new(INIT_SIZE, INIT_SIZE),
    }
  }

  /// Store the `color` in, return the position in the texture of the color and
  /// if the atlas has grown as a Some-Value pair. if three isn't enough space
  /// to store, return None-Value.
  pub(crate) fn store_color_in_palette(
    &mut self,
    color: Color,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
  ) -> Result<(PhysicPoint, bool), AtlasStoreErr> {
    macro store_color($grow: ident) {
      self
        .color_palettes
        .store_color_in_palette(
          color,
          &self.texture,
          &mut self.atlas_allocator,
          device,
          encoder,
        )
        .map(|v| (v, $grow))
    }

    store_color!(false)
      .or_else(|| {
        if self.size.height * 2 <= MAX_SIZE {
          let mut size = self.size;
          size.height *= 2;
          self.grow_texture(size, device, encoder);
          store_color!(true)
        } else if self.size.width < MAX_SIZE {
          let mut size = self.size;
          size.width *= 2;
          self.grow_texture(size, device, encoder);
          store_color!(true)
        } else {
          None
        }
      })
      .ok_or_else(|| AtlasStoreErr::SpaceNotEnough)
  }

  #[inline]
  pub(crate) fn size(&self) -> PhysicSize { self.size }

  /// Flush all data to the texture and ready to commit to gpu.
  /// Call this function before commit drawing to gpu.
  pub(crate) fn flush(
    &mut self,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
  ) {
    self.color_palettes.save_current_palette_to_texture(
      &self.texture,
      device,
      encoder,
    );
  }

  /// Clear the atlas.
  pub(crate) fn clear(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
    self.atlas_allocator.clear();
    self.color_palettes = ColorPalettes::new(&mut self.atlas_allocator);

    let mut encoder =
      device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Render Encoder"),
      });
    {
      encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
          attachment: &self.view,
          resolve_target: None,
          load_op: wgpu::LoadOp::Clear,
          store_op: wgpu::StoreOp::Store,
          clear_color: wgpu::Color::TRANSPARENT,
        }],
        depth_stencil_attachment: None,
      });
    }
    queue.submit(Some(encoder.finish()));
  }

  fn grow_texture(
    &mut self,
    size: PhysicSize,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
  ) {
    self.atlas_allocator.grow(size.to_i32().to_untyped());
    let new_texture = Self::new_texture(device, size.width, size.height);

    encoder.copy_texture_to_texture(
      wgpu::TextureCopyView {
        texture: &self.texture,
        mip_level: 0,
        array_layer: 0,
        origin: wgpu::Origin3d { x: 0, y: 0, z: 0 },
      },
      wgpu::TextureCopyView {
        texture: &new_texture,
        mip_level: 0,
        array_layer: 0,
        origin: wgpu::Origin3d { x: 0, y: 0, z: 0 },
      },
      wgpu::Extent3d {
        width: self.size.width,
        height: self.size.height,
        depth: 1,
      },
    );

    self.size = size;
    self.texture = new_texture;
    self.view = self.texture.create_default_view();
  }

  fn new_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
  ) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
      label: Some("Canvas texture atlas"),
      size: wgpu::Extent3d {
        width,
        height,
        depth: 1,
      },
      dimension: wgpu::TextureDimension::D2,
      format: wgpu::TextureFormat::Bgra8UnormSrgb,
      usage: wgpu::TextureUsage::COPY_DST
        | wgpu::TextureUsage::SAMPLED
        | wgpu::TextureUsage::COPY_SRC,
      mip_level_count: 1,
      sample_count: 1,
    })
  }
}
