use std::{cell::RefCell, sync::Arc};

use ahash::HashMap;
use fontdb::{Database, Query};
pub use fontdb::{FaceInfo, Family, ID};
use ribir_algo::{Rc, Resource};
use ribir_geom::{Point, Rect, rect};
use swash::FontRef;

use crate::{
  Path, PixelImage, Svg,
  path_builder::PathBuilder,
  text::{FontFace, FontFamily, GlyphId, svg_glyph_cache::SvgGlyphCache},
};
/// A wrapper of fontdb and cache font data.
pub struct FontDB {
  default_fonts: Vec<ID>,
  data_base: fontdb::Database,
  cache: HashMap<ID, Option<Face>>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum GlyphBaseline {
  /// The glyph baseline is the normal alphabetic baseline, which is the default
  /// value.
  #[default]
  Alphabetic,
  /// This option adjusts the baseline to position the capital letter in the
  /// middle of the em box.
  Middle,
}

type FontGlyphCache<K, V> = Rc<RefCell<HashMap<K, Option<V>>>>;
#[derive(Clone)]
pub struct Face {
  pub face_id: ID,
  pub source_data: Arc<dyn AsRef<[u8]> + Sync + Send>,
  pub face_data_index: u32,
  raster_image_glyphs: FontGlyphCache<(u16, u16), (Resource<PixelImage>, Point)>,
  outline_glyphs: FontGlyphCache<u16, Resource<Path>>,
  svg_glyphs: Rc<RefCell<SvgGlyphCache>>,
  x_height: u16,
  cap_height: i16,
  ascender: i16,
  descender: i16,
}

impl FontDB {
  /// Sets the default callback fonts for the entire application.
  ///
  /// These fonts will be used when the font specified in the text_style for a
  /// text does not match.
  pub fn set_default_fonts(&mut self, face: &FontFace) {
    self.default_fonts = self.select_all_match(face);
  }

  pub fn default_fonts(&self) -> &[ID] { &self.default_fonts }

  pub fn try_get_face_data(&self, face_id: ID) -> Option<&Face> {
    self.cache.get(&face_id)?.as_ref()
  }

  pub fn face_data_or_insert(&mut self, face_id: ID) -> Option<&Face> {
    get_or_insert_face(&mut self.cache, &self.data_base, face_id).as_ref()
  }

  /// Selects a `FaceInfo` by `id`.
  ///
  /// Returns `None` if a face with such ID was already removed,
  /// or this ID belong to the other `Database`.
  #[inline]
  pub fn face_info(&self, id: ID) -> Option<&FaceInfo> { self.data_base.face(id) }

  /// Returns a reference to an internal storage.
  ///
  /// This can be used for manual font matching.
  #[inline]
  pub fn faces_info_iter(&self) -> impl Iterator<Item = &FaceInfo> + '_ { self.data_base.faces() }

  pub fn faces_data_iter(&mut self) -> impl Iterator<Item = Face> + '_ {
    FaceIter {
      face_id_iter: self.data_base.faces(),
      data_base: &self.data_base,
      cache: &mut self.cache,
    }
  }

  #[inline]
  pub fn load_from_bytes(&mut self, data: Vec<u8>) { self.data_base.load_font_data(data); }

  /// Loads a font file into the `Database`.
  ///
  /// Will load all font faces in case of a font collection.
  #[inline]
  pub fn load_font_file<P: AsRef<std::path::Path>>(
    &mut self, path: P,
  ) -> Result<(), std::io::Error> {
    self.data_base.load_font_file(path)
  }

  /// Attempts to load system fonts.
  ///
  /// Supports Windows, Linux and macOS.
  ///
  /// System fonts loading is a surprisingly complicated task,
  /// mostly unsolvable without interacting with system libraries.
  /// And since `fontdb` tries to be small and portable, this method
  /// will simply scan some predefined directories.
  /// Which means that fonts that are not in those directories must
  /// be added manually.
  pub fn load_system_fonts(&mut self) {
    self.data_base.load_system_fonts();
    self.static_generic_families();
  }

