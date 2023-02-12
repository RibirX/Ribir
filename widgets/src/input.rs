mod caret;
mod caret_state;
mod glyphs_helper;
mod handle;
mod selected_text;
pub use caret_state::CaretState;

use self::{caret::Caret, glyphs_helper::GlyphsHelper, selected_text::SelectedText};
use crate::layout::{ConstrainedBox, Stack};
use crate::prelude::Text;
use ribir_core::{prelude::*, ticker::FrameMsg};

pub struct Placeholder(CowArc<str>);

#[derive(Clone, PartialEq)]
pub struct InputTheme {
  pub min_length: f32,
  pub select_background: Brush,
  pub caret_color: Brush,
}
impl CustomTheme for InputTheme {}

#[derive(Declare)]
pub struct Input {
  #[declare(default = TypographyTheme::of(ctx).body1.text.clone())]
  pub style: TextStyle,
  #[declare(skip)]
  text: CowArc<str>,
  #[declare(skip)]
  caret: CaretState,
  #[declare(default = InputTheme::of(ctx).min_length)]
  min_length: f32,
}

impl Input {
  pub fn text(&self) -> CowArc<str> { self.text.clone() }

  pub fn caret(&self) -> &CaretState { &self.caret }

  pub fn set_text(&mut self, text: impl Into<CowArc<str>>) {
    self.text = text.into();
    self.caret.valid(self.text.len());
  }

  pub fn set_caret(&mut self, caret: CaretState) {
    self.caret = caret;
    self.caret.valid(self.text.len());
  }
}

impl ComposeChild for Input {
  type Child = Option<Placeholder>;
  fn compose_child(this: State<Self>, placeholder: Self::Child) -> Widget {
    widget! {
      states {
        this: this.into_writable(),
        helper: Stateful::new(GlyphsHelper::default()),
      }
      init ctx => {
        let tick_of_layout_ready = ctx.wnd_ctx()
          .frame_tick_stream()
          .filter(|msg| matches!(msg, FrameMsg::LayoutReady(_)));
      }

      ConstrainedBox {
        id: outbox,
        clamp: BoxClamp {
          min: Size::new(input_width(this.style.font_size, this.min_length), 0.),
          max: Size::new(input_width(this.style.font_size, this.min_length), f32::INFINITY)
        },
        auto_focus: true,
        on_char: move |char_event| this.edit_handle(char_event),
        on_key_down: move |key| this.key_handle(key),
        on_pointer_move: move |e| {
          if let CaretState::Selecting(begin, _) = this.caret {
            if e.point_type == PointerType::Mouse
              && e.mouse_buttons() == MouseButtons::PRIMARY {
              let position = to_content_pos(&container, &e.position());
              let cluster = helper.cluster_from_pos(position.x, position.y);
              this.caret = CaretState::Selecting(begin, cluster as usize);
            }
          }
        },
        on_pointer_down: move |e| {
          let position = to_content_pos(&container, &e.position());
          let cluster = helper.cluster_from_pos(position.x, position.y);
          this.caret = CaretState::Selecting(cluster as usize, cluster as usize);
        },
        on_pointer_up: move |_| {
          if let CaretState::Selecting(begin, end) = this.caret {
            this.caret = if begin == end {
             CaretState::Caret(begin)
            } else {
              CaretState::Select(begin, end)
            };
          }
        },

        ScrollableWidget {
          id: container,
          scrollable: Scrollable::X,
          padding: EdgeInsets::horizontal(1.),
          Stack {
            SelectedText {
              id: selected,
              rects: vec![],
            }
            Text {
              id: text,
              text: this.text.clone(),
              style: this.style.clone(),

              on_performed_layout: move |ctx| {
                let bound = ctx.layout_info().expect("layout info must exit in performed_layout").clamp;
                helper.glyphs = Some(Text::text_layout(
                  &text.text,
                  &text.style,
                  ctx.wnd_ctx().typography_store(),
                  bound,
                ));
              }
            }

            DynWidget {
              dyns: placeholder.map(|holder| {
                widget! {
                  Text {
                    visible: this.text.is_empty(),
                    text: holder.0,
                  }
                }
              })
            }

            Caret{
              id: caret,
              top_anchor: 0.,
              left_anchor: 0.,
              focused: outbox.has_focus(),
              size: Size::new(1., 0.),
            }
          }
        }
      }
      finally {
        let_watch!(this.caret)
          .distinct_until_changed()
          .sample(tick_of_layout_ready)
          .subscribe(move |cursor| {
            selected.rects = helper.selection(&cursor.select_range());
            let (offset, height) = helper.cursor(cursor.offset());
            caret.top_anchor = PositionUnit::Pixel(offset.y);
            caret.left_anchor = PositionUnit::Pixel(offset.x);
            caret.size = Size::new(1., height);
          });
        let_watch!(caret.left_anchor.abs_value(1.))
          .scan_initial((0., 0.), |pair, v| (pair.1, v))
          .distinct_until_changed()
          .subscribe(move |(before, after)| {
            let pos = auto_x_scroll_pos(&container, before, after);
            container.silent().jump_to(Point::new(pos, 0.));
          });

        // let_watch!(this.caret).distinct_until_changed() will only be triggered after modify
        // borrow mut from state_ref to manual triggered after init.
        let _:&mut Input = &mut this;
      }
    }
  }
}

impl Placeholder {
  #[inline]
  pub fn new(str: impl Into<CowArc<str>>) -> Self { Self(str.into()) }
}

fn input_width(font_size: FontSize, length: f32) -> f32 {
  FontSize::Em(Em::relative_to(length, font_size))
    .into_pixel()
    .value()
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
