mod handle;
mod layout;
mod caret_state;
mod caret_style;
mod selected_style;

pub use caret_style::{CaretStyle};
pub use caret_state::{CaretState};
pub use selected_style::{SelectedTextStyle};
use layout::{InputLayout, CaretTrigger};

use crate::{
  layout::{Container, ExpandDir},
  prelude::{ExpandBox, Stack, Text},
};
use ribir_core::prelude::*;
use std::{cell::RefCell, rc::Rc};


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
