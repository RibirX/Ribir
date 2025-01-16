use std::ops::Range;

use ribir_core::{
  events::{ImePreEdit, ImePreEditEvent},
  prelude::{
    AppCtx, CharsEvent, KeyCode, KeyboardEvent, NamedKey, PhysicalKey, StateWriter, Substr,
    VirtualKey, *,
  },
};

use super::{
  CaretPosition,
  edit_text::EditText,
  text_glyphs::{TextGlyphs, TextGlyphsPainter, TextGlyphsProvider},
  text_selection::TextSelection,
};
use crate::{input::text_glyphs::VisualGlyphsHelper, prelude::*};

class_names! {
  #[doc = "Class name for the text caret"]
  TEXT_CARET,
}

pub fn edit_text<T: VisualText + EditText + 'static + Clone>(
  text: impl StateWriter<Value = TextGlyphs<T>>,
  selection: impl StateWriter<Value = TextSelection<T>>,
) -> Widget<'static> {
  fn_widget! {
    let mut writer = TextWriter {
      host: part_writer!(text.text_mut()),
      selection: selection.clone_writer()
    };
    let ime = Stateful::new(ImeHandle::new(writer.clone()));
    let mut text = FatObj::new(@$text {});
    let mut caret = FatObj::new(@ {
      let selection = selection.clone_writer();
      pipe!($text.has_focus()).map(move |v|
        if v {
          caret_widget(selection.clone_watcher())
        } else {
          @Void{}.into_widget()
        }
      )
    });

    let ctx = BuildCtx::get();
    let wnd = ctx.window();
    let scrollable = Provider::state_of::<ScrollableProvider>(ctx).map(|p| p.clone_writer());
    let u = watch!($caret.layout_rect())
      .subscribe(move |mut rect| {
        if $text.has_focus() && !$ime.is_in_pre_edit() {
          if let Some(wid) = $caret.track_id().get() {
            if let Some(scrollable) = &scrollable {
              scrollable.write().ensure_visible(wid, Anchor::default(), &wnd);
            }
            rect.origin = wnd.map_to_global(Point::zero(), wid);
            wnd.set_ime_cursor_area(&rect);
          }
        }
      });

    @FocusScope {
      skip_host: true,
      @ $text {
        margin: pipe!($caret.layout_size()).map(|v|EdgeInsets::only_right(v.width)),
        on_disposed: move |_| u.unsubscribe(),
        on_focus_in: move |e| { e.window().set_ime_allowed(true); },
        on_focus_out: move|e| { e.window().set_ime_allowed(false); },
        on_chars: {
          let mut writer = writer.clone();
          move |c| { writer.chars_handle(c); }
        },
        on_key_down: move |k| { writer.keys_handle(k); },
        on_ime_pre_edit: move|e| {
          $ime.write().process_pre_edit(e);
        },
        @ OnlySizedByParent {
          clamp: BoxClamp::EXPAND_BOTH,
          @ { selection }
        }
        @ IgnorePointer { @ { TextGlyphsPainter::<T>::default() } }
        @ OnlySizedByParent { @ { caret } }
      }
    }
  }
  .into_widget()
}

fn caret_widget<T: 'static>(
  selectable: impl StateWatcher<Value = TextSelection<T>>,
) -> Widget<'static> {
  fn_widget! {
    let cache = Provider::state_of::<TextGlyphsProvider<T>>(&BuildCtx::get())
      .expect("Text Caret: TextGlyphs not found")
      .clone_watcher();
    let mut anchor = Anchor::default();
    @IgnorePointer {
      anchor: pipe!({
        if let Some(pos) = $cache.glyphs().as_ref().map(|cache| cache.cursor($selectable.to)) {
          anchor = Anchor::from_point(pos)
        }
        anchor
      }),
      @TextClamp {
        rows: Some(1.),
        class: TEXT_CARET,
        @ Void {  }
      }
    }
  }
  .into_widget()
}

struct TextWriter<H, T> {
  host: H,
  selection: T,
}

