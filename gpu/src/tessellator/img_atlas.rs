use super::{atlas::Atlas, PATH_ATLAS_ID};
use crate::{ColorFormat, GpuTessellatorHelper};
use guillotiere::AtlasAllocator;
use ribir_painter::{AntiAliasing, DeviceRect, DeviceSize, ShallowImage, TextureCfg, TextureX};

pub(crate) struct ImgAtlas<T: TextureX>(Atlas<ShallowImage, T>);

impl<T: TextureX> ImgAtlas<T> {
  pub fn new(size: DeviceSize, helper: &mut impl GpuTessellatorHelper<Texture = T>) -> Self {
    let atlas_allocator = AtlasAllocator::new(size.to_i32().cast_unit());
    let texture = helper.new_texture(
      PATH_ATLAS_ID,
      TextureCfg {
        format: ColorFormat::Rgba8,
        size,
        anti_aliasing: AntiAliasing::None,
      },
    );
    let atlas = Atlas::new(texture);
    Self(atlas)
  }

  /// Store a image in the atlas, and return the rect of its place.
  pub fn store_image(&mut self, image: &ShallowImage) -> Option<DeviceRect> {
    assert_eq!(image.color_format(), ColorFormat::Rgba8);
    let (w, h) = image.size();
    let size = DeviceSize::new(w as u32, h as u32);
    let new_alloc = false;
    self
      .0
      .store(image.clone(), size, |img| Some(img.pixel_bytes()))
  }

  /// deallocate all last recently not used allocation.
  pub fn end_frame(&mut self) { self.0.end_frame() }
}

#[cfg(test)]
pub mod tests {
  use super::*;
  use ribir_painter::Color;
  use std::borrow::Cow;

  const MAX_SIZE: Size = Size::new(1024, 1024);

  pub fn color_image(color: Color, width: u16, height: u16) -> ShallowImage {
    let data = std::iter::repeat(color.into_components())
      .take(width as usize * height as usize)
      .flatten()
      .collect::<Vec<_>>();

    let img = PixelImage::new(Cow::Owned(data), width, height, ColorFormat::Rgba8);
    ShallowImage::new(img)
  }

  #[test]
  fn grow_alloc_keep() {
    let mut atlas = ImgAtlas::new(Size::new(64, 64), MAX_SIZE);
    let red_img = color_image(Color::RED, 32, 32);
    let red_rect = atlas.store_image(&red_img).unwrap();

    assert_eq!(red_rect.min().to_array(), [0, 0]);

    // same image should have same position in atlas
    assert_eq!(red_rect, atlas.store_image(&red_img).unwrap());
    color_img_check(&atlas, &red_rect, Color::RED);

    let yellow_img = color_image(Color::YELLOW, 64, 64);
    let yellow_rect = atlas.store_image(&yellow_img).unwrap();

    // the color should keep after atlas rearrange
    color_img_check(&atlas, &red_rect, Color::RED);
    color_img_check(&atlas, &yellow_rect, Color::YELLOW);
  }

  fn color_img_check(atlas: &ImgAtlas, rect: &Rect, color: Color) {
    const UNIT: usize = ImgAtlas::UNIT;
    let rect = rect.to_usize();
    rect.y_range().for_each(|y| {
      rect.x_range().for_each(|x| {
        assert_eq!(
          atlas.texture[y][UNIT * x..UNIT * (x + 1)],
          color.into_components()
        );
      })
    })
  }
}