  /// Performs a CSS-like query and returns the best matched font face id.
  pub fn select_best_match(&self, face: &FontFace) -> Option<ID> {
    let FontFace { families, stretch, style, weight } = face;
    let families = families
      .iter()
      .map(to_db_family)
      .collect::<Vec<_>>();
    self.data_base.query(&Query {
      families: &families,
      weight: *weight,
      stretch: *stretch,
      style: *style,
    })
  }

  /// Performs a CSS-like query and returns the all matched font face ids
  pub fn select_all_match(&mut self, face: &FontFace) -> Vec<ID> {
    let FontFace { families, stretch, style, weight } = face;
    families
      .iter()
      .filter_map(|f| {
        self.data_base.query(&Query {
          families: &[to_db_family(f)],
          weight: *weight,
          stretch: *stretch,
          style: *style,
        })
      })
      .collect()
  }

  fn static_generic_families(&mut self) {
    // We don't like to depends on some system library and not make the fallback
    // font too complicated. So here are some default fonts collect from web.
    let init_data: [(&[Family], _); 5] = [
      (
        &[
          #[cfg(any(target_os = "windows", target_os = "linux", target_os = "ios"))]
          Family::Name("Times New Roman"),
          #[cfg(target_os = "macos")]
          Family::Name("Times"),
          #[cfg(target_os = "linux")]
          Family::Name("Free Serif"),
          #[cfg(any(target_os = "linux", target_os = "android"))]
          Family::Name("Noto Serif"),
        ],
        Database::set_serif_family as fn(&mut Database, String),
      ),
      (
        &[
          #[cfg(target_os = "windows")]
          Family::Name("Segoe UI"),
          #[cfg(target_os = "windows")]
          Family::Name("Tahoma"),
          #[cfg(any(target_os = "macos", target_os = "ios"))]
          Family::Name("San Francisco"),
          #[cfg(any(target_os = "macos", target_os = "ios"))]
          Family::Name("Helvetica"),
          #[cfg(any(target_os = "macos", target_os = "ios"))]
          Family::Name("Helvetica Neue"),
          #[cfg(target_os = "android")]
          Family::Name("Roboto"),
          #[cfg(target_os = "android")]
          Family::Name("Droid Sans"),
          #[cfg(target_os = "linux")]
          Family::Name("Ubuntu"),
          #[cfg(target_os = "linux")]
          Family::Name("Red Hat"),
          #[cfg(target_os = "linux")]
          Family::Name("DejaVu Sans"),
          #[cfg(target_os = "linux")]
          Family::Name("Noto Sans"),
          #[cfg(target_os = "linux")]
          Family::Name("Liberation Sans"),
        ],
        Database::set_sans_serif_family as fn(&mut Database, String),
      ),
      (
        &[
          #[cfg(target_os = "macos")]
          Family::Name("Apple Chancery"),
          #[cfg(target_os = "ios")]
          Family::Name("Snell Roundhand"),
          #[cfg(target_os = "windows")]
          Family::Name("Comic Sans MS"),
          #[cfg(target_os = "android")]
          Family::Name("Dancing Script"),
          #[cfg(target_os = "linux")]
          Family::Name("DejaVu Serif"),
          #[cfg(target_os = "linux")]
          Family::Name("Noto Serif"),
        ],
        Database::set_cursive_family as fn(&mut Database, String),
      ),
      (
        &[
          #[cfg(any(target_os = "macos", target_os = "ios"))]
          Family::Name("Papyrus"),
          #[cfg(target_os = "windows")]
          Family::Name("Microsoft Sans Serif"),
          #[cfg(target_os = "linux")]
          Family::Name("Free Serif"),
          #[cfg(target_os = "linux")]
          Family::Name("DejaVu Serif"),
          #[cfg(any(target_os = "linux", target_os = "android"))]
          Family::Name("Noto Serif"),
        ],
        Database::set_fantasy_family as fn(&mut Database, String),
      ),
      (
        &[
          #[cfg(target_os = "macos")]
          Family::Name("Andale Mono"),
          #[cfg(target_os = "ios")]
          Family::Name("Courier"),
          #[cfg(target_os = "windows")]
          Family::Name("Courier New"),
          #[cfg(target_os = "android")]
          Family::Name("Droid Sans Mono"),
          #[cfg(target_os = "linux")]
          Family::Name("DejaVu Sans Mono"),
          #[cfg(target_os = "linux")]
          Family::Name("Noto Sans Mono"),
        ],
        Database::set_monospace_family as fn(&mut Database, String),
      ),
    ];

    init_data.iter().for_each(|(families, set_fn)| {
      let name = families
        .iter()
        .filter(|f| {
          self
            .data_base
            .query(&Query { families: &[**f], ..<_>::default() })
            .is_some()
        })
        .map(|f| self.data_base.family_name(f).to_string())
        .next();

      if let Some(name) = name {
        set_fn(&mut self.data_base, name);
      }
    });
  }
}

