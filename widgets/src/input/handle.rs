#![allow(clippy::needless_lifetimes)]
use std::ops::{Deref, DerefMut};

use ribir_core::{
  events::{ImePreEdit, ImePreEditEvent},
  prelude::{
    AppCtx, CharsEvent, CowArc, GraphemeCursor, KeyCode, KeyboardEvent, NamedKey, PhysicalKey,
    StateWriter, Text, TextWriter, VirtualKey, select_next_word, select_prev_word, select_word,
  },
};

use super::{
  CaretPosition, CaretState, SelectRegionData, SelectRegionEvent, glyphs_helper::GlyphsHelper,
};

pub trait EditableText: SelectableText {
  fn set_text_with_caret(&mut self, text: &str, caret: CaretState);

  fn chars_handle(&mut self, event: &CharsEvent) -> bool {
    if event.common.with_command_key() {
      return false;
    }

    let chars = event
      .chars
      .chars()
      .filter(|c| !c.is_control() || c.is_ascii_whitespace())
      .collect::<String>();
    if !chars.is_empty() {
      let rg = self.caret().select_range();
      let mut writer = TextCaretWriter::new(self);
      writer.delete_byte_range(&rg);
      writer.insert_str(&chars);
      return true;
    }
    false
  }

  fn keys_handle(&mut self, text: &Text, event: &KeyboardEvent) -> bool {
    if self.keys_select_handle(text, event) {
      return true;
    }
    let mut deal = false;
    if event.with_command_key() {
      deal = edit_with_command(self, event);
    }
    if !deal {
      deal = edit_with_key(self, event);
    }
    deal
  }
}

pub trait SelectableText: Sized {
  fn text(&self) -> CowArc<str>;

  fn caret(&self) -> CaretState;

  fn set_caret(&mut self, caret: CaretState);

  fn keys_select_handle(&mut self, text: &Text, event: &KeyboardEvent) -> bool {
    if self.text() != text.text {
      return false;
    }

    let mut deal = false;
    if event.with_command_key() {
      deal = select_with_command(self, event);
    }

    if !deal {
      deal = select_with_key(self, text, event);
    }
    deal
  }

  fn select_region_handle(&mut self, text: &Text, e: &SelectRegionEvent) -> bool {
    let glyphs = text.glyphs().unwrap();
    let e = e.data();
    let caret = match e {
      SelectRegionData::SelectRect { from, to } => {
        let begin = glyphs.caret_position_from_pos(*from);
        let end = glyphs.caret_position_from_pos(*to);
        CaretState::Select(begin, end)
      }
      SelectRegionData::SetTo(pos) => {
        let caret = glyphs.caret_position_from_pos(*pos);
        CaretState::Caret(caret)
      }
      SelectRegionData::ShiftTo(pos) => {
        let caret = glyphs.caret_position_from_pos(*pos);
        match self.caret() {
          CaretState::Select(begin, _) | CaretState::Caret(begin) => {
            CaretState::Select(begin, caret)
          }
        }
      }
      SelectRegionData::DoubleSelect(pos) => {
        let caret = glyphs.caret_position_from_pos(*pos);
        let rg = select_word(&text.text, caret.cluster);
        CaretState::Select(CaretPosition { cluster: rg.start, position: None }, CaretPosition {
          cluster: rg.end,
          position: None,
        })
      }
    };
    self.set_caret(caret);
    true
  }
}

pub struct TextCaretWriter<'a, H>
where
  H: EditableText,
{
  host: &'a mut H,
  writer: TextWriter<GraphemeCursor>,
}

impl<'a, H> TextCaretWriter<'a, H>
where
  H: EditableText,
{
  pub fn new(host: &'a mut H) -> Self {
    let cursor = GraphemeCursor(host.caret().cluster());
    let string = host.text().to_string();
    Self { host, writer: TextWriter::new(string, cursor) }
  }
}

impl<'a, H> Drop for TextCaretWriter<'a, H>
where
  H: EditableText,
{
  fn drop(&mut self) {
    use crate::input::caret_state::CaretPosition;
    let Self { host, writer } = self;
    let text = writer.text().to_string();
    let caret = CaretPosition { cluster: writer.byte_offset(), position: None };

    host.set_text_with_caret(&text, caret.into());
  }
}

impl<'a, H> Deref for TextCaretWriter<'a, H>
where
  H: EditableText,
{
  type Target = TextWriter<GraphemeCursor>;
  fn deref(&self) -> &Self::Target { &self.writer }
}

impl<'a, H> DerefMut for TextCaretWriter<'a, H>
where
  H: EditableText,
{
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.writer }
}

#[derive(Debug)]
struct PreEditState {
  position: usize,
  value: Option<String>,
}

pub struct ImeHandle<H> {
  host: H,
  pre_edit: Option<PreEditState>,
}

impl<E, H> ImeHandle<H>
where
  E: EditableText,
  H: StateWriter<Value = E>,
{
  pub fn new(host: H) -> Self { Self { host, pre_edit: None } }

  pub fn is_in_pre_edit(&self) -> bool { self.pre_edit.is_some() }

  pub fn process_pre_edit(&mut self, e: &ImePreEditEvent) {
    match &e.pre_edit {
      ImePreEdit::Begin => {
        let mut host = self.host.write();
        let rg = host.caret().select_range();
        let mut writer = TextCaretWriter::new(&mut *host);
        writer.delete_byte_range(&rg);
        self.pre_edit = Some(PreEditState { position: rg.start, value: None });
      }
      ImePreEdit::PreEdit { value, cursor } => {
        let Some(PreEditState { position, value: edit_value }) = self.pre_edit.as_mut() else {
          return;
        };
        let mut host = self.host.write();
        let mut writer = TextCaretWriter::new(&mut *host);
        if let Some(txt) = edit_value {
          writer.delete_byte_range(&(*position..*position + txt.len()));
        }
        writer.insert_str(value);
        writer.set_to(*position + cursor.map_or(0, |(start, _)| start));
        *edit_value = Some(value.clone());
      }
      ImePreEdit::End => {
        if let Some(PreEditState { value: Some(txt), position, .. }) = self.pre_edit.take() {
          let mut host = self.host.write();
          let mut writer = TextCaretWriter::new(&mut *host);
          writer.delete_byte_range(&(position..position + txt.len()));
        }
      }
    }
  }
}

