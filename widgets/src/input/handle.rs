use std::ops::{Deref, DerefMut};

use ribir_core::prelude::{CharsEvent, GraphemeCursor, KeyboardEvent, TextWriter, VirtualKeyCode};

struct InputWriter<'a> {
  input: &'a mut TextEditorArea,
  writer: TextWriter<GraphemeCursor>,
}

impl<'a> InputWriter<'a> {
  fn new(input: &'a mut TextEditorArea) -> Self {
    let cursor = GraphemeCursor(input.caret.offset());
    let string = input.text.to_string();
    Self {
      input,
      writer: TextWriter::new(string, cursor),
    }
  }
}

impl<'a> Drop for InputWriter<'a> {
  fn drop(&mut self) {
    let Self { input, writer } = self;
    input.caret = writer.byte_offset().into();
    input.text = writer.text().clone().into();
  }
}

impl<'a> Deref for InputWriter<'a> {
  type Target = TextWriter<GraphemeCursor>;
  fn deref(&self) -> &Self::Target { &self.writer }
}

impl<'a> DerefMut for InputWriter<'a> {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.writer }
}

use super::{glyphs_helper::GlyphsHelper, TextEditorArea};
impl TextEditorArea {
  pub(crate) fn edit_handle(&mut self, event: &mut CharsEvent) {
    if event.common.with_command_key() {
      return;
    }
    let chars = event
      .chars
      .chars()
      .filter(|c| !c.is_control())
      .collect::<String>();
    if !chars.is_empty() {
      let rg = self.caret.select_range();
      let mut writer = InputWriter::new(self);
      writer.delete_byte_range(&rg);
      writer.insert_str(&chars);
    }
  }

  pub(crate) fn key_handle(&mut self, event: &mut KeyboardEvent, helper: &GlyphsHelper) {
    let mut deal = false;
    if event.common.with_command_key() {
      deal = self.key_with_command(event, helper)
    }

    if deal {
      return;
    }
    self.single_key(event, helper);
  }

  fn key_with_command(&mut self, event: &mut KeyboardEvent, _helper: &GlyphsHelper) -> bool {
    if event.key == VirtualKeyCode::V && event.common.with_command_key() {
      let clipboard = event.context().clipboard();
      let txt = clipboard.borrow_mut().read_text();
      if let Ok(txt) = txt {
        let rg = self.caret.select_range();
        let mut writer = InputWriter::new(self);
        if !rg.is_empty() {
          writer.delete_byte_range(&rg);
        }
        writer.insert_chars(&txt);
      }
      return true;
    }
    false
  }

  fn single_key(&mut self, key: &mut KeyboardEvent, helper: &GlyphsHelper) -> bool {
    match key.key {
      VirtualKeyCode::Left => {
        self.caret = helper.prev_cluster(self.caret.offset()).into();
      }
      VirtualKeyCode::Right => {
        self.caret = helper.next_cluster(self.caret.offset()).into();
      }
      VirtualKeyCode::Up => {
        self.caret = helper.up_cluster(self.caret.offset()).into();
      }
      VirtualKeyCode::Down => {
        self.caret = helper.down_cluster(self.caret.offset()).into();
      }
      VirtualKeyCode::NumpadEnter | VirtualKeyCode::Return => {
        if self.multi_line {
          InputWriter::new(self).insert_str("\r");
        }
      }
      VirtualKeyCode::Back => {
        let rg = self.caret.select_range();
        if rg.is_empty() {
          InputWriter::new(self).back_space();
        } else {
          InputWriter::new(self).delete_byte_range(&rg);
        }
      }
      VirtualKeyCode::Delete => {
        let rg = self.caret.select_range();
        if rg.is_empty() {
          InputWriter::new(self).del_char();
        } else {
          InputWriter::new(self).delete_byte_range(&rg);
        }
      }
      _ => (),
    };
    true
  }
}
