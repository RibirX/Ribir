use std::ops::Range;

use ribir_core::prelude::*;

use super::{
  CaretPosition, OnlySizedByParent, PointerSelectData, PointerSelectEvent, PointerSelectRegion,
  Stack, VisualText,
  edit_text::BaseText,
  text_glyphs::{TextGlyphs, TextGlyphsProvider, VisualGlyphsHelper},
};

class_names! {
  #[doc = "Class name for the text high light rect"]
  TEXT_HIGH_LIGHT,
}

#[derive(Default, Declare)]
pub struct TextSelection<T>
where
  T: 'static,
{
  pub from: CaretPosition,
  pub to: CaretPosition,
  #[declare(default)]
  marker: PhantomData<T>,
}

impl<T: VisualText + Clone + 'static> Compose for TextSelection<T> {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    fn_widget! {
      let text_glyphs = Provider::state_of::<TextGlyphsProvider<T>>(BuildCtx::get())
        .expect("TextSelection: TextGlyphs not found")
        .clone_watcher();
      @OnlySizedByParent {
        @ PointerSelectRegion {
          on_custom_concrete_event: {
            move |e: &mut PointerSelectEvent| {
              match e.data() {
                PointerSelectData::Move{ from, to } |
                PointerSelectData::End { from, to } => {
                  let glyphs = $text_glyphs.glyphs().cloned();
                  if let Some(glyphs) = glyphs {
                    $this.write().from = glyphs.caret_position_from_pos(*from);
                    $this.write().to = glyphs.caret_position_from_pos(*to);
                  }
                },
                _ => {}
              }
            }
          },
          on_key_down: move |e| {
            let selection = $this.select_with_key(&$text_glyphs, e);
            if let Some((from, to)) = selection {
              $this.write().from = from;
              $this.write().to = to;
            }
          },
          on_pointer_down: move |e| {
            let glyphs = $text_glyphs.glyphs().cloned();
            if let Some(glyphs) = glyphs {
              let caret = glyphs.caret_position_from_pos(e.position());
              if e.with_shift_key() {
                $this.write().to = CaretPosition{ cluster: caret.cluster, position: None };
              } else {
                $this.write().from = CaretPosition{ cluster: caret.cluster, position: None };
                $this.write().to = CaretPosition{ cluster: caret.cluster, position: None };
              }
            }
          },
          on_double_tap: move |e| {
            let caret = $text_glyphs
              .glyphs()
              .map(|glyphs| glyphs.caret_position_from_pos(e.position()));
            if let Some(caret) = caret {
              let rg = $text_glyphs.text().select_token(caret.cluster);
              $this.write().from = CaretPosition{ cluster: rg.start, position: None };
              $this.write().to = CaretPosition{ cluster: rg.end, position: None };
            }
          },
          @Stack {
            clamp: BoxClamp::EXPAND_BOTH,
            @ { pipe!(
                $text_glyphs.glyphs()
                  .map(|glyphs| glyphs.select_range(&$this.selection()))
                  .unwrap_or_default()
                ).map(|rcs| {
                  rcs.into_iter().map(move |rc| {
                    @Container {
                      class: TEXT_HIGH_LIGHT,
                      anchor: Anchor::from_point(rc.origin),
                      size: rc.size,
                    }.into_widget()
                  })
                })
              }
          }
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

impl<T: BaseText> TextSelection<T> {
  pub fn selection(&self) -> Range<usize> {
    let start = self.from.cluster.min(self.to.cluster);
    let end = self.from.cluster.max(self.to.cluster);
    Range { start, end }
  }

  fn select_with_key(
    &self, text: &TextGlyphs<T>, event: &KeyboardEvent,
  ) -> Option<(CaretPosition, CaretPosition)> {
    if let Ok(selection) = self.deal_with_command(text, event) {
      return selection;
    }
    let new_caret_position = match event.key() {
      VirtualKey::Named(NamedKey::ArrowLeft) => {
        if is_move_by_word(event) {
          let mut rg = text.text().select_token(self.to.cluster);
          if rg.start == self.to.cluster && self.to.cluster > 1 {
            rg = text.text().select_token(self.to.cluster - 1);
          }
          Some(CaretPosition { cluster: rg.start, position: None })
        } else if event.with_command_key() {
          text
            .glyphs()
            .map(|glyphs| glyphs.line_begin(self.to))
        } else {
          text.glyphs().map(|glyphs| glyphs.prev(self.to))
        }
      }
      VirtualKey::Named(NamedKey::ArrowRight) => {
        if is_move_by_word(event) {
          let mut rg = text.text().select_token(self.to.cluster);
          if rg.end == self.to.cluster {
            rg = text.text().select_token(self.to.cluster + 1);
          }
          Some(CaretPosition { cluster: rg.end, position: None })
        } else if event.with_command_key() {
          text
            .glyphs()
            .map(|glyphs| glyphs.line_end(self.to))
        } else {
          text.glyphs().map(|glyphs| glyphs.next(self.to))
        }
      }
      VirtualKey::Named(NamedKey::ArrowUp) => text.glyphs().map(|glyph| glyph.up(self.to)),
      VirtualKey::Named(NamedKey::ArrowDown) => text.glyphs().map(|glyph| glyph.down(self.to)),
      VirtualKey::Named(NamedKey::Home) => text
        .glyphs()
        .map(|glyph| glyph.line_begin(self.to)),
      VirtualKey::Named(NamedKey::End) => text.glyphs().map(|glyph| glyph.line_end(self.to)),
      _ => None,
    };

    if let Some(caret) = new_caret_position {
      if event.with_shift_key() {
        return Some((self.from, caret));
      } else {
        return Some((caret, caret));
      };
    }
    None
  }

  fn deal_with_command(
    &self, glyphs: &TextGlyphs<T>, event: &KeyboardEvent,
  ) -> Result<Option<(CaretPosition, CaretPosition)>, ()> {
    if !event.with_command_key() {
      return Err(());
    }
    match event.key_code() {
      PhysicalKey::Code(KeyCode::KeyC) => {
        let rg = self.selection();
        let text = glyphs.text().substr(rg);
        if !text.is_empty() {
          let clipboard = AppCtx::clipboard();
          let _ = clipboard.borrow_mut().clear();
          let _ = clipboard.borrow_mut().write_text(&text);
        }
        Ok(None)
      }
      PhysicalKey::Code(KeyCode::KeyA) => {
        let len = glyphs.text().len();
        if len > 0 {
          Ok(Some((CaretPosition { cluster: 0, position: None }, CaretPosition {
            cluster: len,
            position: None,
          })))
        } else {
          Ok(None)
        }
      }
      _ => Err(()),
    }
  }
}
