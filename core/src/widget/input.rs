use std::{ops::Range, time::Duration};

use crate::{impl_query_self_only, prelude::*};
use ::text::{CharacterCursor, ControlChar, GraphemeCursor, TextWriter, VisualGlyphs};
use painter::TextStyle;

#[derive(Debug)]
pub enum CaretState {
  Caret(usize),
  Select(usize, usize),
  Selecting(usize, usize),
}

impl From<usize> for CaretState {
  fn from(c: usize) -> Self { CaretState::Caret(c) }
}

impl From<(usize, usize)> for CaretState {
  fn from((begin, end): (usize, usize)) -> Self { CaretState::Select(begin, end) }
}

impl Default for CaretState {
  fn default() -> Self { 0.into() }
}

impl CaretState {
  fn select_range(&self) -> (usize, usize) {
    match *self {
      CaretState::Caret(cursor) => (cursor, cursor),
      CaretState::Select(begin, end) => (begin.min(end), begin.max(end)),
      CaretState::Selecting(begin, end) => (begin.min(end), begin.max(end)),
    }
  }

  fn cursor(&self) -> GraphemeCursor {
    match *self {
      CaretState::Caret(cursor) => GraphemeCursor(cursor),
      CaretState::Select(_, end) => GraphemeCursor(end),
      CaretState::Selecting(_, end) => GraphemeCursor(end),
    }
  }
}

struct GlyphHelper {
  glyphs: Option<VisualGlyphs>,
}

impl Default for GlyphHelper {
  fn default() -> Self { Self { glyphs: None } }
}

impl GlyphHelper {
  fn caret(&self, cursor: usize) -> Rect {
    if let Some(glyphs) = &self.glyphs {
      let (para, offset) = glyphs.position_by_cluster(cursor as u32);
      let (glphy, line_height) = glyphs.glyph_rect(para, offset);
      return Rect::new(
        Point::new(glphy.min_x(), glphy.max_y()),
        Size::new(1., line_height),
      );
    } else {
      Rect::zero()
    }
  }

  fn cluster_from_pos(&self, x: f32, y: f32) -> u32 {
    if let Some(glyphs) = &self.glyphs {
      let (para, offset) = glyphs.nearest_glyph(x, y);
      return glyphs.position_to_cluster(para, offset);
    } else {
      return 0;
    }
  }

  fn select_rects(&self, (start, end): (usize, usize)) -> Vec<Rect> {
    if let Some(glyphs) = &self.glyphs {
      glyphs
        .select_range(Range { start, end })
        .iter()
        .map(|r| {
          Rect::new(
            Point::new(r.min_x().value(), r.min_y().value()),
            Size::new(r.width().value(), r.height().value()),
          )
        })
        .collect()
    } else {
      vec![]
    }
  }
}

#[derive(Declare)]
pub struct Input {
  pub text: String,
  #[declare(default = TypographyTheme::of(ctx).body1.text.clone())]
  pub style: TextStyle,
  #[declare(default)]
  pub caret: CaretState,
}

impl Input {
  fn edit_handle(&mut self, c: char) {
    let has_deleted = self.del_selected();

    let mut cursor = self.caret.cursor();
    let mut writer = TextWriter::new(&mut self.text, &mut cursor);
    match c {
      ControlChar::DEL => {
        if !has_deleted {
          writer.del_char()
        }
      }
      ControlChar::BACKSPACE => {
        if !has_deleted {
          writer.back_space()
        }
      }
      _ => writer.insert_char(c),
    };
    self.caret = cursor.byte_offset().into();
  }

  fn key_handle(&mut self, key: &mut KeyboardEvent) {
    let mut cursor = self.caret.cursor();
    let mut writer = TextWriter::new(&mut self.text, &mut cursor);
    match key.key {
      VirtualKeyCode::Left => {
        writer.move_to_prev();
        self.caret = cursor.byte_offset().into();
      }
      VirtualKeyCode::Right => {
        writer.move_to_next();
        self.caret = cursor.byte_offset().into();
      }
      _ => (),
    };
  }

  fn del_selected(&mut self) -> bool {
    let (begin, end) = self.caret.select_range();
    if begin == end {
      return false;
    }
    self.text.drain(begin..end);
    self.caret = begin.into();
    return true;
  }
}

#[derive(Declare)]
struct Caret {
  rect: Rect,
  color: Color,
}

impl Query for Caret {
  impl_query_self_only!();
}

