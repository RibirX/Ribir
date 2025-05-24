use std::ops::Range;

use ribir_core::prelude::*;

use super::{Stack, VisualText, text_glyphs::*, *};

class_names! {
  #[doc = "The name of the class for the text selection highlight rectangles"]
  TEXT_SELECTION,
}

/// This widget is similar to [`Text`](ribir_core::prelude::Text) but with added
/// selectable functionality.
///
/// The `text` field is a generic type and must implement the [`VisualText`]
/// trait. When declaring this widget, you must specify the type of visual text
/// to be used explicitly.
///
/// # Example
///
/// ```no_run
/// use ribir::prelude::*;
///
/// let w = fn_widget! {
///   @TextSelectable::<CowArc<str>> {
///     text: "Hello world"
///   }
/// };
/// App::run(w);
/// ```
#[derive(Default, Declare)]
pub struct TextSelectable<T>
where
  T: 'static,
{
  #[declare(skip)]
  pub selection: Selection,
  #[declare(custom)]
  pub text: TextGlyphs<T>,
}

#[derive(Copy, Clone, Default)]
pub struct Selection {
  pub from: CaretPosition,
  pub to: CaretPosition,
}

impl<T> TextSelectableDeclarer<T> {
  pub fn text<K: ?Sized>(&mut self, text: impl RInto<PipeValue<T>, K>) -> &mut Self {
    let text = text.r_into().map(TextGlyphs::new);
    self.text = Some(text);
    self
  }
}

impl<T: VisualText + Clone + 'static> Compose for TextSelectable<T> {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    fn_widget! {
      let selection = part_writer!(&mut this.selection);

      @PointerSelectRegion {
        on_custom_concrete_event: {
          move |e: &mut PointerSelectEvent| {
            match e.data() {
              PointerSelectData::Move{ from, to } |
              PointerSelectData::End { from, to } => {
                let new_sel = $this.glyphs().map(|g| {
                  Selection {
                    from: g.caret_position_from_pos(*from),
                    to: g.caret_position_from_pos(*to),
                  }
                });
                if let Some(new_sel) = new_sel {
                  *$selection.write() = new_sel;
                }
              },
              _ => {}
            }
          }
        },
        on_key_down: move |e| {
          let new_sel = $this.select_with_key(e);
          if let Some(new_sel) = new_sel {
            *$selection.write() = new_sel;
          }
        },
        on_pointer_down: move |e| {
          let caret = $this.glyphs().map(|g| g.caret_position_from_pos(e.position()));
          if let Some(caret) = caret {
            let mut selection = $selection.write();
            if e.with_shift_key() {
              selection.to = CaretPosition{ cluster: caret.cluster, position: None };
            } else {
              selection.from = CaretPosition{ cluster: caret.cluster, position: None };
              selection.to = CaretPosition{ cluster: caret.cluster, position: None };
            }
          }
        },
        on_double_tap: move |e| {
          let caret = $this
            .glyphs()
            .map(|glyphs| glyphs.caret_position_from_pos(e.position()));
          if let Some(caret) = caret {
            let rg = $this.text().select_token(caret.cluster);
            $selection.write().from = CaretPosition{ cluster: rg.start, position: None };
            $selection.write().to = CaretPosition{ cluster: rg.end, position: None };
          }
        },
        @Stack {
          @NoAffectedParentSize {
            @Stack {
              @pipe! {
                let this = $this;
                let rcs = this.glyphs()
                  .map(|glyphs| glyphs.select_range(&$selection.cluster_rg()))
                  .unwrap_or_default();
                rcs.into_iter().map(move |rc| {
                  @Container {
                    class: TEXT_SELECTION,
                    anchor: Anchor::from_point(rc.origin),
                    size: rc.size,
                  }
                })
              }
            }
          }
          @part_writer!(&mut this.text)
        }
      }

    }
    .into_widget()
  }
}

