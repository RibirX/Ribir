use ribir_core::prelude::*;

/// Label is a a brief description given for purposes of widget that like
/// `CheckBox` and `Input`.
///
/// There are difference between `Label` and `Text`. `Text` design to use as an
/// individual widget and user can specify its style, but `Label` only can used
/// with its purpose widget, and its style is detected by its purpose widget not
/// user.
#[derive(Declare)]
pub struct Label {
  #[declare(convert=into)]
  pub desc: ArcStr,
  /// the position to place the label.
  #[declare(default)]
  pub position: Position,
}

/// Describe label position before or after purpose widget.`
#[derive(Default)]
pub enum Position {
  Before,
  #[default]
  After,
}
