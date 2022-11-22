use crate::{
  layout::{Container, ExpandDir},
  prelude::{ExpandBox, Stack, Text},
};
use ribir_core::{impl_query_self_only, prelude::*};
use std::{cell::RefCell, ops::Range, rc::Rc, time::Duration};

#[derive(Clone, Copy, Debug)]
pub enum CaretState {
  Caret(usize),
  Select(usize, usize),
  Selecting(usize, usize),
}

#[derive(Declare)]
pub struct Input {
  #[declare(default)]
  pub text: String,
  #[declare(default = TypographyTheme::of(ctx).body1.text.clone())]
  pub style: TextStyle,

  #[declare(default, convert=strip_option)]
  pub placeholder: Option<String>,
  #[declare(default, convert=strip_option)]
  pub placeholder_style: Option<TextStyle>,

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
pub struct CaretStyle {
  pub font: TextStyle,
}

#[derive(Declare)]
struct CaretTrigger {
  #[declare(default)]
  caret: CaretState,
}

impl Compose for CaretTrigger {
  fn compose(_: StateWidget<Self>) -> Widget
  where
    Self: Sized,
  {
    Void {}.into_widget()
  }
}

impl ComposeStyle for CaretStyle {
  type Host = Widget;
  fn compose_style(this: Stateful<Self>, host: Self::Host) -> Widget
    where
      Self: Sized {
    widget! {
      states {this: this.into_stateful(),}
      init ctx => {
        let caret_transition = Transition::declare_builder()
        .duration(Duration::from_secs(1))
        .easing(easing::steps(2, easing::StepsJump::JumpNone))
        .repeat(f32::INFINITY)
        .build(ctx);
      }
      
      DynWidget {
        id: caret,
        opacity: 1.,
        background: this.font.foreground.clone(),
        mounted: move |_| animate1.run(),

        dyns: host,
      }
      Animate {
        id: animate1,
        transition: caret_transition,
        prop: prop!(caret.opacity),
        from: 0.,
      }
    }
  }
}

#[derive(Declare)]
pub struct SelectedTextStyle {
}

impl ComposeStyle for SelectedTextStyle {
  type Host = Widget;
  fn compose_style(_: Stateful<Self>, host: Self::Host) -> Widget
    where
      Self: Sized {
    widget! {
      DynWidget {
        background: Color::from_rgb(181, 215, 254), // todo: follow application active state
        dyns: host,
      }
    }
  }
}

#[derive(Default)]
struct InputGlyphInfo {
  glyphs: Option<VisualGlyphs>,
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

#[derive(Declare, MultiChild)]
struct InputLayout {
  #[declare(default)]
  cursor_height: f32,
  #[declare(default)]
  cursor_offset: Point,
  #[declare(default)]
  select_rects: Vec<Rect>,