fn edit_with_command<F: EditableText>(this: &mut F, event: &KeyboardEvent) -> bool {
  if !event.with_command_key() {
    return false;
  }

  // use the physical key to make sure the keyboard with different
  // layout use the same key as shortcut.
  match event.key_code() {
    PhysicalKey::Code(KeyCode::KeyV) => {
      let clipboard = AppCtx::clipboard();
      let txt = clipboard.borrow_mut().read_text();
      if let Ok(txt) = txt {
        let rg = this.caret().select_range();
        let mut writer = TextCaretWriter::new(this);
        if !rg.is_empty() {
          writer.delete_byte_range(&rg);
        }
        writer.insert_chars(&txt);
      }
      true
    }
    PhysicalKey::Code(KeyCode::KeyX) => {
      let rg = this.caret().select_range();
      if !rg.is_empty() {
        let txt = this.text().substr(rg.clone()).to_string();
        TextCaretWriter::new(&mut *this).delete_byte_range(&rg);
        let clipboard = AppCtx::clipboard();
        let _ = clipboard.borrow_mut().clear();
        let _ = clipboard.borrow_mut().write_text(&txt);
      }
      true
    }
    _ => false,
  }
}

fn edit_with_key<F: EditableText>(this: &mut F, key: &KeyboardEvent) -> bool {
  match key.key() {
    VirtualKey::Named(NamedKey::Backspace) => {
      let rg = this.caret().select_range();
      if rg.is_empty() {
        TextCaretWriter::new(&mut *this).back_space();
      } else {
        TextCaretWriter::new(&mut *this).delete_byte_range(&rg);
      }
    }
    VirtualKey::Named(NamedKey::Delete) => {
      let rg = this.caret().select_range();
      if rg.is_empty() {
        TextCaretWriter::new(&mut *this).del_char();
      } else {
        TextCaretWriter::new(&mut *this).delete_byte_range(&rg);
      }
    }
    _ => (),
  };
  true
}
fn select_with_command(this: &mut impl SelectableText, event: &KeyboardEvent) -> bool {
  // use the physical key to make sure the keyboard with different
  // layout use the same key as shortcut.
  match event.key_code() {
    PhysicalKey::Code(KeyCode::KeyC) => {
      let rg = this.caret().select_range();
      let text = this.text();
      let selected_text = &text[rg];
      if !text.is_empty() {
        let clipboard = AppCtx::clipboard();
        let _ = clipboard.borrow_mut().clear();
        let _ = clipboard.borrow_mut().write_text(selected_text);
      }
      true
    }
    PhysicalKey::Code(KeyCode::KeyA) => {
      let len = this.text().len();
      if len > 0 {
        this.set_caret(CaretState::Select(
          CaretPosition { cluster: 0, position: None },
          CaretPosition { cluster: len, position: None },
        ));
      }
      true
    }
    _ => false,
  }
}

fn is_move_by_word(event: &KeyboardEvent) -> bool {
  #[cfg(target_os = "macos")]
  return event.with_alt_key();
  #[cfg(not(target_os = "macos"))]
  return event.with_ctrl_key();
}

fn select_with_key(this: &mut impl SelectableText, text: &Text, event: &KeyboardEvent) -> bool {
  let Some(glyphs) = text.glyphs() else { return false };

  let old_caret = this.caret();
  let text = text.text.clone();
  let new_caret_position = match event.key() {
    VirtualKey::Named(NamedKey::ArrowLeft) => {
      if is_move_by_word(event) {
        let cluster = select_prev_word(&text, old_caret.cluster(), false).start;
        Some(CaretPosition { cluster, position: None })
      } else if event.with_command_key() {
        Some(glyphs.line_begin(old_caret.caret_position()))
      } else {
        Some(glyphs.prev(old_caret.caret_position()))
      }
    }
    VirtualKey::Named(NamedKey::ArrowRight) => {
      if is_move_by_word(event) {
        let cluster = select_next_word(&text, old_caret.cluster(), true).end;
        Some(CaretPosition { cluster, position: None })
      } else if event.with_command_key() {
        Some(glyphs.line_end(old_caret.caret_position()))
      } else {
        Some(glyphs.next(old_caret.caret_position()))
      }
    }
    VirtualKey::Named(NamedKey::ArrowUp) => Some(glyphs.up(old_caret.caret_position())),
    VirtualKey::Named(NamedKey::ArrowDown) => Some(glyphs.down(old_caret.caret_position())),
    VirtualKey::Named(NamedKey::Home) => Some(glyphs.line_begin(old_caret.caret_position())),
    VirtualKey::Named(NamedKey::End) => Some(glyphs.line_end(old_caret.caret_position())),
    _ => None,
  };

  if new_caret_position.is_some() {
    let caret = if event.with_shift_key() {
      match old_caret {
        CaretState::Caret(begin) | CaretState::Select(begin, _) => {
          CaretState::Select(begin, new_caret_position.unwrap())
        }
      }
    } else {
      new_caret_position.unwrap().into()
    };
    this.set_caret(caret);
  }
  new_caret_position.is_some()
}