fn is_move_by_word(event: &KeyboardEvent) -> bool {
  #[cfg(target_os = "macos")]
  return event.with_alt_key();
  #[cfg(not(target_os = "macos"))]
  return event.with_ctrl_key();
}

impl<T> TextSelectable<T> {
  pub fn cluster_rg(&self) -> Range<usize> { self.selection.cluster_rg() }
}

impl Selection {
  pub fn cluster_rg(&self) -> Range<usize> {
    let start = self.from.cluster.min(self.to.cluster);
    let end = self.from.cluster.max(self.to.cluster);
    Range { start, end }
  }
}

impl<T: BaseText> TextSelectable<T> {
  fn select_with_key(&self, event: &KeyboardEvent) -> Option<Selection> {
    if let Ok(selection) = self.deal_with_command(event) {
      return selection;
    }
    let glyphs = self.glyphs()?;
    let cur_sel = &self.selection;
    let text = &self.text;
    let new_caret = match event.key() {
      VirtualKey::Named(NamedKey::ArrowLeft) => {
        if is_move_by_word(event) {
          let mut rg = text.text().select_token(cur_sel.to.cluster);
          if rg.start == cur_sel.to.cluster && cur_sel.to.cluster > 1 {
            rg = text.text().select_token(cur_sel.to.cluster - 1);
          }
          CaretPosition { cluster: rg.start, position: None }
        } else if event.with_command_key() {
          glyphs.line_begin(cur_sel.to)
        } else {
          glyphs.prev(cur_sel.to)
        }
      }
      VirtualKey::Named(NamedKey::ArrowRight) => {
        if is_move_by_word(event) {
          let mut rg = text.text().select_token(cur_sel.to.cluster);
          if rg.end == cur_sel.to.cluster {
            rg = text.text().select_token(cur_sel.to.cluster + 1);
          }
          CaretPosition { cluster: rg.end, position: None }
        } else if event.with_command_key() {
          glyphs.line_end(cur_sel.to)
        } else {
          glyphs.next(cur_sel.to)
        }
      }
      VirtualKey::Named(NamedKey::ArrowUp) => glyphs.up(cur_sel.to),
      VirtualKey::Named(NamedKey::ArrowDown) => glyphs.down(cur_sel.to),
      VirtualKey::Named(NamedKey::Home) => glyphs.line_begin(cur_sel.to),
      VirtualKey::Named(NamedKey::End) => glyphs.line_end(cur_sel.to),
      _ => return None,
    };

    let from = if event.with_shift_key() { cur_sel.from } else { new_caret };
    Some(Selection { from, to: new_caret })
  }

  fn deal_with_command(&self, event: &KeyboardEvent) -> Result<Option<Selection>, ()> {
    if !event.with_command_key() {
      return Err(());
    }
    let text = self.text.text();
    match event.key_code() {
      PhysicalKey::Code(KeyCode::KeyC) => {
        let rg = self.cluster_rg();
        let text = text.substr(rg);
        if !text.is_empty() {
          let clipboard = AppCtx::clipboard();
          let _ = clipboard.borrow_mut().clear();
          let _ = clipboard.borrow_mut().write_text(&text);
        }
        Ok(None)
      }
      PhysicalKey::Code(KeyCode::KeyA) => {
        if text.len() > 0 {
          let selection = Selection {
            from: CaretPosition { cluster: 0, position: None },
            to: CaretPosition { cluster: text.len(), position: None },
          };
          Ok(Some(selection))
        } else {
          Ok(None)
        }
      }
      _ => Err(()),
    }
  }
}

impl Selection {
  pub fn splat(pos: CaretPosition) -> Selection { Selection { from: pos, to: pos } }
}

impl<T> std::ops::Deref for TextSelectable<T> {
  type Target = TextGlyphs<T>;

  fn deref(&self) -> &Self::Target { &self.text }
}

impl<T> std::ops::DerefMut for TextSelectable<T> {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.text }
}
