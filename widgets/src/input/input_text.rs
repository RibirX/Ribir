use std::{cell::RefCell, ops::Range, rc::Rc};

use ribir_core::prelude::*;

use crate::prelude::Text;

#[derive(Declare)]
pub(crate) struct InputText {
  #[declare(convert=into)]
  pub(crate) text: CowArc<str>,
  pub(crate) style: TextStyle,

  #[declare(default)]
  pub(crate) glyphs_helper: Rc<RefCell<GlyphsHelper>>,
}

#[derive(Default)]
pub(crate) struct GlyphsHelper {
  glyphs: Option<VisualGlyphs>,
}

impl Compose for InputText {
  fn compose(this: StateWidget<Self>) -> Widget {
    let placeholder = "\r";
    widget! {
        states {this: this.into_stateful()}
        Text {
          id: text,
          text: this.text.to_string() + placeholder,
          style: this.style.clone(),

          performed_layout: move |ctx| {
            let bound = ctx.layout_info().expect("layout info must exit in performed_layout").clamp;
            *this.glyphs_helper.borrow_mut() = GlyphsHelper {
              glyphs: Some(Text::text_layout(
              &text.text,
              &text.style,
              ctx.wnd_ctx().typography_store(),
              bound,
            ))};
          }
      }
    }
  }
}

impl GlyphsHelper {
  pub(crate) fn cluster_from_pos(&self, x: f32, y: f32) -> u32 {
    let glyphs = self.glyphs.as_ref().unwrap();
    let (para, offset) = glyphs.nearest_glyph(x, y);
    return glyphs.position_to_cluster(para, offset);
  }

  pub(crate) fn cursor(&self, cursor: usize) -> (Point, f32) {
    let glyphs = self.glyphs.as_ref().unwrap();
    let (para, offset) = glyphs.position_by_cluster(cursor as u32);
    let (glphy, line_height) = glyphs.glyph_rect(para, offset);

    (Point::new(glphy.min_x(), glphy.max_y()), line_height)
  }

  pub(crate) fn selection(&self, (start, end): (usize, usize)) -> Vec<Rect> {
    if start == end {
      return vec![];
    }
    self
      .glyphs
      .as_ref()
      .unwrap()
      .select_range(Range { start, end })
      .iter()
      .map(|r| {
        Rect::new(
          Point::new(r.min_x().value(), r.min_y().value()),
          Size::new(r.width().value(), r.height().value()),
        )
      })
      .collect()
  }
}
