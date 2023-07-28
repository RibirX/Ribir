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
              let cluster = this.helper.cluster_from_pos(position.x, position.y);
              this.caret = CaretState::Selecting(begin, cluster as usize);
            }
          }
        },
        on_pointer_down: move |e| {
          let position = e.position();
          let cluster = this.helper.cluster_from_pos(position.x, position.y);
          let begin = if e.with_shift_key() {
            match this.caret {
              CaretState::Caret(begin) => begin,
              CaretState::Select(begin, _) => begin,
              CaretState::Selecting(begin, _) => begin,
            }
          } else {
            cluster as usize
          };
          this.caret = CaretState::Selecting(begin, cluster as usize);
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
          let cluster = this.helper.cluster_from_pos(position.x, position.y);
          let rg = select_word(&text.text, cluster as usize);
          this.caret = CaretState::Select(rg.start, rg.end);
        },

        on_key_down: move |event| key_handle(&mut this, &text.text, event),
        SelectedText {
          id: selected,
          visible: host.has_focus(),
          rects: vec![],
        }
        DynWidget {
          dyns: text.clone().into_widget(),
          on_performed_layout: move |ctx| {
            let bound = ctx.layout_info().expect("layout info must exit in performed_layout").clamp;
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
    .into_widget()
  }
}

impl TextSelectable {
  pub fn cursor_layout(&self) -> (Point, f32) { self.helper.cursor(self.caret.offset()) }

  fn selected_rect(&self) -> Vec<Rect> { self.helper.selection(&self.caret.select_range()) }
}

fn key_handle(this: &mut StateRef<TextSelectable>, text: &CowArc<str>, event: &mut KeyboardEvent) {
  let mut deal = false;
  if event.with_command_key() {
    deal = deal_with_command(this, text, event);
  }

  if !deal {
    deal_with_selection(this, text, event);
  }
}

fn deal_with_command(
  this: &mut StateRef<TextSelectable>,
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
      this.caret = CaretState::Select(0, text.len());
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

fn move_to_line_begin(this: &mut StateRef<TextSelectable>) {
  let (row, _) = this.helper.glyph_position(this.caret.offset());
  this.caret = this.helper.cluster_from_glyph_position(row, 0).into();
}

fn move_to_line_end(this: &mut StateRef<TextSelectable>) {
  let (row, _) = this.helper.glyph_position(this.caret.offset());
  this.caret = this.helper.cluster_from_glyph_position(row + 1, 0).into();
}

fn deal_with_selection(this: &mut StateRef<TextSelectable>, text: &str, event: &mut KeyboardEvent) {
  let old_caret = this.caret;
  match event.key {
    VirtualKeyCode::Left => {
      if is_move_by_word(event) {
        this.caret = select_prev_word(text, this.caret.offset(), false)
          .start
          .into();
      } else if event.with_command_key() {
        move_to_line_begin(this);
      } else {
        this.caret = this.helper.prev_cluster(this.caret.offset()).into();
      }
    }
    VirtualKeyCode::Right => {
      if is_move_by_word(event) {
        this.caret = select_next_word(text, this.caret.offset(), true).end.into();
      } else if event.with_command_key() {
        move_to_line_end(this);
      } else {
        this.caret = this.helper.next_cluster(this.caret.offset()).into();
      }
    }
    VirtualKeyCode::Up => {
      this.caret = this.helper.up_cluster(this.caret.offset()).into();
    }
    VirtualKeyCode::Down => {
      this.caret = this.helper.down_cluster(this.caret.offset()).into();
    }
    VirtualKeyCode::Home => {
      move_to_line_begin(this);
    }
    VirtualKeyCode::End => {
      move_to_line_end(this);
    }
    _ => (),
  }

  if event.with_shift_key() && old_caret != this.caret {
    this.caret = match old_caret {
      CaretState::Caret(begin) | CaretState::Select(begin, _) | CaretState::Selecting(begin, _) => {
        CaretState::Select(begin, this.caret.offset())
      }
    };
  }
}
