use super::{surface::Texture, Color, DevicePoint, DeviceSize};
use guillotiere::*;
mod color_palette;
use color_palette::ColorPalettes;

pub(crate) struct TextureAtlas {
  pub(crate) texture: Texture,
  pub(crate) view: wgpu::TextureView,
  atlas_allocator: AtlasAllocator,
  color_palettes: ColorPalettes,
}

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
    const INIT: u32 = Texture::INIT_DIMENSION;
    let size = DeviceSize::new(INIT, INIT);
    let mut atlas_allocator = AtlasAllocator::new(size.cast_unit().to_i32());
    let texture = Texture::new(
      device,
      size,
      wgpu::TextureUsage::COPY_DST | wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_SRC,
    );
    TextureAtlas {
      view: texture.raw_texture.create_default_view(),
      texture,
      color_palettes: ColorPalettes::new(&mut atlas_allocator),
      atlas_allocator,
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
    queue: &wgpu::Queue,
  ) -> Result<(DevicePoint, bool), AtlasStoreErr> {
    macro store_color($grow: ident) {
      self
        .color_palettes
        .store_color_in_palette(
          color.clone(),
          &self.texture.raw_texture,
          &mut self.atlas_allocator,
          device,
          encoder,
        )
        .map(|v| (v, $grow))
    }

    store_color!(false)
      .or_else(|| {
        let mut size = self.texture.size();
        if size.height * 2 <= Texture::MAX_DIMENSION {
          size.height *= 2;
          self.grow_texture(size, device, queue);
          store_color!(true)
        } else if size.width < Texture::MAX_DIMENSION {
          size.width *= 2;
          self.grow_texture(size, device, queue);
          store_color!(true)
        } else {
          None
        }
      })
      .ok_or_else(|| AtlasStoreErr::SpaceNotEnough)
  }

  #[inline]
  pub(crate) fn size(&self) -> DeviceSize { self.texture.size() }

  /// Flush all data to the texture and ready to commit to gpu.
  /// Call this function before commit drawing to gpu.
  pub(crate) fn flush(&mut self, device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder) {
    self
      .color_palettes
      .save_current_palette_to_texture(&self.texture.raw_texture, device, encoder);
  }

  /// Clear the atlas.
  pub(crate) fn clear(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
    self.atlas_allocator.clear();
    self.color_palettes = ColorPalettes::new(&mut self.atlas_allocator);

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
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

  fn grow_texture(&mut self, size: DeviceSize, device: &wgpu::Device, queue: &wgpu::Queue) {
    self.atlas_allocator.grow(size.to_i32().to_untyped());
    self.texture.resize(device, queue, size);
    self.view = self.texture.raw_texture.create_default_view();
  }
}
