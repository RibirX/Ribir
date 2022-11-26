use std::ops::Range;

use ribir_core::{impl_query_self_only, prelude::*};

use crate::prelude::Text;

use super::CaretState;
#[derive(Default)]
pub(crate) struct InputGlyphInfo {
  glyphs: Option<VisualGlyphs>,
}

#[derive(Declare, MultiChild)]
pub(crate) struct InputLayout {
  #[declare(default)]
  pub(crate) cursor_height: f32,
  #[declare(default)]
  pub(crate) cursor_offset: Point,
  #[declare(default)]
  pub(crate) select_rects: Vec<Rect>,

  #[declare(default)]
  layout: InputGlyphInfo,
}

impl InputLayout {
  pub(crate) fn update_layout(
    &mut self,
    text: &CowArc<str>,
    style: &TextStyle,
    caret: CaretState,
    typography_store: &TypographyStore,
  ) {
    self.layout = InputGlyphInfo {
      glyphs: Some(Text::text_layout(
        text,
        style,
        &typography_store,
        BoxClamp::default(),
      )),
    };

    self.select_rects = self.layout.update_selection(caret.select_range());
    (self.cursor_offset, self.cursor_height) = self.layout.update_cursor(caret.offset());
  }

  pub(crate) fn cluster_from_pos(&self, x: f32, y: f32) -> u32 {
    self.layout.cluster_from_pos(x, y)
  }
}

impl Query for InputLayout {
  impl_query_self_only!();
}

impl Render for InputLayout {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let (ctx, children) = ctx.split_children();
    children.fold(Size::zero(), |size, c| {
      let child_size = ctx.perform_child_layout(c, clamp);
      size.max(child_size)
    })
  }

  fn paint(&self, _ctx: &mut PaintingCtx) {}

  fn only_sized_by_parent(&self) -> bool { false }
}

impl InputGlyphInfo {
  fn cluster_from_pos(&self, x: f32, y: f32) -> u32 {
    let glyphs = self.glyphs.as_ref().unwrap();
    let (para, offset) = glyphs.nearest_glyph(x, y);
    return glyphs.position_to_cluster(para, offset);
  }

  fn update_cursor(&mut self, cursor: usize) -> (Point, f32) {
    let glyphs = self.glyphs.as_ref().unwrap();
    let (para, offset) = glyphs.position_by_cluster(cursor as u32);
    let (glphy, line_height) = glyphs.glyph_rect(para, offset);

    (Point::new(glphy.min_x(), glphy.max_y()), line_height)
  }

  fn update_selection(&mut self, (start, end): (usize, usize)) -> Vec<Rect> {
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

#[derive(Declare)]
pub(crate) struct CaretTrigger {
  #[declare(default)]
  pub(crate) caret: CaretState,
}

impl Compose for CaretTrigger {
  fn compose(_: StateWidget<Self>) -> Widget
  where
    Self: Sized,
  {
    Void {}.into_widget()
  }
}
