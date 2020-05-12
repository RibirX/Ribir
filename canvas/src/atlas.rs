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

impl TextureAtlas {
  pub(crate) fn new(device: &wgpu::Device) -> Self {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
      label: Some("Canvas texture atlas"),
      size: wgpu::Extent3d {
        width: INIT_SIZE,
        height: INIT_SIZE,
        depth: 1,
      },
      array_layer_count: 1,
      dimension: wgpu::TextureDimension::D2,
      format: wgpu::TextureFormat::Bgra8UnormSrgb,
      usage: wgpu::TextureUsage::COPY_DST | wgpu::TextureUsage::SAMPLED,
      mip_level_count: 1,
      sample_count: 1,
    });

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
  ) -> Option<(PhysicPoint, bool)> {
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

    store_color!(false).or_else(|| {
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
  }

  fn grow_texture(
    &mut self,
    size: PhysicSize,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
  ) {
    self.atlas_allocator.grow(size.to_i32().to_untyped());
    let new_texture = device.create_texture(&wgpu::TextureDescriptor {
      label: Some("Canvas texture atlas"),
      size: wgpu::Extent3d {
        width: INIT_SIZE,
        height: INIT_SIZE,
        depth: 1,
      },
      array_layer_count: 1,
      dimension: wgpu::TextureDimension::D2,
      format: wgpu::TextureFormat::Bgra8UnormSrgb,
      usage: wgpu::TextureUsage::COPY_DST | wgpu::TextureUsage::SAMPLED,
      mip_level_count: 1,
      sample_count: 1,
    });

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
}
