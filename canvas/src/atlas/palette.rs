use super::Color;
use guillotiere::*;

pub const PALETTE_SIZE: u32 = DEFAULT_OPTIONS.small_size_threshold as u32;

pub fn color_as_bgra(color: Color) -> u32 {
  let [r, g, b, a] = color.into_raw();
  unsafe { std::mem::transmute_copy(&[b, g, r, a]) }
}

#[derive(Default)]
pub struct Palette {
  store: [[u32; (PALETTE_SIZE) as usize]; PALETTE_SIZE as usize],
  size: u32,
}

type PaletteVector = euclid::Vector2D<i32, euclid::UnknownUnit>;

impl Palette {
  #[inline]
  pub fn is_fulled(&self) -> bool { self.size >= PALETTE_SIZE * PALETTE_SIZE }

  /// This function not check if the platte fulled, caller should check it
  /// before add.
  pub fn add_color(&mut self, color: Color) -> PaletteVector {
    let index = self.size;
    let row = index / PALETTE_SIZE;
    let col = index % PALETTE_SIZE;
    self.store[row as usize][col as usize] = color_as_bgra(color);
    let pos = PaletteVector::new(col as i32, row as i32);
    self.size += index + 1;

    pos
  }

  #[inline]
  pub fn clear(&mut self) {
    self.store = <_>::default();
    self.size = 0;
  }

  #[inline]
  pub fn data(&self) -> &[u32] {
    unsafe {
      std::slice::from_raw_parts(
        &self.store as *const _ as *const u32,
        (PALETTE_SIZE * PALETTE_SIZE) as usize,
      )
    }
  }

  #[inline]
  pub fn stored_color_size(&self) -> u32 { self.size }
}
