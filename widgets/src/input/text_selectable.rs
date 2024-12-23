use std::ops::Range;

use ribir_core::prelude::*;

use super::glyphs_helper::TextGlyphsHelper;
use crate::{
  input::{glyphs_helper::GlyphsHelper, selected_text::*},
  prelude::*,
};

#[derive(Declare, Default)]
pub struct TextSelectable {
  #[declare(default)]
  pub caret: CaretState,

  #[declare(skip)]
  text: CowArc<str>,
}

pub trait SelectableText {
  fn selected_text(&self) -> Substr {
    let rg = self.select_range();
    self.text().substr(rg)
  }

  fn select_range(&self) -> Range<usize>;

  fn text(&self) -> &CowArc<str>;

  fn caret(&self) -> CaretState;

  fn set_caret(&mut self, caret: CaretState);

  fn select_text_rect(&self, text: &Text) -> Vec<Rect> {
    text
      .glyphs()
      .and_then(|glyphs| {
        let helper = TextGlyphsHelper::new(text.text.clone(), glyphs.clone());
        helper.selection(self.text(), &self.select_range())
      })
      .unwrap_or_default()
  }

  fn caret_position(&self, text: &Text) -> Option<Point> {
    text.glyphs().and_then(|glyphs| {
      let helper = TextGlyphsHelper::new(text.text.clone(), glyphs.clone());
      helper.cursor(self.text(), self.caret().caret_position())
    })
  }

  fn current_line_height(&self, text: &Text) -> Option<f32> {
    text.glyphs().and_then(|glyphs| {
      let helper = TextGlyphsHelper::new(text.text.clone(), glyphs.clone());
      helper.line_height(self.text(), self.caret().caret_position())
    })
  }
}

impl SelectableText for TextSelectable {
  fn select_range(&self) -> Range<usize> { self.caret.select_range() }
  fn text(&self) -> &CowArc<str> { &self.text }
  fn caret(&self) -> CaretState { self.caret }
  fn set_caret(&mut self, caret: CaretState) { self.caret = caret; }
}

impl TextSelectable {
  fn reset(&mut self, text: &CowArc<str>) {
    self.text = text.clone();
    self.caret = CaretState::default();
  }
}

pub(crate) fn bind_point_listener<T: SelectableText>(
  this: impl StateWriter<Value = T> + 'static, host: Widget, text: Reader<Text>,
) -> Widget {
  fn_widget! {
    let host = FatObj::new(host);
    @ $host {
      on_pointer_down: move |e| {
        let mut this = $this.write();
        let position = e.position();
        if let Some(helper) = $text.glyphs() {
          let end = helper.caret_position_from_pos(position.x, position.y);
          let begin = if e.with_shift_key() {
            match this.caret() {
              CaretState::Caret(begin) |
              CaretState::Select(begin, _) |
              CaretState::Selecting(begin, _) => begin,
            }
          } else {
            end
          };
          this.set_caret(CaretState::Selecting(begin, end));
        }
      },
      on_pointer_move: move |e| {
        let mut this = $this.write();
        if let CaretState::Selecting(begin, _) = this.caret() {
          if e.point_type == PointerType::Mouse
            && e.mouse_buttons() == MouseButtons::PRIMARY {
              if let Some(glyphs) = $text.glyphs() {
                let position = e.position();
                let end = glyphs.caret_position_from_pos(position.x, position.y);
                this.set_caret(CaretState::Selecting(begin, end));
              }
          }
        }
      },
      on_pointer_up: move |_| {
        let mut this = $this.write();
        if let CaretState::Selecting(begin, end) = this.caret() {
          let caret = if begin == end {
            CaretState::Caret(begin)
          } else {
            CaretState::Select(begin, end)
          };

          this.set_caret(caret);
        }
      },
      on_double_tap: move |e| {
        if let Some(glyphs) = $text.glyphs() {
          let position = e.position();
          let caret = glyphs.caret_position_from_pos(position.x, position.y);
          let rg = select_word(&$text.text, caret.cluster);
          $this.write().set_caret(CaretState::Select(
            CaretPosition { cluster: rg.start, position: None },
            CaretPosition { cluster: rg.end, position: None }
          ));
        }
      }
    }
  }
  .into_widget()
}

