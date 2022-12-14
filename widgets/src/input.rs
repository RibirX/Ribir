mod caret;
mod caret_state;
mod handle;
mod input_text;
mod selected_text;
pub use caret::CaretStyle;
pub use caret_state::CaretState;
pub use selected_text::SelectedTextStyle;

use crate::layout::{ExpandBox, ExpandDir, Stack};
use ribir_core::prelude::*;

use self::{caret::Caret, input_text::InputText, selected_text::SelectedText};

#[derive(Declare)]
pub struct Input {
  #[declare(default)]
  pub text: String,
  #[declare(default = TypographyTheme::of(ctx).body1.text.clone())]
  pub style: TextStyle,

  #[declare(default)]
  pub caret: CaretState,
}

impl ComposeChild for Input {
  type Child = Option<Widget>;
  fn compose_child(this: StateWidget<Self>, placeholder: Self::Child) -> Widget
  where
    Self: Sized,
  {
    widget! {
      states {
        this: this.into_stateful(),
      }
      ExpandBox {
        id: outbox,
        dir: ExpandDir::X,
        auto_focus: true,
        char: move |c| this.edit_handle(c.char),
        key_down: move |key| this.key_handle(key),
        pointer_move: move |e| {
          if let CaretState::Selecting(begin, _) = this.caret {
            if e.point_type == PointerType::Mouse
              && e.mouse_buttons() == MouseButtons::PRIMARY {
              let position = to_content_pos(&container, &e.position());
              let cluster = text.glyphs_helper.borrow().cluster_from_pos(position.x, position.y);
              this.caret = CaretState::Selecting(begin, cluster as usize);
            }
          }
        },
        pointer_down: move |e| {
          let position = to_content_pos(&container, &e.position());
          let cluster = text.glyphs_helper.borrow().cluster_from_pos(position.x, position.y);
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

        ScrollableWidget {
          id: container,
          scrollable: Scrollable::X,

          Stack {
            padding: EdgeInsets::horizontal(1.),
            SelectedText {
              caret: this.caret,
              glyphs_helper: text.glyphs_helper.clone(),
            }
            InputText {
              id: text,
              text: this.text.clone(),
              style: this.style.clone(),
            }

            DynWidget {
              visible: this.text.is_empty(),
              dyns: placeholder,
            }

            DynWidget {
              dyns: (outbox.has_focus()).then(move || widget! {
                Caret {
                  id: caret,
                  caret: this.caret,
                  font: this.style.clone(),
                  glyphs_helper: text.glyphs_helper.clone(),
                }

                finally {
                  let_watch!(caret.layout_pos())
                    .scan_initial((Point::zero(), Point::zero()), |pair, v| (pair.1, v))
                    .distinct_until_changed()
                    .subscribe(move |(before, after)| {
                      let pos = auto_x_scroll_pos(&container, before.x, after.x);
                      container.silent().jump_to(Point::new(pos, 0.));
                    });
                }
              })
            }
          }
        }
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
