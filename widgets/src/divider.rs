use ribir_core::prelude::*;

use crate::prelude::*;

/// Divider is a thin horizontal or vertical line, with indent on either side.
///
/// # example
/// ```
/// # use ribir_core::prelude::*;
/// # use ribir_widgets::prelude::*;
///
/// // use default Divider default settings
/// let widget = fn_widget! {
///   @Column {
///     @SizedBox { size: Size::new(10., 0.) }
///     @Divider { }
///     @SizedBox { size: Size::new(10., 0.) }
///   }
/// };
///
/// // with indent settings
/// let widget = fn_widget! {
///   @Column {
///     @SizedBox { size: Size::new(10., 0.) }
///     @Divider {
///       indent: DividerIndent::Start,
///     }
///     @SizedBox { size: Size::new(10., 0.) }
///   }
/// };
/// ```

#[derive(Default, Clone, Copy)]
pub enum DividerIndent {
  /// Does not indent the divider.
  #[default]
  None,
  /// Indents the divider with equal indent on the leading sides.
  Start,
  /// Indents the divider with equal indent on the trailing sides.
  End,
  /// Indents the divider with equal indent on the both sides.
  Both,
}

class_names! {
  #[doc = "class name for horizontal divider"]
  HORIZONTAL_DIVIDER,
  #[doc = "class name for horizontal divider with inset at leading"]
  HORIZONTAL_DIVIDER_INDENT_START,
  #[doc = "class name for horizontal divider with inset at trailing"]
  HORIZONTAL_DIVIDER_INDENT_END,
  #[doc = "class name for horizontal divider with inset at both sides"]
  HORIZONTAL_DIVIDER_INDENT_BOTH,
  #[doc = "class name for vertical divider"]
  VERTICAL_DIVIDER,
  #[doc = "class name for vertical divider with inset at leading"]
  VERTICAL_DIVIDER_INDENT_START,
  #[doc = "class name for vertical divider with inset at trailing"]
  VERTICAL_DIVIDER_INDENT_END,
  #[doc = "class name for vertical divider with inset at both sides"]
  VERTICAL_DIVIDER_INDENT_BOTH,
}

#[derive(Default, Declare)]
pub struct Divider {
  #[declare(default)]
  indent: DividerIndent,

  #[declare(default = Direction::Horizontal)]
  direction: Direction,
}

impl Compose for Divider {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    fn_widget! {
      @Void {
        class: pipe!(($this.indent, $this.direction)).map(|(s,d)| {
          match (s, d) {
            (DividerIndent::None, Direction::Horizontal) => HORIZONTAL_DIVIDER,
            (DividerIndent::Start, Direction::Horizontal) => HORIZONTAL_DIVIDER_INDENT_START,
            (DividerIndent::End, Direction::Horizontal) => HORIZONTAL_DIVIDER_INDENT_END,
            (DividerIndent::Both, Direction::Horizontal) => HORIZONTAL_DIVIDER_INDENT_BOTH,
            (DividerIndent::None, Direction::Vertical) => VERTICAL_DIVIDER,
            (DividerIndent::Start, Direction::Vertical) => VERTICAL_DIVIDER_INDENT_START,
            (DividerIndent::End, Direction::Vertical) => VERTICAL_DIVIDER_INDENT_END,
            (DividerIndent::Both, Direction::Vertical) => VERTICAL_DIVIDER_INDENT_BOTH,
          }
        }),

      }
    }
    .into_widget()
  }
}