impl<T: EditText + 'static, H, S> TextWriter<H, S>
where
  H: StateWriter<Value = T>,
  S: StateWriter<Value = TextSelection<T>>,
{
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
      self.insert(&chars);
      return true;
    }
    false
  }

  fn keys_handle(&mut self, event: &KeyboardEvent) -> bool {
    let mut deal = false;
    if event.with_command_key() {
      deal = self.edit_with_command(event);
    }
    if !deal {
      deal = self.edit_with_key(event);
    }
    deal
  }

  fn clone(&self) -> TextWriter<H, S> {
    TextWriter { host: self.host.clone_writer(), selection: self.selection.clone_writer() }
  }

  fn edit_with_command(&mut self, event: &KeyboardEvent) -> bool {
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
          self.insert(&txt);
        }
        true
      }
      PhysicalKey::Code(KeyCode::KeyX) => {
        let rg = self.selection();
        if !rg.is_empty() {
          let txt = self.substr(rg.clone()).to_string();
          self.delete_selection();
          let clipboard = AppCtx::clipboard();
          let _ = clipboard.borrow_mut().clear();
          let _ = clipboard.borrow_mut().write_text(&txt);
        }
        true
      }
      _ => false,
    }
  }

  fn edit_with_key(&mut self, key: &KeyboardEvent) -> bool {
    match key.key() {
      VirtualKey::Named(NamedKey::Backspace) => {
        let mut rg = self.selection();
        if rg.is_empty() {
          let len = self.measure_bytes(rg.start, -1);
          rg = Range { start: rg.start - len, end: rg.start };
        }
        self.delete(rg);
      }
      VirtualKey::Named(NamedKey::Delete) => {
        let mut rg = self.selection();
        if rg.is_empty() {
          let len = self.measure_bytes(rg.start, 1);
          rg = Range { start: rg.start, end: rg.start + len };
        }
        self.delete(rg);
      }
      _ => (),
    };
    true
  }

  fn measure_bytes(&self, start: usize, len: isize) -> usize {
    self.host.read().measure_bytes(start, len)
  }

  fn set_selection(&mut self, from: CaretPosition, to: CaretPosition) {
    self.selection.write().from = from;
    self.selection.write().to = to;
  }

  fn selection(&self) -> Range<usize> { self.selection.read().selection() }

  fn substr(&self, rg: Range<usize>) -> Substr { self.host.read().substr(rg) }

  fn insert(&mut self, chars: &str) -> usize {
    let rg = self.selection();
    let delete_rg = self.host.write().delete(rg);
    let len = self
      .host
      .write()
      .insert_str(delete_rg.start, chars);
    let pos = CaretPosition { cluster: len + delete_rg.start, position: None };
    self.selection.write().from = pos;
    self.selection.write().to = pos;
    len
  }

  fn delete_selection(&mut self) {
    let rg = self.selection();
    let delete_rg = self.host.write().delete(rg);
    self.selection.write().from = CaretPosition { cluster: delete_rg.start, position: None };
    self.selection.write().to = CaretPosition { cluster: delete_rg.start, position: None };
  }

  fn delete(&mut self, rg: Range<usize>) {
    let delete_rg = self.host.write().delete(rg);
    self.selection.write().from = CaretPosition { cluster: delete_rg.start, position: None };
    self.selection.write().to = CaretPosition { cluster: delete_rg.start, position: None };
  }
}

#[derive(Debug)]
struct PreEditState {
  position: usize,
  value: Option<String>,
}

pub struct ImeHandle<H, S> {
  writer: TextWriter<H, S>,
  pre_edit: Option<PreEditState>,
}

impl<T, H, S> ImeHandle<H, S>
where
  T: EditText + 'static,
  H: StateWriter<Value = T>,
  S: StateWriter<Value = TextSelection<T>>,
{
  fn new(writer: TextWriter<H, S>) -> Self { Self { writer, pre_edit: None } }

  fn is_in_pre_edit(&self) -> bool { self.pre_edit.is_some() }

  fn process_pre_edit(&mut self, e: &ImePreEditEvent) {
    match &e.pre_edit {
      ImePreEdit::Begin => {
        self.writer.delete_selection();
        self.pre_edit = Some(PreEditState { position: self.writer.selection().start, value: None });
      }
      ImePreEdit::PreEdit { value, cursor } => {
        let Some(PreEditState { position, value: edit_value }) = self.pre_edit.as_mut() else {
          return;
        };
        if let Some(txt) = edit_value {
          self
            .writer
            .delete(Range { start: *position, end: *position + txt.len() });
        }
        let len = self.writer.insert(value);
        let pos = if len == value.len() {
          *edit_value = Some(value.clone());
          CaretPosition {
            cluster: *position + cursor.map(|(start, _)| start).unwrap_or(0),
            position: None,
          }
        } else {
          *edit_value = Some(
            self
              .writer
              .substr(Range { start: *position, end: *position + len })
              .to_string(),
          );
          CaretPosition { cluster: *position + len, position: None }
        };
        self.writer.set_selection(pos, pos);
      }
      ImePreEdit::End => {
        if let Some(PreEditState { value: Some(txt), position, .. }) = self.pre_edit.take() {
          self
            .writer
            .delete(Range { start: position, end: position + txt.len() });
        }
      }
    }
  }
}
