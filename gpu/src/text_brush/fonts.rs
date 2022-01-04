// todo: deprecated
use super::{FontId, Vertex};
pub use font_kit::properties::{
  Properties as FontProperties, Stretch as FontStretch, Style as FontStyle, Weight as FontWeight,
};
use font_kit::{
  family_name::FamilyName, font::Font as FK_Font, loader::Loader, source::SystemSource,
};
use glyph_brush::{ab_glyph::FontArc, GlyphBrush};
use std::collections::HashMap;
use std::sync::Arc;

pub struct Font {
  pub font: FK_Font,
  pub id: FontId,
}

/// Manage loaded font.
pub(crate) struct Fonts {
  load_fonts: HashMap<FontKey, Font>,
  source: SystemSource,
}

impl Fonts {
  pub fn new() -> Self {
    Self {
      load_fonts: <_>::default(),
      source: SystemSource::new(),
    }
  }

  /// If the data represents a collection (`.ttc`/`.otc`/etc.), `font_index`
  /// specifies the index of the font to load from it. If the data represents
  /// a single font, pass 0 for `font_index`.
  pub fn load_from_bytes(
    &mut self,
    font_data: Vec<u8>,
    font_index: u32,
    brush: &mut GlyphBrush<[Vertex; 4], u32>,
  ) -> Result<&Font, Box<dyn std::error::Error>> {
    let font = FK_Font::from_bytes(Arc::new(font_data), font_index)?;
    self.try_insert_font(font, brush)
  }

  /// Loads a font from the path to a `.ttf`/`.otf`/etc. file.
  ///
  /// If the file is a collection (`.ttc`/`.otc`/etc.), `font_index` specifies
  /// the index of the font to load from it. If the file represents a single
  /// font, pass 0 for `font_index`.
  pub fn load_from_path<P: AsRef<std::path::Path>>(
    &mut self,
    path: P,
    font_index: u32,
    brush: &mut GlyphBrush<[Vertex; 4], u32>,
  ) -> Result<&Font, Box<dyn std::error::Error>> {
    let font = <FK_Font as Loader>::from_path(path, font_index)?;
    self.try_insert_font(font, brush)
  }

  /// Performs font matching according to the CSS Fonts Level 3 specification
  /// and returns matched fonts.
  pub fn select_best_match(
    &mut self,
    family_names: &str,
    props: &FontProperties,
    brush: &mut GlyphBrush<[Vertex; 4], u32>,
  ) -> Result<&Font, Box<dyn std::error::Error>> {
    for family in family_names.split(',') {
      let family = family.replace('\'', "");
      let family = family.trim();

      let key = FontKey {
        family: family.to_string(),
        props: *props,
      };
      if self.load_fonts.contains_key(&key) {
        return self.load_fonts.get(&key).ok_or_else(|| unreachable!());
      } else {
        let font = self
          .source
          .select_best_match(&[family_name(family)], props)
          .ok()
          .and_then(|handle| handle.load().ok());
        if let Some(font) = font {
          return self.try_insert_font(font, brush);
        }
      }
    }

    Err("No match font".into())
  }

  fn try_insert_font(
    &mut self,
    font: FK_Font,
    brush: &mut GlyphBrush<[Vertex; 4], u32>,
  ) -> Result<&Font, Box<dyn std::error::Error>> {
    let key = FontKey::from_fk_font(&font);
    if self.load_fonts.contains_key(&key) {
      // todo: we should replace old font
      self.load_fonts.get(&key).ok_or_else(|| unreachable!())
    } else {
      let data = font.copy_font_data().ok_or("font not available")?;
      // unsafe introduce:
      // Text brush logic keep font's data long live than brush.
      let font_bytes: &'static [u8] = unsafe { std::mem::transmute(data.as_slice()) };
      let brush_font = FontArc::try_from_slice(font_bytes)?;
      let id = brush.add_font(brush_font);

      let font = self.load_fonts.entry(key).or_insert(Font { font, id });
      Ok(font)
    }
  }
}

fn family_name(name: &str) -> FamilyName {
  match name {
    "serif" => FamilyName::Serif,
    "sans-serif" => FamilyName::SansSerif,
    "monospace" => FamilyName::Monospace,
    "cursive" => FamilyName::Cursive,
    "fantasy" => FamilyName::Fantasy,
    _ => FamilyName::Title(name.to_string()),
  }
}

#[derive(Debug)]
struct FontKey {
  family: String,
  props: FontProperties,
}

impl std::hash::Hash for FontKey {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    state.write(self.family.as_bytes());
    state.write_u8(self.props.style as u8);
    state.write_u32(self.props.weight.0.to_bits());
    state.write_u32(self.props.stretch.0.to_bits());
  }
}

impl PartialEq for FontKey {
  #[inline]
  fn eq(&self, other: &Self) -> bool { self.family == other.family && self.props == other.props }
}
impl Eq for FontKey {}

impl FontKey {
  #[inline]
  fn from_fk_font(font: &FK_Font) -> Self {
    FontKey {
      family: font.family_name(),
      props: font.properties(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use glyph_brush::GlyphBrushBuilder;

  #[test]
  fn load_font_from_path() {
    let mut brush = GlyphBrushBuilder::using_fonts(vec![]).build();
    let mut fonts = Fonts::new();
    let path = env!("CARGO_MANIFEST_DIR").to_owned() + "/fonts/DejaVuSans.ttf";
    let font = fonts.load_from_path(path, 0, &mut brush);
    assert_eq!(font.unwrap().font.family_name(), "DejaVu Sans");
  }

  #[test]
  fn load_font_from_bytes() {
    let mut brush = GlyphBrushBuilder::using_fonts(vec![]).build();
    let mut fonts = Fonts::new();
    let bytes = include_bytes!("../../fonts/GaramondNo8-Reg.ttf");
    let font = fonts.load_from_bytes(bytes.to_vec(), 0, &mut brush);
    assert_eq!(font.unwrap().font.family_name(), "GaramondNo8");
  }

  #[test]
  fn match_font() {
    let mut brush = GlyphBrushBuilder::using_fonts(vec![]).build();
    let mut fonts = Fonts::new();
    let path = env!("CARGO_MANIFEST_DIR").to_owned() + "/fonts/DejaVuSans.ttf";
    let _ = fonts.load_from_path(path, 0, &mut brush);
    let mut props = FontProperties::new();

    {
      let font = fonts.select_best_match("DejaVu Sans, Arial", &props, &mut brush);
      // match custom load fonts.
      assert_eq!(font.unwrap().font.family_name(), "DejaVu Sans");
    }

    props.weight = FontWeight::BOLD;
    let font;
    // match default fonts
    #[cfg(target_os = "linux")]
    {
      font = fonts
        .select_best_match("DejaVu Serif, Noto Serif, DejaVu Sans", &props, &mut brush)
        .unwrap();

      assert!(font.font.family_name().contains("Serif"));
    }

    #[cfg(target_os = "macos")]
    {
      font = fonts
        .select_best_match("Arial, DejaVu Sans", &props, &mut brush)
        .unwrap();

      assert_eq!(font.font.family_name(), "Arial");
    }

    assert_eq!(font.font.properties().weight, FontWeight::BOLD);
  }
}
