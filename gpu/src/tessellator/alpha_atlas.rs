use super::{atlas::Atlas, AlphaItem, IdPath, PATH_ATLAS_ID};
use crate::{ColorFormat, GpuTessellatorHelper};
use guillotiere::{Allocation, AtlasAllocator};
use ribir_painter::{AntiAliasing, DeviceSize, Point, ShallowImage, TextureCfg, TextureX};

/// This atlas stores tiled paths of the last frame draw.
pub(crate) struct AlphaAtlas<T: TextureX> {
  atlas: Atlas<AlphaItem, T>,
  canvas: Option<PathCanvas<T>>,
}

impl<T: TextureX> AlphaAtlas<T> {
  pub fn new(
    atlas_size: DeviceSize,
    helper: &mut impl GpuTessellatorHelper<Texture = T>,
    anti_aliasing: AntiAliasing,
  ) -> Self {
    let canvas = (anti_aliasing != AntiAliasing::None).then(|| {
      let texture = helper.new_texture(
        PATH_ATLAS_ID,
        TextureCfg {
          format: ColorFormat::Alpha8,
          size: atlas_size / 2,
          anti_aliasing: AntiAliasing::Msaa4X,
        },
      );
      PathCanvas::new(texture)
    });

    let atlas_allocator = AtlasAllocator::new(atlas_size.to_i32().cast_unit());
    let texture = helper.new_texture(
      PATH_ATLAS_ID,
      TextureCfg {
        format: ColorFormat::Alpha8,
        size: atlas_size,
        anti_aliasing: AntiAliasing::None,
      },
    );
    AlphaAtlas { atlas: Atlas::new(texture), canvas }
  }

  fn store_image(&mut self, img: ShallowImage) -> Option<Allocation> {
    assert_eq!(img.color_format(), ColorFormat::Alpha8);

    todo!();
  }

  fn store_path(&mut self, path: IdPath) {
    todo!(
      "
        1. need to tile path!
        2. check path  if 
      "
    );
  }

  pub fn end_frame(&mut self) {
    if let Some(canvas) = &mut self.canvas {
      canvas.clear();
    }

    todo!()
  }
}

/// A Canvas was used to draw tiled paths with anti-aliasing, let them as close
/// as possible but not overlapping.
///
/// Notice: For a filled path, it will only draw its border.
pub(crate) struct PathCanvas<T: TextureX> {
  texture: T,
  atlas_allocator: AtlasAllocator,
  records: ahash::HashMap<IdPath, Allocation>,
}

impl<T: TextureX> PathCanvas<T> {
  fn new(texture: T) -> Self {
    assert!(texture.format() == ColorFormat::Alpha8);

    Self {
      atlas_allocator: AtlasAllocator::new(texture.size().to_i32().cast_unit()),
      texture,
      records: <_>::default(),
    }
  }

  fn record_path(&mut self, path: IdPath, size: DeviceSize) -> Option<Allocation> {
    debug_assert_eq!(path.path.box_rect().origin, Point::zero());
    let alloc = self.records.get(&path);
    if alloc.is_some() {
      return alloc;
    }
    let alloc = self.atlas_allocator.allocate(size.into())?;
    self.records.insert(path, alloc);
    Some(alloc)
  }

  fn submit(&mut self, helper: &mut impl GpuTessellatorHelper) {
    todo!("generate the path triangles and draw them in texture");
  }

  fn path_iter(&self) -> impl Iterator<Item = (&IdPath, &Allocation)> { self.records.iter() }

  fn clear(&mut self) {
    self.atlas_allocator.clear();
    self.records.clear()
  }
}
