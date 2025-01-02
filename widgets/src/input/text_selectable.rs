use ribir_core::prelude::*;

use super::text_high_light::high_light_widget;
use crate::prelude::*;

pub struct TextSelectChanged {
  pub text: CowArc<str>,
  pub caret: CaretState,
}

pub type TextSelectChangedEvent = CustomEvent<TextSelectChanged>;

/// A Widget that extends [`Text`] to support text selection.
///
/// # Example
/// ```no_run
/// use ribir::prelude::*;
/// fn_widget! {
///   @TextSelectable {
///     @{ "Hello world" }
///   }
/// }
/// App::run(w);
/// ```
#[derive(Declare)]
pub struct TextSelectable {
  #[declare(default)]
  caret: CaretState,
}

impl TextSelectable {
  fn notify_changed(&self, track_id: TrackId, text: CowArc<str>, wnd: &Window) {
    if let Some(id) = track_id.get() {
      wnd.bubble_custom_event(id, TextSelectChanged { text, caret: self.caret });
    }
  }
}

#[derive(Template)]
pub enum TextSelectableTml {
  Text(Stateful<Text>),
  TextWidget(FatObj<State<Text>>),
  Raw(TextInit),
}

impl<'c> ComposeChild<'c> for TextSelectable {
  type Child = TextSelectableTml;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    fn_widget! {
      let text = match child {
        TextSelectableTml::Text(text) => {
          text
        }
        TextSelectableTml::Raw(text) => {
          @Text { text }.clone_writer()
        }
        TextSelectableTml::TextWidget(text) => {
          text.clone_writer()
        }
      };
      let mut stack = @Stack {};
      let high_light_rect = @ {
        high_light_widget(this.map_writer(|v| PartData::from_ref(&v.caret)), text.clone_watcher())
      };
      @ $stack {
        tab_index: -1_i16,
        on_key_down: {
          let caret_writer = this.map_writer(|v| PartData::from_ref_mut(&mut v.caret));
          move |e| {
            let changed = TextOnlySelectable {
              text: $text.text.clone(),
              caret: caret_writer.clone_writer(),
            }.keys_select_handle(&$text, e);
            if changed {
              $this.notify_changed($stack.track_id(), $text.text.clone(), &e.window());
            }
          }
        },

        @ { high_light_rect }
        @ SelectRegion {
          on_custom_event: {
            let caret_writer = this.map_writer(|v| PartData::from_ref_mut(&mut v.caret));
            move |e: &mut SelectRegionEvent| {
              let changed = TextOnlySelectable {
                text: $text.text.clone(),
                caret:caret_writer.clone_writer(),
              }.select_region_handle(&$text, e);
              if changed {
                $this.notify_changed($stack.track_id(), $text.text.clone(), &e.window());
              }
            }
          },
          @ { text.clone_writer() }
        }
      }
    }
    .into_widget()
  }
}

struct TextOnlySelectable<C> {
  text: CowArc<str>,
  caret: C,
}

impl<C: StateWriter<Value = CaretState>> SelectableText for TextOnlySelectable<C> {
  fn caret(&self) -> CaretState { *self.caret.read() }

  fn set_caret(&mut self, caret: CaretState) { *self.caret.write() = caret; }

  fn text(&self) -> CowArc<str> { self.text.clone() }
}