  #[declare(default)]
  layout: InputGlyphInfo,
}

impl InputLayout {
  pub fn update_layout(
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

  pub fn cluster_from_pos(&self, x: f32, y: f32) -> u32 { self.layout.cluster_from_pos(x, y) }
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

fn auto_x_scroll_pos(container: &ScrollableWidget, before: f32, after: f32) -> f32 {
  let view_size = container.scroll_view_size();
  let content_size = container.scroll_content_size();
  let current = container.scroll_pos.x;
  let view_after = current + after;
  let view_before = current + before;
  let inner_view = |pos| (0. <= pos && pos < view_size.width);
  if content_size.width <= view_size.width || inner_view(view_after) {
    return current;
  }
  let pos = if !inner_view(view_before) {
    view_size.width / 2.
  } else if view_after < 0. {
    0.
  } else {
    view_size.width - 1.
  };
  pos - after
}

fn to_content_pos(container: &ScrollableWidget, view_position: &Point) -> Point {
  *view_position - Size::from(container.scroll_pos.to_vector())
}

impl Compose for Input {
  fn compose(this: StateWidget<Self>) -> Widget
  where
    Self: Sized,
  {
    let placeholder = "\r";
    widget! {
        states {
          this: this.into_stateful(),
          focused: (false).into_stateful(),
        }
        init {
          let cursor_change = Rc::new(RefCell::new(Option::<(Point, Point)>::None));
          let scroll_change = cursor_change.clone();
        }
        ExpandBox {
          dir: ExpandDir::X,
          auto_focus: true,
          char: move |c| this.edit_handle(c.char),
          key_down: move |key| this.key_handle(key),
          pointer_move: move |e| {
            if let CaretState::Selecting(begin, _) = this.caret {
              if e.point_type == PointerType::Mouse
                && e.mouse_buttons() == MouseButtons::PRIMARY {
                let position = to_content_pos(&container, &e.position());
                let cluster = auxiliary.cluster_from_pos(position.x, position.y);
                this.caret = CaretState::Selecting(begin, cluster as usize);
              }
            }
          },
          pointer_down: move |e| {
            let position = to_content_pos(&container, &e.position());
            let cluster = auxiliary.cluster_from_pos(position.x, position.y);
            this.caret = CaretState::Selecting(cluster as usize, cluster as usize);
          },
          pointer_up: move |_| {
            if let CaretState::Selecting(begin, end) = this.caret {
              this.caret = if begin == end {
               CaretState::Caret(begin as usize)
              } else {
                CaretState::Select(begin, end)
              };
            }
          },
          focus_in: move |_| *focused = true,
          focus_out: move |_| *focused = false,

          ScrollableWidget {
            id: container,
            scrollable: Scrollable::X,
            performed_layout: move |_| {
              scroll_change.borrow_mut().take().map(|(before, after)| {
                let pos = auto_x_scroll_pos(&container, before.x, after.x);
                container.silent().jump_to(Point::new(pos, 0.));
              });
            },

            Stack {
              DynWidget {
                dyns: auxiliary.select_rects.clone().into_iter().map(|rc| {
                  widget! {
                    SelectedTextStyle {
                      top_anchor: rc.origin.y,
                      left_anchor: rc.origin.x,
                      Container {
                        background: Color::from_rgb(181, 215, 254),
                        size: rc.size.clone(),
                      }
                    }
                  }
                }).collect::<Vec<_>>()
              }
              InputLayout {
                id: auxiliary,
                performed_layout: move |ctx| {
                  auxiliary.silent().update_layout(&text.text, &text.style, this.caret, &ctx.app_ctx().typography_store);
                },
                Text {
                  id: text,
                  text: this.text.clone() + placeholder,
                  style: this.style.clone(),
                  padding: EdgeInsets::horizontal(1.),
                }
                CaretTrigger {
                  caret: this.caret,
                }
              }
              Text {
                visible: this.text.is_empty() && this.placeholder.is_some(),
                text: this.placeholder.clone().unwrap_or("".to_string()),
                style: this.placeholder_style.as_ref().unwrap_or(&this.style).clone()
              }

              DynWidget {
                dyns: (*focused).then(move || {
                  widget! {
                    CaretStyle{
                      top_anchor: auxiliary.cursor_offset.y,
                      left_anchor: auxiliary.cursor_offset.x,
                      font: text.style.clone(),
                      Container {
                        size: Size::new(1., auxiliary.cursor_height),
                      }
                    }
                  }
                })
              }
            }
          }
        }

        on auxiliary.cursor_offset {
          change: move |val| *cursor_change.borrow_mut() = Some(val)
        }
      }
  }
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

  fn cursor(&self) -> GraphemeCursor { GraphemeCursor(self.offset()) }

  fn offset(&self) -> usize {
    match *self {
      CaretState::Caret(cursor) => cursor,
      CaretState::Select(_, end) => end,
      CaretState::Selecting(_, end) => end,
    }
  }
}
