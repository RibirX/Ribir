use crate::prelude::*;

/// A widget that represents an empty node in the widget tree.
///
/// This widget is used when you need a placeholder widget that doesn't render
/// anything and doesn't accept children. It's useful for conditional rendering
/// or as a neutral widget in compositions.
///
/// # Example
///
/// ```rust no_run
/// use ribir::prelude::*;
///
/// fn_widget! {
///   @Void {}
/// };
/// ```
#[declare]
pub struct Void;

impl Render for Void {
  fn perform_layout(&self, clamp: BoxClamp, _: &mut LayoutCtx) -> Size { clamp.min }

  fn paint(&self, _: &mut PaintingCtx) {}
}