impl Compose for Caret {
  fn compose(this: StateWidget<Self>) -> Widget {
    widget! {
      track {this: this.into_stateful(),}
      CaretRender {
        id: caret,
        visible: true,
        rect: this.rect,
        color: this.color,
        on_mounted: move |_| animate1.clone_stateful().run(),
      }
      animations {
        caret.visible: Animate {
          id: animate1,
          from: State {
            caret.visible: !caret.visible,
          },
          transition: Transition {
            duration: Duration::from_secs(1),
            easing: easing::steps(2, easing::StepsJump::JumpNone),
          }.repeat(Repeat::Infinite),
        }
      }
    }
  }
}

// todo: should be replace by sizedbox or rectangle widget after visible has
// support
#[derive(Declare)]
struct CaretRender {
  visible: bool,
  rect: Rect,
  color: Color,
}

impl Query for CaretRender {
  impl_query_self_only!();
}

impl Render for CaretRender {
  fn perform_layout(&self, _: BoxClamp, _ctx: &mut LayoutCtx) -> Size { Size::zero() }

  fn paint(&self, ctx: &mut PaintingCtx) {
    if !self.visible {
      return;
    }
    let painter = ctx.painter();
    painter.rect(&self.rect);
    painter.set_brush(Brush::Color(self.color));
    painter.fill();
  }

  fn only_sized_by_parent(&self) -> bool { false }
}

#[derive(Declare)]
struct SelectedTextBackground {
  focus: bool,
  
  #[declare(default = TextSelectedBackground::of(ctx).focus.clone())]
  focus_color: Color,
  #[declare(default = TextSelectedBackground::of(ctx).blur.clone())]
  blur_color: Color,
  rects: Vec<Rect>,
}

impl Query for SelectedTextBackground {
  impl_query_self_only!();
}

impl Render for SelectedTextBackground {
  fn perform_layout(&self, _: BoxClamp, _ctx: &mut LayoutCtx) -> Size { Size::zero() }

  fn paint(&self, ctx: &mut PaintingCtx) {
    let color = match self.focus {
      true => self.focus_color,
      false => self.blur_color,
    };

    let painter = ctx.painter();
    painter.set_brush(Brush::Color(color));
    self.rects.iter().for_each(|rc| {
      painter.rect(&rc);
      painter.fill();
    })
  }

  fn only_sized_by_parent(&self) -> bool { false }
}

impl Compose for Input {
  fn compose(this: StateWidget<Self>) -> Widget {
    let placeholder = "\r";
    widget! {
      track { this: this.into_stateful(), helper: GlyphHelper::default().into_stateful(), focus: false.into_stateful()  }
      Stack {
        on_char: move |c| this.edit_handle(c.char),
        on_key_down: move |key| this.key_handle(key),
        on_focus: move |_| *focus = true,
        on_blur: move |_| *focus = false,

        on_pointer_move: move |e| {
          if let CaretState::Selecting(begin, _) = this.caret {
            if e.point_type == PointerType::Mouse
               && e.mouse_buttons() == MouseButtons::PRIMARY {
              let cluster = helper.cluster_from_pos(e.position().x, e.position().y);
              this.caret = CaretState::Selecting(begin, cluster as usize);
            }
          }
        },
        on_pointer_down: move |e| {
          let cluster = helper.cluster_from_pos(e.position().x, e.position().y);
          this.caret = CaretState::Selecting(cluster as usize, cluster as usize);
        },
        on_pointer_up: move |_e| {
          if let CaretState::Selecting(begin, end) = this.caret {
            this.caret = if begin == end {
                CaretState::Caret(begin as usize)              }
              else {
              CaretState::Select(begin, end)
            };
          }
        },

        SizedBox {
          size: INFINITY_SIZE,
        }
        SelectedTextBackground {
          focus: *focus,
          rects: helper.select_rects(this.caret.select_range()),
        }
        Text {
          id: text,
          text: this.text.clone() + placeholder,
          style: this.style.clone(),
          on_performed_layout: move |ctx| {
            let app_ctx = ctx.widget_tree().app_ctx().borrow();
            helper.glyphs = Some(text.text_layout(&app_ctx.typography_store, BoxClamp::default()));
          },
        }
        ExprWidget {
          expr: (*focus).then(|| {
            Caret {
              rect: helper.caret(this.caret.cursor().byte_offset()),
              color: ctx.theme().caret_color,
            }
          })
        }  
      }

    }
  }
}
