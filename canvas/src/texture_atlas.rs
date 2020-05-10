use guillotiere::*;
pub(crate) struct TextureAtlas {
  pub(crate) texture: wgpu::Texture,
  pub(crate) view: wgpu::TextureView,
  atlas_allocator: AtlasAllocator,
}

const INIT_SIZE: u32 = 512;
const MAX_SIZE: u32 = 2048;

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

    TextureAtlas {
      view: texture.create_default_view(),
      texture,
      atlas_allocator: AtlasAllocator::new(size2(
        INIT_SIZE as i32,
        INIT_SIZE as i32,
      )),
    }
  }
}