impl Default for FontDB {
  fn default() -> FontDB {
    let mut data_base = fontdb::Database::new();
    data_base.load_font_data(include_bytes!("./Lato-Regular.ttf").to_vec());
    let default_font = data_base.faces().next().map(|f| f.id).unwrap();
    let mut this = FontDB { default_fonts: vec![default_font], data_base, cache: <_>::default() };
    this.face_data_or_insert(default_font);
    this
  }
}

impl Face {
  pub fn from_data(
    face_id: ID, source_data: Arc<dyn AsRef<[u8]> + Sync + Send>, face_index: u32,
  ) -> Option<Self> {
    let ptr_data = source_data.as_ref().as_ref() as *const [u8];
    // Safety: we know the ptr_data has some valid lifetime with source data, and
    // hold them in same struct.
    let slice = unsafe { &*ptr_data };
    let swash_font = FontRef::from_index(slice, face_index as usize)?;
    let metrics = swash_font.metrics(&[]);

    let ascender = metrics.ascent as i16;
    let descender = (-metrics.descent) as i16;
    let cap_height = metrics.cap_height as i16;
    let x_height = metrics.x_height as u16;

    Some(Face {
      source_data,
      face_data_index: face_index,
      face_id,
      outline_glyphs: <_>::default(),
      raster_image_glyphs: <_>::default(),
      svg_glyphs: <_>::default(),
      x_height,
      ascender,
      descender,
      cap_height,
    })
  }

  pub fn ascender(&self) -> i16 { self.ascender }

  pub fn descender(&self) -> i16 { self.descender }

  pub fn x_height(&self) -> u16 { self.x_height }

  pub fn vertical_height(&self) -> Option<i16> { Some(self.ascender - self.descender) }

  pub fn baseline_offset(&self, baseline: GlyphBaseline) -> i16 {
    match baseline {
      GlyphBaseline::Alphabetic => 0,
      GlyphBaseline::Middle => (self.units_per_em() as i16 - self.cap_height) / 2,
    }
  }

  #[inline]
  pub fn has_char(&self, c: char) -> bool {
    let swash_font = self.as_font_ref();
    swash_font.charmap().map(c) != 0
  }

