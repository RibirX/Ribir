use std::ops::Range;

use ribir_core::prelude::*;

use super::{
  CaretPosition,
  edit_text::EditText,
  text_selectable::{Selection, TextSelectable},
};
use crate::{input::text_glyphs::VisualGlyphsHelper, prelude::*};

class_names! {
  #[doc = "Class name for the text caret"]
  TEXT_CARET,
}

#[derive(Declare, Default)]
pub struct BasicEditor<T: 'static> {
  host: TextSelectable<T>,
  pre_edit: Option<PreEditState>,
}

impl<T: Default + VisualText + EditText + Clone + 'static> Compose for BasicEditor<T> {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    fn_widget! {
      let mut text = FatObj::new(part_writer!(&mut this.host));

      let this2 = this.clone_writer();
      let caret = pipe! {
        let this = this2.clone_writer();
        $text.is_focused().then(|| fn_widget! {Self::caret_widget(this)})
      };
      let mut caret = FatObj::new(caret);

      @Stack {
        fit: StackFit::Passthrough,
        @ $text {
          margin: pipe!($caret.layout_size()).map(|v|EdgeInsets::only_right(v.width)),
          on_focus_in: move |e| { e.window().set_ime_allowed(true); },
          on_focus_out: move|e| { e.window().set_ime_allowed(false); },
          on_chars: move |e| {
            let mut this = $this.write();
            if !this.chars_handle(e) {
              this.forget_modifies();
            }
          },
          on_key_down: move |k| {
            let mut this = $this.write();
            if !this.keys_handle(k) {
              this.forget_modifies();
            }
          },
          on_ime_pre_edit: move|e| { $this.write().process_pre_edit(e);},
        }
        @IgnorePointer {
          @UnconstrainedBox {
            dir: UnconstrainedDir::Both,
            @OnlySizedByParent { @ { caret } }
          }
        }
      }
    }
    .into_widget()
  }
}

impl<T: EditText + 'static> BasicEditor<T> {
  fn caret_widget(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    fn_widget! {
      let mut caret = @TextClamp {
        rows: Some(1.),
        class: TEXT_CARET,
        on_performed_layout: move |e| {
          let caret_size = e.box_size().unwrap();
          if !$this.is_in_pre_edit() {
            if let Some(mut scrollable) = Provider::write_of::<ScrollableWidget>(e) {
              let wnd = e.window();
              let lt = scrollable.map_to_content(Point::zero(), e.current_target(), &wnd).unwrap();
              scrollable.visible_content_box(Rect::new(lt, caret_size), Anchor::default());
            }
          }
          let pos = e.map_to_global(Point::zero());
          e.window().set_ime_cursor_area(&Rect::new(pos, caret_size));
        },
        @ { Void }
      };
      let wnd = BuildCtx::get().window();
      let u = watch!($this;).subscribe(move |_| {
        wnd.once_layout_ready(move || {
          $caret.write().anchor = Anchor::from_point($this.caret_pos())
        })
      });
      caret.on_disposed(move |_| u.unsubscribe());
      caret
    }
    .into_widget()
  }
  fn caret_pos(&self) -> Point {
    self
      .glyphs()
      .map(|g| g.cursor(self.selection.to))
      .unwrap_or_default()
  }

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
          return true;
        }
      }
      PhysicalKey::Code(KeyCode::KeyX) => {
        let rg = self.cluster_rg();
        if !rg.is_empty() {
          let txt = self.substr(rg).to_string();
          self.del_sel();
          let clipboard = AppCtx::clipboard();
          let _ = clipboard.borrow_mut().clear();
          let _ = clipboard.borrow_mut().write_text(&txt);
          return true;
        }
      }
      _ => {}
    };
    false
  }

  fn edit_with_key(&mut self, key: &KeyboardEvent) -> bool {
    match key.key() {
      VirtualKey::Named(NamedKey::Backspace) => {
        let mut rg = self.cluster_rg();
        if rg.is_empty() {
          let len = self.measure_bytes(rg.start, -1);
          rg = Range { start: rg.start - len, end: rg.start };
        }
        !self.delete(rg).is_empty()
      }
      VirtualKey::Named(NamedKey::Delete) => {
        let mut rg = self.cluster_rg();
        if rg.is_empty() {
          let len = self.measure_bytes(rg.start, 1);
          rg = Range { start: rg.start, end: rg.start + len };
        }
        !self.delete(rg).is_empty()
      }
      _ => false,
    }
  }

  fn insert(&mut self, chars: &str) -> usize {
    let del_rg = self.del_sel();
    let len = self.insert_str(del_rg.start, chars);
    let pos = CaretPosition { cluster: len + del_rg.start, position: None };
    self.host.selection = Selection::splat(pos);
    len
  }

  fn del_sel(&mut self) -> Range<usize> { self.delete(self.cluster_rg()) }

  fn delete(&mut self, rg: Range<usize>) -> Range<usize> {
    let del_rg = self.del_rg_str(rg);
    self.host.selection = Selection::splat(CaretPosition { cluster: del_rg.start, position: None });
    del_rg
  }

  fn is_in_pre_edit(&self) -> bool { self.pre_edit.is_some() }

  fn process_pre_edit(&mut self, e: &ImePreEditEvent) {
    match &e.pre_edit {
      ImePreEdit::Begin => {
        self.del_sel();
        self.pre_edit = Some(PreEditState { position: self.cluster_rg().start, value: None });
      }
      ImePreEdit::PreEdit { value, cursor } => {
        let Some(pre_edit) = self.pre_edit.as_mut() else {
          return;
        };
        // Safety: it is safe to modify all the fields to avoid conflicts with the
        // borrow checker.
        let PreEditState { position: pos, value: editing } =
          unsafe { &mut *(pre_edit as *mut PreEditState) };
        if let Some(txt) = editing {
          self.delete(Range { start: *pos, end: *pos + txt.len() });
        }
        let len = self.insert(value);
        let pos = if len == value.len() {
          *editing = Some(value.clone());
          CaretPosition {
            cluster: *pos + cursor.map(|(start, _)| start).unwrap_or(0),
            position: None,
          }
        } else {
          *editing = Some(
            self
              .substr(Range { start: *pos, end: *pos + len })
              .to_string(),
          );
          CaretPosition { cluster: *pos + len, position: None }
        };
        self.host.selection = Selection::splat(pos);
      }
      ImePreEdit::End => {
        if let Some(PreEditState { value: Some(txt), position, .. }) = self.pre_edit.take() {
          self.delete(Range { start: position, end: position + txt.len() });
        }
      }
    }
  }
}

#[derive(Debug)]
struct PreEditState {
  position: usize,
  value: Option<String>,
}

impl<T> std::ops::Deref for BasicEditor<T> {
  type Target = TextSelectable<T>;

  fn deref(&self) -> &Self::Target { &self.host }
}

impl<T> std::ops::DerefMut for BasicEditor<T> {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.host }
}
