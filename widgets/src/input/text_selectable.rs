use super::caret_state::CaretPosition;
use super::CaretState;
use super::{glyphs_helper::GlyphsHelper, selected_text::SelectedText};
use crate::layout::{Stack, StackFit};
use crate::prelude::Text;
use ribir_core::prelude::*;

#[derive(Declare, Default)]
pub struct TextSelectable {
  #[declare(default)]
  pub caret: CaretState,
  #[declare(skip, default)]
  pub(crate) helper: GlyphsHelper,
}

impl ComposeChild for TextSelectable {
  type Child = State<Text>;
  fn compose_child(this: State<Self>, text: Self::Child) -> Widget {
    let this = this.into_writable();
    widget! {
      states {
        this: this.clone(),
        text: text.into_readonly(),
      }
      Stack {
        id: host,
        fit: StackFit::Passthrough,
        on_pointer_move: move |e| {
          if let CaretState::Selecting(begin, _) = this.caret {
            if e.point_type == PointerType::Mouse
              && e.mouse_buttons() == MouseButtons::PRIMARY {
              let position = e.position();
              let end = this.helper.caret_position_from_pos(position.x, position.y);
              this.caret = CaretState::Selecting(begin, end);
            }
          }
        },
        on_pointer_down: move |e| {
          let position = e.position();
          let end = this.helper.caret_position_from_pos(position.x, position.y);
          let begin = if e.with_shift_key() {
            match this.caret {
              CaretState::Caret(begin) |
              CaretState::Select(begin, _) |
              CaretState::Selecting(begin, _) => begin,
            }
          } else {
            end
          };
          this.caret = CaretState::Selecting(begin, end);
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
        on_double_tap: move |e| {
          let position = e.position();
          let caret = this.helper.caret_position_from_pos(position.x, position.y);
          let rg = select_word(&text.text, caret.cluster);
          this.caret = CaretState::Select(
            CaretPosition { cluster: rg.start, position: None },
            CaretPosition { cluster: rg.end, position: None }
          );
        },

        on_key_down: move |event| key_handle(&mut this, &text.text, event),
        SelectedText {
          id: selected,
          visible: host.has_focus(),
          rects: vec![],
        }
        DynWidget {
          dyns: text.clone(),
          on_performed_layout: move |ctx| {
            let bound = ctx.layout_clamp().expect("layout info must exit in performed_layout");
            this.helper.glyphs = Some(text.text_layout(
              AppCtx::typography_store(),
              bound.max,
            ));
            this.forget_modifies();
          }
        }
      }
      finally {
        this.modifies()
          .subscribe(move |_| {
            selected.rects = this.selected_rect();
          });
      }
    }
    .into()
  }
}

impl TextSelectable {
  pub fn cursor_layout(&self) -> (Point, f32) { self.helper.cursor(self.caret.caret_position()) }

  fn selected_rect(&self) -> Vec<Rect> { self.helper.selection(&self.caret.select_range()) }
}

fn key_handle(this: &mut RefState<TextSelectable>, text: &CowArc<str>, event: &mut KeyboardEvent) {
  let mut deal = false;
  if event.with_command_key() {
    deal = deal_with_command(this, text, event);
  }

  if !deal {
    deal_with_selection(this, text, event);
  }
}

fn deal_with_command(
  this: &mut RefState<TextSelectable>,
  text: &CowArc<str>,
  event: &mut KeyboardEvent,
) -> bool {
  match event.key {
    VirtualKeyCode::C => {
      let rg = this.caret.select_range();
      if !rg.is_empty() {
        let clipboard = AppCtx::clipboard();
        let _ = clipboard.borrow_mut().clear();
        let _ = clipboard.borrow_mut().write_text(&text.substr(rg));
      }
    }
    VirtualKeyCode::A => {
      this.caret = CaretState::Select(
        CaretPosition { cluster: 0, position: None },
        CaretPosition { cluster: text.len(), position: None },
      );
    }
    _ => return false,
  }
  true
}

fn is_move_by_word(event: &KeyboardEvent) -> bool {
  #[cfg(target_os = "macos")]
  return event.with_alt_key();
  #[cfg(not(target_os = "macos"))]
  return event.with_ctrl_key();
}

fn deal_with_selection(this: &mut RefState<TextSelectable>, text: &str, event: &mut KeyboardEvent) {
  let old_caret = this.caret;
  match event.key {
    VirtualKeyCode::Left => {
      if is_move_by_word(event) {
        let cluster = select_prev_word(text, this.caret.cluster(), false).start;
        this.caret = CaretPosition { cluster, position: None }.into();
      } else if event.with_command_key() {
        this.caret = this.helper.line_begin(this.caret.caret_position()).into()
      } else {
        this.caret = this.helper.prev(this.caret.caret_position()).into();
      }
    }
    VirtualKeyCode::Right => {
      if is_move_by_word(event) {
        let cluster = select_next_word(text, this.caret.cluster(), true).end;
        this.caret = CaretPosition { cluster, position: None }.into()
      } else if event.with_command_key() {
        this.caret = this.helper.line_end(this.caret.caret_position()).into()
      } else {
        this.caret = this.helper.next(this.caret.caret_position()).into();
      }
    }
    VirtualKeyCode::Up => {
      this.caret = this.helper.up(this.caret.caret_position()).into();
    }
    VirtualKeyCode::Down => {
      this.caret = this.helper.down(this.caret.caret_position()).into();
    }
    VirtualKeyCode::Home => this.caret = this.helper.line_begin(this.caret.caret_position()).into(),
    VirtualKeyCode::End => this.caret = this.helper.line_end(this.caret.caret_position()).into(),
    _ => (),
  }

  if event.with_shift_key() && old_caret != this.caret {
    this.caret = match old_caret {
      CaretState::Caret(begin) | CaretState::Select(begin, _) | CaretState::Selecting(begin, _) => {
        CaretState::Select(begin, this.caret.caret_position())
      }
    };
  }
}
