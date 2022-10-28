use crate::prelude::{SizedBox, Stack, Text};
use ribir_core::{impl_query_self_only, prelude::*};
use std::{ops::Range, time::Duration};

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
  #[declare(default)]
  pub text: String,
  #[declare(default = TypographyTheme::of(ctx).body1.text.clone())]
  pub style: TextStyle,
  #[declare(default)]
  pub caret: CaretState,

  #[declare(default, convert=strip_option)]
  pub placeholder: Option<String>,
  #[declare(default, convert=strip_option)]
  pub placeholder_style: Option<TextStyle>,
}

impl Input {
  pub fn text_in_show(&self) -> String {
    if self.text.is_empty() {
      self
        .placeholder
        .as_ref()
        .map_or(String::default(), |s| s.clone())
    } else {
      self.text.clone()
    }
  }
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
      SizedBox {
        id: caret,
        opacity: 1.,
        size: this.rect.size,
        top_anchor: this.rect.min_y(),
        left_anchor: this.rect.min_x(),
        background: this.color,
        mounted: move |_| animate1.run(),
      }
      Animate {
        id: animate1,
        from: State {
          caret.opacity: 1. - caret.opacity,
        },
        transition: Transition {
          duration: Duration::from_secs(1),
          easing: easing::steps(2, easing::StepsJump::JumpNone),
          repeat: f32::INFINITY
        }
      }
    }
  }
}

#[derive(Declare)]
struct SelectedTextBackground {
  is_focused: bool,

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
    let color = match self.is_focused {
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
      track {
        this: this.into_stateful(),
        helper: GlyphHelper::default().into_stateful(),
      }
      Stack {
        id: container,
        char: move |c| this.edit_handle(c.char),
        key_down: move |key| this.key_handle(key),

        pointer_move: move |e| {
          if let CaretState::Selecting(begin, _) = this.caret {
            if e.point_type == PointerType::Mouse
               && e.mouse_buttons() == MouseButtons::PRIMARY {
              let cluster = helper.cluster_from_pos(e.position().x, e.position().y);
              this.caret = CaretState::Selecting(begin, cluster as usize);
            }
          }
        },
        pointer_down: move |e| {
          let cluster = helper.cluster_from_pos(e.position().x, e.position().y);
          this.caret = CaretState::Selecting(cluster as usize, cluster as usize);
        },
        pointer_up: move |_| {
          if let CaretState::Selecting(begin, end) = this.caret {
            this.caret = if begin == end {
                CaretState::Caret(begin as usize)
              }
              else {
              CaretState::Select(begin, end)
            };
          }
        },

        SizedBox {
          size: INFINITY_SIZE,
        }
        SelectedTextBackground {
          is_focused: container.has_focus(),
          rects: helper.select_rects(this.caret.select_range()),
        }
        Text {
          id: text,
          text: this.text.clone() + placeholder,
          style: this.style.clone(),
          performed_layout: move |ctx| {
            helper.glyphs = Some(text.text_layout(&ctx.app_ctx().typography_store, BoxClamp::default()));
          },
        }
        ExprWidget {
          expr: (container.has_focus()).then(|| {
            widget!{
              Caret {
                rect: helper.caret(this.caret.cursor().byte_offset()),
                color: ctx.theme().caret_color,
              }
            }
          })
        }
        ExprWidget {
          expr: (this.text.is_empty() && this.placeholder.is_some()).then(|| {
            widget! {
              Text {
                text: this.placeholder.as_ref().unwrap().clone(),
                style: this.placeholder_style.as_ref().unwrap_or(&this.style).clone()
              }
            }
          })

        }
      }

    }
  }
}