impl ComposeChild<'static> for TextSelectable {
  type Child = FatObj<State<Text>>;
  fn compose_child(this: impl StateWriter<Value = Self>, text: Self::Child) -> Widget<'static> {
    let src = text.into_inner();

    fn_widget! {
      let  text = @ $src {};
      $this.silent().text = $text.text.clone();
      watch!($text.text.clone())
        .subscribe(move |v| {
          if $this.text != $text.text {
            $this.write().reset(&v);
          }
        });

      let only_text = text.clone_reader();

      let stack = @Stack {
        fit: StackFit::Loose,
      };

      let high_light_rect = @ OnlySizedByParent {
        @ SelectedHighLight {
          rects: pipe! { $this.select_text_rect(&$text)}
        }
      };
      let text_widget = text.into_widget();
      let text_widget = bind_point_listener(
        this.clone_writer(),
        text_widget,
        only_text.clone_reader(),
      );

      @ $stack {
        tab_index: -1_i16,
        on_blur: move |_| { $this.write().set_caret(CaretState::default()); },
        on_key_down: move |k| {
          select_key_handle(&this, &$only_text,  k);
        },
        @ $high_light_rect { }
        @ $text_widget {}
      }
    }
    .into_widget()
  }
}

pub(crate) fn select_key_handle<F: SelectableText>(
  this: &impl StateWriter<Value = F>, text: &Text, event: &KeyboardEvent,
) {
  let mut deal = false;
  if event.with_command_key() {
    deal = deal_with_command(this, event);
  }

  if !deal {
    deal_with_selection(this, text, event);
  }
}

fn deal_with_command<F: SelectableText>(
  this: &impl StateWriter<Value = F>, event: &KeyboardEvent,
) -> bool {
  // use the physical key to make sure the keyboard with different
  // layout use the same key as shortcut.
  match event.key_code() {
    PhysicalKey::Code(KeyCode::KeyC) => {
      let text = this.read().selected_text();
      if !text.is_empty() {
        let clipboard = AppCtx::clipboard();
        let _ = clipboard.borrow_mut().clear();
        let _ = clipboard.borrow_mut().write_text(&text);
      }
    }
    PhysicalKey::Code(KeyCode::KeyA) => {
      let len = this.read().text().len();
      this.write().set_caret(CaretState::Select(
        CaretPosition { cluster: 0, position: None },
        CaretPosition { cluster: len, position: None },
      ));
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

fn deal_with_selection<F: SelectableText>(
  this: &impl StateWriter<Value = F>, text: &Text, event: &KeyboardEvent,
) {
  let Some(glyphs) = text.glyphs() else { return };
  let helper = TextGlyphsHelper::new(text.text.clone(), glyphs.clone());

  let old_caret = this.read().caret();
  let text = this.read().text().clone();
  let new_caret_position = match event.key() {
    VirtualKey::Named(NamedKey::ArrowLeft) => {
      if is_move_by_word(event) {
        let cluster = select_prev_word(&text, old_caret.cluster(), false).start;
        Some(CaretPosition { cluster, position: None })
      } else if event.with_command_key() {
        helper.line_begin(&text, old_caret.caret_position())
      } else {
        helper.prev(&text, old_caret.caret_position())
      }
    }
    VirtualKey::Named(NamedKey::ArrowRight) => {
      if is_move_by_word(event) {
        let cluster = select_next_word(&text, old_caret.cluster(), true).end;
        Some(CaretPosition { cluster, position: None })
      } else if event.with_command_key() {
        helper.line_end(&text, old_caret.caret_position())
      } else {
        helper.next(&text, old_caret.caret_position())
      }
    }
    VirtualKey::Named(NamedKey::ArrowUp) => helper.up(&text, old_caret.caret_position()),
    VirtualKey::Named(NamedKey::ArrowDown) => helper.down(&text, old_caret.caret_position()),
    VirtualKey::Named(NamedKey::Home) => helper.line_begin(&text, old_caret.caret_position()),
    VirtualKey::Named(NamedKey::End) => helper.line_end(&text, old_caret.caret_position()),
    _ => None,
  };

  if new_caret_position.is_some() {
    if event.with_shift_key() {
      this.write().set_caret(match old_caret {
        CaretState::Caret(begin)
        | CaretState::Select(begin, _)
        | CaretState::Selecting(begin, _) => CaretState::Select(begin, new_caret_position.unwrap()),
      })
    } else {
      this
        .write()
        .set_caret(new_caret_position.unwrap().into())
    }
  }
}
