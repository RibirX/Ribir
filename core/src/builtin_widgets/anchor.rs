//! Anchor widget for positioning children relative to their parent.
//!
//! This module re-exports anchor types from `widget_tree::layout_info`.

// Re-export anchor types from layout_info
pub use crate::widget_tree::{Anchor, AnchorX, AnchorY};
use crate::{prelude::*, wrap_render::WrapRender};

/// A widget that positions its child.
///
/// Use `x` and `y` to position the child. The value can be a fixed pixel value,
/// a percentage of the parent size, or a calculated value.
///
/// # Example, with fixed pixel value
///
/// ```rust
/// use ribir::prelude::*;
///
/// fn_widget! {
///   @Text {
///     text: "Hello World!",
///     x: 10.,
///     y: AnchorY::percent(0.5),
///   }
/// };
/// ```
///
/// # Example, align relactive to parent
///
/// ``` rust
/// use ribir::prelude::*;
///
/// fn_widget! {
///   @Text {
///     text: "Hello World!",
///     x: AnchorX::at_center(),
///     y: AnchorY::at_center().offset(-10.), // move 10 pixel up above center
///   }
/// };
/// ```
impl Declare for Anchor {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl_compose_child_for_wrap_render!(Anchor);

impl WrapRender for Anchor {
  fn perform_layout(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx) -> Size {
    let child_size = host.perform_layout(clamp, ctx);

    if let Some(x) = self.x.clone() {
      ctx.update_anchor_x(ctx.widget_id(), x);
    }
    if let Some(y) = self.y.clone() {
      ctx.update_anchor_y(ctx.widget_id(), y);
    }
    child_size
  }

  #[inline]
  fn wrapper_dirty_phase(&self) -> DirtyPhase { DirtyPhase::Layout }
}

/// Type alias for the custom anchor function.
pub type CustomAnchorFn<T> = fn(&T, Size, BoxClamp, &mut LayoutCtx) -> Anchor;

/// A generalized wrap_render widget that accepts a custom anchor-setting
/// closure. The closure is called after the child layout is complete, receiving
/// the data, child size, clamp, and layout context to set the anchor.
///
/// # Note
///
/// It is recommended to use standard layout widgets (like `Row`, `Column`,
/// `Stack`) to position children whenever possible. `CustomAnchor` is intended
/// for complex or dynamic positioning scenarios that cannot be achieved with
/// standard layouts.
///
/// # Reactivity
///
/// If your anchor logic depends on mutable state, you must pass that state via
/// the `data` field. This ensures that when the state changes, the layout is
/// re-calculated. If you capture a variable directly in the closure without
/// passing it through `data`, the layout will not automatically update when
/// that variable changes.
///
/// # Example
///
/// ```rust
/// use ribir::prelude::*;
///
/// fn_widget! {
///   let state = Stateful::new(10.);
///
///   @CustomAnchor {
///     // Pass the state value via `data`. When `state` changes, `CustomAnchor`
///     // will be notified to re-layout.
///     data: pipe!(*$read(state)),
///     anchor: |offset, _, _, _| {
///       // Use the passed data `offset` to calculate the anchor
///       Anchor::from_point(Point::new(*offset, *offset))
///     },
///     @Text { text: "I move diagonally!" }
///   }
/// };
/// ```
pub struct CustomAnchor<T> {
  data: T,
  anchor: CustomAnchorFn<T>,
}

/// Declarer for `CustomAnchor`.
pub struct CustomAnchorDeclarer<T: 'static> {
  data: Option<PipeValue<T>>,
  anchor: Option<CustomAnchorFn<T>>,
}

impl<T: 'static> CustomAnchorDeclarer<T> {
  /// Sets the anchor closure for the `CustomAnchor`.
  pub fn with_anchor(&mut self, f: CustomAnchorFn<T>) -> &mut Self {
    self.anchor = Some(f);
    self
  }

  /// Sets the data for the `CustomAnchor`.
  pub fn with_data<K: ?Sized>(&mut self, data: impl RInto<PipeValue<T>, K>) -> &mut Self {
    self.data = Some(data.r_into());
    self
  }
}

impl<T: 'static> Declare for CustomAnchor<T> {
  type Builder = CustomAnchorDeclarer<T>;

  fn declarer() -> Self::Builder { CustomAnchorDeclarer { data: None, anchor: None } }
}

impl<T: 'static> ObjDeclarer for CustomAnchorDeclarer<T> {
  type Target = FatObj<Stateful<CustomAnchor<T>>>;

  fn finish(self) -> Self::Target {
    let (data, data_pipe) = self.data.unwrap().unzip();
    let anchor = self.anchor.unwrap();
    let state = Stateful::new(CustomAnchor { data, anchor });
    let writer = state.clone_writer();
    let mut fat = FatObj::new(state);

    if let Some(pipe) = data_pipe {
      let handle = pipe
        .subscribe(move |v| {
          writer.write().data = v;
        })
        .unsubscribe_when_dropped();
      fat.on_disposed(move |_| drop(handle));
    }

    fat
  }
}

impl<T: 'static> WrapRender for CustomAnchor<T> {
  fn perform_layout(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx) -> Size {
    let child_size = host.perform_layout(clamp, ctx);
    let anchor = (self.anchor)(&self.data, child_size, clamp, ctx);
    if let Some(x) = anchor.x {
      ctx.update_anchor_x(ctx.widget_id(), x);
    }
    if let Some(y) = anchor.y {
      ctx.update_anchor_y(ctx.widget_id(), y);
    }
    child_size
  }

  #[inline]
  fn wrapper_dirty_phase(&self) -> DirtyPhase { DirtyPhase::Layout }
}

impl<'c, T: 'static> ComposeChild<'c> for CustomAnchor<T> {
  type Child = Widget<'c>;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    WrapRender::combine_child(this, child)
  }
}
