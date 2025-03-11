use crate::prelude::*;

/// A widget that allows access to the layout result of its child.
///
/// ## Caution: Avoid Dependency on Layout Results for View Updates
///
/// Layout operations occur frequently, so relying on layout results to update
/// other widgets should be done with extreme caution. Doing so can easily lead
/// to performance issues, such as double layouts or infinite layout loops
/// within a single frame.
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
/// **Solution**:
/// - Use `OnlySizedByParent` to wrap `B` if `B` does not affect the size of its
///   parent. This prevents unnecessary dirty propagation to the parent.
/// - Ensure that there is no circular dependency in your logic.
///
/// ## Avoid Using Layout Results to Create Pipe Widgets
///
/// Layout result changes are only notified within a `Data` scope. This means
/// that if you use a layout result as the upstream of a pipe widget, the pipe
/// widget will not update when the layout result of the widget changes.

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
    fn_widget! {
      FatObj::new(child).on_performed_layout(move |e| {
        let new_rect = e.box_rect().unwrap();
        if $this.rect != new_rect {
          $this.silent().rect = new_rect;
        }
      })
    }
    .into_widget()
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
      let second_box = @MockBox { size: pipe!($first_box.layout_size()) };
      @MockMulti {
        @ { [first_box, second_box ] }
      }
    }),
    LayoutCase::default().with_size(Size::new(200., 200.)),
    LayoutCase::new(&[0, 0]).with_size(Size::new(100., 200.)),
    LayoutCase::new(&[0, 1]).with_size(Size::new(100., 200.))
  );
}
