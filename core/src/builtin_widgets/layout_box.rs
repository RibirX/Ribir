use crate::prelude::*;

/// A widget that exposes the layout result of its child.
///
/// This built-in `FatObj` field provides helper methods such as
/// `layout_rect()`, `layout_size()`, `layout_pos()`, `layout_left()`,
/// `layout_top()`, `layout_width()`, `layout_height()` to access the child's
/// layout information.
///
/// # Example, the text will show the width of the parent container.
///
/// ```rust
/// use ribir::prelude::*;
///
/// fn_widget! {
///   let mut container = @Container { size: Size::new(100., 100.) };
///   @(container) {
///     @Text {
///       text: $read(container.layout_width()).to_string()
///     }
///   }
/// };
/// ```
///
/// ## Caution: Avoid depending on layout results for view updates
///
/// Layout runs frequently. Using layout results to trigger updates in other
/// widgets can easily cause performance issues, including double layouts or
/// infinite layout loops within the same frame.
///
/// ### Example Scenario:
///
/// Suppose Widget A depends on the layout size of Widget B.
///
/// 1. Widget A completes its layout and notifies Widget B.
/// 2. Since Widget B depends on Widget A's layout result, it becomes "dirty"
///    and requests a layout.
/// 3. When Widget B becomes dirty, it may also mark all its ancestors as dirty.
/// 4. Since Widget A is a descendant of one of Widget B's ancestors, Widget A
///    may be laid out again when that ancestor performs its layout.
/// 5. If the layout result of Widget A in step 4 differs from step 1, the
///    process repeats from step 1.
///
/// In step 4, this can lead to a double layout within the same frame. In step
/// 5, it may result in an infinite layout loop.
///
/// **Mitigations**
///
/// - Wrap widgets with `OnlySizedByParent` if they do not affect their parent's
///   size to prevent unnecessary dirty propagation.
/// - Avoid circular dependencies between widgets' layout and state updates.
///
/// ## Avoid using layout results as pipe widget sources
///
/// Layout result changes are notified only inside a `Data` scope. If you use
/// a layout result as the upstream source for a pipe widget, that pipe may not
/// update when the layout result changes.

#[derive(Default)]
pub struct LayoutBox {
  /// the rect box of its child and the coordinate is relative to its parent.
  rect: Rect,
}

impl Declare for LayoutBox {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl<'c> ComposeChild<'c> for LayoutBox {
  type Child = Widget<'c>;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    let mut w = FatObj::new(child);
    w.on_performed_layout(move |e| {
      let new_rect = e.box_rect().unwrap();
      let mut this = this.silent();
      if this.rect != new_rect {
        this.rect = new_rect;
      }
    });

    w.into_widget()
  }
}

impl LayoutBox {
  /// return the rect after layout of the widget
  #[inline]
  pub fn layout_rect(&self) -> Rect { self.rect }

  /// return the position relative to parent after layout of the widget
  #[inline]
  pub fn layout_pos(&self) -> Point { self.rect.origin }

  /// return the size after layout of the widget
  #[inline]
  pub fn layout_size(&self) -> Size { self.rect.size }

  /// return the left position relative parent after layout of the widget
  #[inline]
  pub fn layout_left(&self) -> f32 { self.rect.min_x() }

  /// return the top position relative parent after layout of the widget
  #[inline]
  pub fn layout_top(&self) -> f32 { self.rect.min_y() }

  /// return the width after layout of the widget
  #[inline]
  pub fn layout_width(&self) -> f32 { self.rect.width() }

  /// return the height after layout of the widget
  #[inline]
  pub fn layout_height(&self) -> f32 { self.rect.height() }
}

#[cfg(test)]
mod tests {
  use ribir_dev_helper::*;

  use super::*;
  use crate::test_helper::*;

  widget_layout_test!(
    smoke,
    WidgetTester::new(fn_widget! {
      let mut first_box = @MockBox { size: Size::new(100., 200.) };
      let second_box = @MockBox { size: pipe!(*$read(first_box.layout_size())) };
      @MockMulti {
        @ { [first_box, second_box ] }
      }
    }),
    LayoutCase::default().with_size(Size::new(200., 200.)),
    LayoutCase::new(&[0, 0]).with_size(Size::new(100., 200.)),
    LayoutCase::new(&[0, 1]).with_size(Size::new(100., 200.))
  );
}