  pub fn as_font_ref(&self) -> FontRef<'_> {
    let ptr_data = self.source_data.as_ref().as_ref() as *const [u8];
    let slice = unsafe { &*ptr_data };
    FontRef::from_index(slice, self.face_data_index as usize).unwrap()
  }

  pub fn outline_glyph(&self, glyph_id: u16) -> Option<Resource<Path>> {
    self
      .outline_glyphs
      .borrow_mut()
      .entry(glyph_id)
      .or_insert_with(|| {
        let swash_font = self.as_font_ref();
        let mut scaler = swash::scale::ScaleContext::new();
        let mut builder = GlyphOutlineBuilder::default();
        if let Some(outline) = scaler
          .builder(swash_font)
          .size(self.units_per_em() as f32)
          .build()
          .scale_outline(glyph_id)
        {
          let mut points = outline.points().iter();
          for verb in outline.verbs() {
            match verb {
              swash::zeno::Verb::MoveTo => {
                let p = points.next().unwrap();
                builder.move_to(p.x, p.y);
              }
              swash::zeno::Verb::LineTo => {
                let p = points.next().unwrap();
                builder.line_to(p.x, p.y);
              }
              swash::zeno::Verb::QuadTo => {
                let p1 = points.next().unwrap();
                let p = points.next().unwrap();
                builder.quad_to(p1.x, p1.y, p.x, p.y);
              }
              swash::zeno::Verb::CurveTo => {
                let p1 = points.next().unwrap();
                let p2 = points.next().unwrap();
                let p = points.next().unwrap();
                builder.curve_to(p1.x, p1.y, p2.x, p2.y, p.x, p.y);
              }
              swash::zeno::Verb::Close => {
                builder.close();
              }
            }
          }
          let path = builder.build(rect(0., 0., 0., 0.));
          Some(Resource::new(path))
        } else {
          None
        }
      })
      .as_ref()
      .cloned()
  }

  pub fn glyph_raster_image(
    &self, glyph_id: u16, img_size: u16,
  ) -> Option<(Resource<PixelImage>, ribir_geom::Point)> {
    self
      .raster_image_glyphs
      .borrow_mut()
      .entry((glyph_id, img_size))
      .or_insert_with(|| {
        let swash_font = self.as_font_ref();
        let mut scaler = swash::scale::ScaleContext::new();
        let mut scaler = scaler
          .builder(swash_font)
          .size(img_size as f32)
          .hint(true)
          .build();
        let image = swash::scale::Render::new(&[
          swash::scale::Source::ColorBitmap(swash::scale::StrikeWith::BestFit),
          swash::scale::Source::Bitmap(swash::scale::StrikeWith::BestFit),
          swash::scale::Source::Outline,
        ])
        .render(&mut scaler, glyph_id);

        image.map(|img| {
          let format = match img.content {
            swash::scale::image::Content::Color => crate::ColorFormat::Rgba8,
            _ => crate::ColorFormat::Alpha8,
          };
          let pixel_img = PixelImage::new(
            std::borrow::Cow::Owned(img.data),
            img.placement.width,
            img.placement.height,
            format,
          );
          (
            Resource::new(pixel_img),
            ribir_geom::Point::new(img.placement.left as f32, -img.placement.top as f32),
          )
        })
      })
      .clone()
  }

  // Used to map a GlyphId back to its primary char.

  pub fn glyph_svg_image(&self, glyph_id: u16) -> Option<Svg> {
    self
      .svg_glyphs
      .borrow_mut()
      .svg_or_insert(GlyphId(glyph_id), &self.as_font_ref(), self.face_data_index)
      .clone()
  }

  #[inline]
  pub fn units_per_em(&self) -> u16 { self.as_font_ref().metrics(&[]).units_per_em }
}

fn to_db_family(f: &FontFamily) -> Family<'_> {
  match f {
    FontFamily::Name(name) => Family::Name(name),
    FontFamily::Serif => Family::Serif,
    FontFamily::SansSerif => Family::SansSerif,
    FontFamily::Cursive => Family::Cursive,
    FontFamily::Fantasy => Family::Fantasy,
    FontFamily::Monospace => Family::Monospace,
  }
}

#[derive(Default)]
struct GlyphOutlineBuilder {
  builder: PathBuilder,
  closed: bool,
}

impl GlyphOutlineBuilder {
  fn build(mut self, bounds: Rect) -> Path {
    if !self.closed {
      self.builder.end_path(false);
    }
    self.builder.build_with_bounds(bounds)
  }
}

impl GlyphOutlineBuilder {
  fn move_to(&mut self, x: f32, y: f32) {
    self.closed = false;
    self.builder.begin_path(Point::new(x, y));
  }

  fn line_to(&mut self, x: f32, y: f32) {
    self.closed = false;
    self.builder.line_to(Point::new(x, y));
  }

  fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
    self.closed = false;
    self
      .builder
      .quadratic_curve_to(Point::new(x1, y1), Point::new(x, y));
  }

  fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
    self.closed = false;
    self
      .builder
      .bezier_curve_to(Point::new(x1, y1), Point::new(x2, y2), Point::new(x, y));
  }

  fn close(&mut self) {
    if !self.closed {
      self.closed = true;
      self.builder.end_path(true)
    }
  }
}

