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
  type Child = FatObj<State<Text>>;
  fn compose_child(this: impl StateWriter<Value = Self>, text: Self::Child) -> impl WidgetBuilder {
    fn_widget! {
      let mut host = @Stack { fit: StackFit::Passthrough };
      let selected = @SelectedText {
        visible: pipe!($host.has_focus()),
        rects: pipe!($this.selected_rect()),
      };

      @$host {
        on_pointer_move: move |e| {
          let mut this = $this.write();
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
          let end = $this.helper.caret_position_from_pos(position.x, position.y);
          let begin = if e.with_shift_key() {
            match $this.caret {
              CaretState::Caret(begin) |
              CaretState::Select(begin, _) |
              CaretState::Selecting(begin, _) => begin,
            }
          } else {
            end
          };
          $this.write().caret = CaretState::Selecting(begin, end);
        },
        on_pointer_up: move |_| {
          let mut this = $this.write();
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
          let caret = $this.helper.caret_position_from_pos(position.x, position.y);
          let rg = select_word(&$text.text, caret.cluster);
          $this.write().caret = CaretState::Select(
            CaretPosition { cluster: rg.start, position: None },
            CaretPosition { cluster: rg.end, position: None }
          );
        },
        on_key_down: move |event| key_handle(&mut $this.write(), &$text.text, event),
        on_performed_layout: move |e: &mut LifecycleEvent| {
          let mut this = $this.write();
          let bound = e.layout_clamp().expect("layout info must exit in performed_layout");
          this.helper.glyphs = Some($text.text_layout(bound.max));
          this.forget_modifies();
        },
        @ { selected }
        @ { text }
      }
    }
  }
}

impl TextSelectable {
  pub fn cursor_layout(&self) -> (Point, f32) { self.helper.cursor(self.caret.caret_position()) }

  fn selected_rect(&self) -> Vec<Rect> { self.helper.selection(&self.caret.select_range()) }
}

fn key_handle(this: &mut TextSelectable, text: &CowArc<str>, event: &KeyboardEvent) {
  let mut deal = false;
  if event.with_command_key() {
    deal = deal_with_command(this, text, event);
  }

  if !deal {
    deal_with_selection(this, text, event);
  }
}

fn deal_with_command(this: &mut TextSelectable, text: &CowArc<str>, event: &KeyboardEvent) -> bool {
  // use the physical key to make sure the keyboard with different
  // layout use the same key as shortcut.
  match event.key_code() {
    PhysicalKey::Code(KeyCode::KeyC) => {
      let rg = this.caret.select_range();
      if !rg.is_empty() {
        let clipboard = AppCtx::clipboard();
        let _ = clipboard.borrow_mut().clear();
        let _ = clipboard.borrow_mut().write_text(&text.substr(rg));
      }
    }
    PhysicalKey::Code(KeyCode::KeyA) => {
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

fn deal_with_selection(this: &mut TextSelectable, text: &str, event: &KeyboardEvent) {
  let old_caret = this.caret;
  match event.key() {
    VirtualKey::Named(NamedKey::ArrowLeft) => {
      if is_move_by_word(event) {
        let cluster = select_prev_word(text, this.caret.cluster(), false).start;
        this.caret = CaretPosition { cluster, position: None }.into();
      } else if event.with_command_key() {
        this.caret = this.helper.line_begin(this.caret.caret_position()).into()
      } else {
        this.caret = this.helper.prev(this.caret.caret_position()).into();
      }
    }
    VirtualKey::Named(NamedKey::ArrowRight) => {
      if is_move_by_word(event) {
        let cluster = select_next_word(text, this.caret.cluster(), true).end;
        this.caret = CaretPosition { cluster, position: None }.into()
      } else if event.with_command_key() {
        this.caret = this.helper.line_end(this.caret.caret_position()).into()
      } else {
        this.caret = this.helper.next(this.caret.caret_position()).into();
      }
    }
    VirtualKey::Named(NamedKey::ArrowUp) => {
      this.caret = this.helper.up(this.caret.caret_position()).into();
    }
    VirtualKey::Named(NamedKey::ArrowDown) => {
      this.caret = this.helper.down(this.caret.caret_position()).into();
    }
    VirtualKey::Named(NamedKey::Home) => {
      this.caret = this.helper.line_begin(this.caret.caret_position()).into()
    }
    VirtualKey::Named(NamedKey::End) => {
      this.caret = this.helper.line_end(this.caret.caret_position()).into()
    }
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
