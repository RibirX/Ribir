use canvas::{Color, FontInfo};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FontStyle {
  pub font_family: Option<String>,
  pub font_color: Option<Color>,
  pub font_size: Option<f32>,
  pub font_weight: Option<f32>,
  pub bold: Option<bool>,
  pub italics: Option<bool>,
}

impl FontStyle {
  pub fn with_family(mut self, family: String) -> FontStyle {
    self.font_family = Some(family);
    self
  }
  pub fn with_color(mut self, color: Color) -> FontStyle {
    self.font_color = Some(color);
    self
  }
  pub fn with_size(mut self, size: f32) -> FontStyle {
    self.font_size = Some(size);
    self
  }
  pub fn with_weight(mut self, weight: f32) -> FontStyle {
    self.font_weight = Some(weight);
    self
  }
  pub fn with_bold(mut self, bold: bool) -> FontStyle {
    self.bold = Some(bold);
    self
  }

  pub fn with_italics(mut self, italics: bool) -> FontStyle {
    self.italics = Some(italics);
    self
  }
}

pub fn default_font() -> FontInfo { FontInfo::default() }

pub fn to_font(font_style: &Option<Arc<FontStyle>>) -> FontInfo {
  let mut font = default_font();
  if font_style.is_some() {
    let style = font_style.as_ref().unwrap();
    if let Some(font_family) = &style.font_family {
      font.family = font_family.to_string();
    }
    if let Some(font_size) = &style.font_size {
      font.font_size = *font_size;
    }
  }
  font
}

pub fn font_color(font_style: &Option<Arc<FontStyle>>) -> Color {
  font_style
    .as_ref()
    .and_then(|style| style.font_color.as_ref())
    .unwrap_or(&Color::BLACK)
    .clone()
}