impl std::ops::Deref for Face {
  type Target = ID;

  #[inline]
  fn deref(&self) -> &Self::Target { &self.face_id }
}

pub struct FaceIter<'a, T>
where
  T: Iterator<Item = &'a FaceInfo>,
{
  face_id_iter: T,
  data_base: &'a Database,
  cache: &'a mut HashMap<ID, Option<Face>>,
}

impl<'a, T> Iterator for FaceIter<'a, T>
where
  T: Iterator<Item = &'a FaceInfo>,
{
  type Item = Face;
  fn next(&mut self) -> Option<Self::Item> {
    loop {
      let info = self.face_id_iter.next()?;
      let face = get_or_insert_face(self.cache, self.data_base, info.id)
        .as_ref()
        .cloned();
      if face.is_some() {
        return face;
      }
    }
  }
}

fn get_or_insert_face<'a>(
  cache: &'a mut HashMap<ID, Option<Face>>, data_base: &'a Database, id: ID,
) -> &'a Option<Face> {
  cache.entry(id).or_insert_with(|| {
    data_base
      .face_source(id)
      .and_then(|(src, face_index)| {
        let source_data = match src {
          fontdb::Source::Binary(data) => Some(data),
          fontdb::Source::File(_) => {
            let mut source_data = None;
            data_base.with_face_data(id, |data, index| {
              assert_eq!(face_index, index);
              let data: Arc<dyn AsRef<[u8]> + Sync + Send> = Arc::new(data.to_owned());
              source_data = Some(data);
            });
            source_data
          }
          fontdb::Source::SharedFile(_, data) => Some(data),
        }?;
        Face::from_data(id, source_data, face_index)
      })
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::FontWeight;

  #[test]
  fn load_font_from_path() {
    let mut db = FontDB::default();
    let path = env!("CARGO_MANIFEST_DIR").to_owned() + "/../fonts/DejaVuSans.ttf";
    db.load_font_file(path).unwrap();
    let face_id = db.select_best_match(&FontFace {
      families: vec![FontFamily::Name("DejaVu Sans".into())].into_boxed_slice(),
      ..<_>::default()
    });
    assert!(face_id.is_some());

    let info = db.face_info(face_id.unwrap()).unwrap();

    assert_eq!(info.families.len(), 1);
    assert_eq!(info.families[0].0, "DejaVu Sans");
  }

  #[test]
  fn load_font_from_bytes() {
    let mut db = FontDB::default();
    let bytes = include_bytes!("../../../fonts/GaramondNo8-Reg.ttf");
    db.load_from_bytes(bytes.to_vec());

    let face_id = db.select_best_match(&FontFace {
      families: vec![FontFamily::Name("GaramondNo8".into())].into_boxed_slice(),
      ..<_>::default()
    });
    assert!(face_id.is_some());
  }

  #[test]
  fn load_sys_fonts() {
    let mut db = FontDB::default();
    db.load_system_fonts();
    assert!(db.faces_info_iter().next().is_some())
  }

  #[test]
  fn match_font() {
    let mut fonts = FontDB::default();
    fonts.load_system_fonts();
    let path = env!("CARGO_MANIFEST_DIR").to_owned() + "/../fonts/DejaVuSans.ttf";
    let _ = fonts.load_font_file(path);

    let mut face = FontFace {
      families: vec![FontFamily::Name("DejaVu Sans".into()), FontFamily::SansSerif]
        .into_boxed_slice(),
      ..<_>::default()
    };

    // match custom load fonts.
    let id = fonts.select_best_match(&face).unwrap();
    let info = fonts.face_info(id).unwrap();
    assert_eq!(info.families.len(), 1);
    assert_eq!(info.families[0].0, "DejaVu Sans");
    fonts.data_base.remove_face(id);

    face.weight = FontWeight::BOLD;

    let id = fonts.select_best_match(&face);
    assert!(id.is_some());
    let info = fonts.face_info(id.unwrap()).unwrap();
    assert_eq!(info.weight, FontWeight::BOLD);
  }
}
