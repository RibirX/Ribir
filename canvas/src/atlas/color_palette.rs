use super::{Color, DevicePoint};
use guillotiere::*;
use zerocopy::AsBytes;

const PALETTE_SIZE: u32 = DEFAULT_OPTIONS.small_size_threshold as u32;

pub(crate) struct ColorPalettes {
  indexed_colors: std::collections::HashMap<u32, DevicePoint>,
  current_palette: Palette,
  current_alloc: Allocation,
}

impl ColorPalettes {
  pub(crate) fn new(atlas: &mut AtlasAllocator) -> Self {
    let current_allocation = Self::allocate_palette(atlas).expect("init palettes space must have.");
    Self {
      indexed_colors: Default::default(),
      current_palette: Default::default(),
      current_alloc: current_allocation,
    }
  }
  /// store a color in palette, and return the color position of the texture
  pub(crate) fn store_color_in_palette(
    &mut self,
    color: Color,
    texture: &wgpu::Texture,
    atlas_allocator: &mut AtlasAllocator,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
  ) -> Option<DevicePoint> {
    if let Some(pos) = self.indexed_colors.get(&color_hash(color)) {
      return Some(*pos);
    }
    if !self.current_palette.is_fulled() {
      let pos = self.add_color(color);
      return Some(pos);
    }

    // We need create a new palette to store color
    self.save_current_palette_to_texture(texture, device, encoder);
    self.current_alloc = Self::allocate_palette(atlas_allocator)?;
    self.current_palette = Default::default();

    Some(self.add_color(color))
  }

  /// Copy current palette to texture
  pub(crate) fn save_current_palette_to_texture(
    &self,
    texture: &wgpu::Texture,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
  ) {
    let buffer = device.create_buffer_with_data(
      self.current_palette.store.as_bytes(),
      wgpu::BufferUsage::COPY_SRC,
    );

    let origin = self.current_alloc.rectangle.min;
    encoder.copy_buffer_to_texture(
      wgpu::BufferCopyView {
        buffer: &buffer,
        offset: 0,
        bytes_per_row: PALETTE_SIZE * std::mem::size_of::<u32>() as u32,
        rows_per_image: PALETTE_SIZE,
      },
      wgpu::TextureCopyView {
        texture,
        mip_level: 0,
        array_layer: 0,
        origin: wgpu::Origin3d {
          x: origin.x as u32,
          y: origin.y as u32,
          z: 0,
        },
      },
      wgpu::Extent3d {
        width: PALETTE_SIZE,
        height: PALETTE_SIZE,
        depth: 1,
      },
    );
  }

  fn add_color(&mut self, color: Color) -> DevicePoint {
    let offset = self.current_palette.add_color(color);
    let pos = self.current_alloc.rectangle.min + offset;
    let pos = DevicePoint::new(pos.x as u32, pos.y as u32);
    self.indexed_colors.insert(color_hash(color), pos);
    pos
  }

  fn allocate_palette(atlas: &mut AtlasAllocator) -> Option<Allocation> {
    atlas.allocate(Size::new(PALETTE_SIZE as i32, PALETTE_SIZE as i32))
  }
}

#[inline]
fn color_hash(color: Color) -> u32 { unsafe { std::mem::transmute_copy(&color) } }

#[inline]
fn color_as_bgra(color: Color) -> u32 {
  unsafe { std::mem::transmute_copy(&[color.blue, color.green, color.red, color.alpha]) }
}

#[derive(Default)]
struct Palette {
  store: [[u32; PALETTE_SIZE as usize]; PALETTE_SIZE as usize],
  size: u32,
}

type PaletteVector = euclid::Vector2D<i32, euclid::UnknownUnit>;
impl Palette {
  #[inline]
  fn is_fulled(&self) -> bool { self.size >= PALETTE_SIZE ^ 2 }

  /// This function not check if the platte fulled, caller should check it
  /// before add.
  fn add_color(&mut self, color: Color) -> PaletteVector {
    let index = self.size;
    let row = index / PALETTE_SIZE;
    let col = index % PALETTE_SIZE;
    self.store[row as usize][col as usize] = color_as_bgra(color);
    let pos = PaletteVector::new(col as i32, row as i32);
    self.size += index + 1;

    pos
  }
}
