use std::ops::Range;

use ribir_core::prelude::*;

use super::{
  CaretPosition,
  text_glyphs::{TextGlyphs, TextGlyphsPainter},
  text_selection::TextSelection,
};

/// A Widget that extends [`Text`] to support text selection.
///
/// # Example
/// ```no_run
/// use ribir::prelude::*;
/// let w = fn_widget! {
///   @TextSelectable {
///     text: "Hello world"
///   }
/// };
/// App::run(w);
/// ```
#[derive(Declare)]
pub struct TextSelectable {
  text: TextGlyphs<CowArc<str>>,
  #[declare(skip)]
  selection: TextSelection<CowArc<str>>,
}

impl TextSelectable {
  pub fn set_selection(&mut self, rg: Range<usize>) {
    self.selection.from = CaretPosition { cluster: rg.start, position: None };
    self.selection.to = CaretPosition { cluster: rg.start, position: None };
  }
}

impl Compose for TextSelectable {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    fn_widget! {
      let selection = FatObj::new(part_writer!(&mut this.selection));
      let text = part_writer!(&mut this.text);
      @ $text {
        @ $selection{ tab_index: -1_i16 }
        @ IgnorePointer { @ { TextGlyphsPainter::<CowArc<str>>::default() } }
      }
    }
    .into_widget()
  }
}
