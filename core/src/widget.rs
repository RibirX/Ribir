#[doc(hidden)]
pub use std::{
  any::{Any, TypeId},
  marker::PhantomData,
  ops::Deref,
};
use std::{cell::RefCell, convert::Infallible};

use ops::box_it::CloneableBoxOp;
use ribir_algo::Sc;
use widget_id::RenderQueryable;

pub(crate) use crate::widget_tree::*;
use crate::{context::*, prelude::*, render_helper::PureRender};
pub trait Compose: Sized {
  /// Describes the part of the user interface represented by this widget.
  /// Called by framework, should never directly call it.
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static>;
}

pub struct HitTest {
  pub hit: bool,
  pub can_hit_child: bool,
}

/// RenderWidget is a widget which want to paint something or do a layout to
/// calc itself size and update children positions.
pub trait Render: 'static {
  /// Do the work of computing the layout for this widget, and return the
  /// size it need.
  ///
  /// In implementing this function, You are responsible for calling every
  /// children's perform_layout across the `LayoutCtx`
  ///
  /// ## Guidelines for implementing this method
  ///
  /// - The clamp should restrict the size to always fall within the specified
  ///   range.
  /// - Avoid returning infinity or NaN size, as this could result in a crash.
  ///   If your size calculation is dependent on the `clamp.max`, you might want
  ///   to consider using [`LayoutCtx::fixed_max`].
  /// - Parent has responsibility to call the children's perform_layout, and
  ///   update the children's position. If the children position is not updated
  ///   that will set to zero.
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size;

  /// Draw the widget on the paint device using `PaintingCtx::painter` within
  /// its own coordinate system. This method should not handle painting of
  /// children; the framework will handle painting of children individually. The
  /// framework ensures that the parent is always painted before its children.
  fn paint(&self, _: &mut PaintingCtx) {}

  /// Whether the constraints from parent are the only input to detect the
  /// widget size, and child nodes' size not affect its size.
  fn only_sized_by_parent(&self) -> bool { false }

  /// Verify if the provided position is within this widget and return whether
  /// its child can be hit if the widget itself is not hit.
  fn hit_test(&self, ctx: &mut HitTestCtx, pos: Point) -> HitTest {
    let hit = ctx.box_hit_test(pos);
    // If the widget is not solely sized by the parent, indicating it is not a
    // fixed-size container, we permit the child to receive hits even if it
    // extends beyond its parent boundaries.
    HitTest { hit, can_hit_child: hit || !self.only_sized_by_parent() }
  }

  /// By default, this function returns a `Layout` phase to indicate that the
  /// widget should be marked as dirty when modified. When the layout phase is
  /// marked as dirty, the paint phase will also be affected.
  fn dirty_phase(&self) -> DirtyPhase { DirtyPhase::Layout }

  /// Return a transform to map the coordinate from its parent to this widget.
  fn get_transform(&self) -> Option<Transform> { None }
}

/// The common type of all widget can convert to.
pub struct Widget<'w>(InnerWidget<'w>);

pub struct InnerWidget<'w>(Box<dyn FnOnce(&mut BuildCtx) -> WidgetId + 'w>);

/// A boxed function widget that can be called multiple times to regenerate
/// widget.
#[derive(Clone, ChildOfCompose)]
pub struct GenWidget(InnerGenWidget);
type InnerGenWidget = Sc<RefCell<Box<dyn FnMut() -> Widget<'static>>>>;

/// The `FnWidget<'w>` is a type alias that denotes a boxed trait object of a
/// function widget.
///
/// It already implements `IntoChild`, allowing any function widget to be
/// converted to `FnWidget`. Therefore, using `FnWidget` as the child type of
/// `ComposeChild` enables the acceptance of all function widgets.
#[derive(ChildOfCompose)]
pub struct FnWidget<'w>(Box<dyn FnOnce() -> Widget<'w> + 'w>);

// The widget type marker.
pub const COMPOSE: usize = 1;
pub const RENDER: usize = 2;
pub const FN: usize = 3;
pub const STATELESS_COMPOSE: usize = 4;

/// Defines a trait for converting any widget into a `Widget` type. Direct
/// implementation of this trait is not recommended as it is automatically
/// implemented by the framework.
///
/// Instead, focus on implementing `Compose`, `Render`, or `ComposeChild`.
pub trait IntoWidget<'w, const M: usize>: 'w {
  fn into_widget(self) -> Widget<'w>;
}

/// A trait used by the framework to implement `IntoWidget`. Unlike
/// `IntoWidget`, this trait is not implemented for `Widget` itself. This design
/// choice allows the framework to use either `IntoWidget` or `IntoWidgetStrict`
/// as a generic bound, preventing implementation conflicts.
pub(crate) trait IntoWidgetStrict<'w, const M: usize>: 'w {
  fn into_widget_strict(self) -> Widget<'w>;
}

impl GenWidget {
  pub fn new(f: impl FnMut() -> Widget<'static> + 'static) -> Self {
    Self(Sc::new(RefCell::new(Box::new(f))))
  }

  pub fn gen_widget(&self) -> Widget<'static> { self.0.borrow_mut()() }
}

impl<'w> FnWidget<'w> {
  pub fn new(f: impl FnOnce() -> Widget<'w> + 'w) -> Self { Self(Box::new(f)) }

  pub fn call(self) -> Widget<'w> { (self.0)() }
}

impl<'w> IntoWidget<'w, FN> for Widget<'w> {
  #[inline(always)]
  fn into_widget(self) -> Widget<'w> { self }
}

impl<'w, const M: usize, T: IntoWidgetStrict<'w, M>> IntoWidget<'w, M> for T {
  #[inline(always)]
  fn into_widget(self) -> Widget<'w> { self.into_widget_strict() }
}

impl<C: Compose + 'static> IntoWidgetStrict<'static, STATELESS_COMPOSE> for C {
  #[inline]
  fn into_widget_strict(self) -> Widget<'static> {
    Compose::compose(State::value(self)).into_widget()
  }
}

impl<R: Render + 'static> IntoWidgetStrict<'static, RENDER> for R {
  fn into_widget_strict(self) -> Widget<'static> { Widget::from_render(Box::new(PureRender(self))) }
}

impl<W: ComposeChild<'static, Child = Option<C>>, C> Compose for W {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    ComposeChild::compose_child(this, None)
  }
}

impl<'w, F> IntoWidgetStrict<'w, FN> for F
where
  F: FnOnce() -> Widget<'w> + 'w,
{
  fn into_widget_strict(self) -> Widget<'w> { Widget::from_fn(move |ctx| self().call(ctx)) }
}

impl<'w> IntoWidgetStrict<'w, FN> for FnWidget<'w> {
  #[inline]
  fn into_widget_strict(self) -> Widget<'w> { self.0.into_widget_strict() }
}

impl IntoWidgetStrict<'static, FN> for GenWidget {
  #[inline]
  fn into_widget_strict(self) -> Widget<'static> { self.gen_widget() }
}

impl<'w> Widget<'w> {
  /// Invoke a function when the root node of the widget is built, passing its
  /// ID and build context as parameters.
  pub fn on_build(self, f: impl FnOnce(WidgetId) + 'w) -> Self {
    Widget::from_fn(move |ctx| {
      let id = self.call(ctx);
      f(id);
      id
    })
  }

  /// Subscribe to the modified `upstream` to mark the widget as dirty when the
  /// `upstream` emits a modify event containing `ModifyScope::FRAMEWORK`.
  ///
  /// # Panic
  /// This method only works within a build process; otherwise, it will
  /// result in a panic.
  pub fn dirty_on(
    self, upstream: CloneableBoxOp<'static, ModifyScope, Infallible>, dirty: DirtyPhase,
  ) -> Self {
    let track = TrackWidgetId::default();
    let id = track.track_id();

    let tree = BuildCtx::get_mut().tree_mut();
    let marker = tree.dirty_marker();
    let h = upstream
      .filter(|b| b.contains(ModifyScope::FRAMEWORK))
      .subscribe(move |_| {
        if let Some(id) = id.get() {
          marker.mark(id, dirty);
        }
      })
      .unsubscribe_when_dropped();

    track
      .with_child(self)
      .into_widget()
      .attach_anonymous_data(h)
  }

  pub(crate) fn from_render(r: Box<dyn RenderQueryable>) -> Widget<'static> {
    Widget::from_fn(|_| BuildCtx::get_mut().tree_mut().alloc_node(r))
  }

  /// Attach anonymous data to a widget and user can't query it.
  pub fn attach_anonymous_data(self, data: impl Any) -> Self {
    self.on_build(|id| id.attach_anonymous_data(data, BuildCtx::get_mut().tree_mut()))
  }

  pub fn attach_data(self, data: Box<dyn Query>) -> Self {
    self.on_build(|id| id.attach_data(data, BuildCtx::get_mut().tree_mut()))
  }

  /// Attach a state to a widget and try to unwrap it before attaching.
  ///
  /// User can query the state or its value type.
  pub fn try_unwrap_state_and_attach<D: Any>(
    self, data: impl StateWriter<Value = D> + 'static,
  ) -> Self {
    let data: Box<dyn Query> = match data.try_into_value() {
      Ok(data) => Box::new(Queryable(data)),
      Err(data) => Box::new(data),
    };
    self.attach_data(data)
  }

  /// Convert an ID back to a widget.
  ///
  /// # Note
  ///
  /// It's important to remember that we construct the tree lazily. In most
  /// cases, you should avoid using this method to create a widget unless you
  /// are certain that the entire logic is suitable for creating this widget
  /// from an ID.
  pub(crate) fn from_id(id: WidgetId) -> Widget<'static> { Widget::from_fn(move |_| id) }

  pub(crate) fn new(parent: Widget<'w>, children: Vec<Widget<'w>>) -> Widget<'w> {
    Widget::from_fn(move |ctx| ctx.build_parent(parent, children))
  }

  pub(crate) fn from_fn(f: impl FnOnce(&mut BuildCtx) -> WidgetId + 'w) -> Widget<'w> {
    Widget(InnerWidget(Box::new(f)))
  }

  pub(crate) fn call(self, ctx: &mut BuildCtx) -> WidgetId { (self.0.0)(ctx) }
}

impl<F: FnMut() -> Widget<'static> + 'static> From<F> for GenWidget {
  #[inline]
  fn from(f: F) -> Self { Self::new(f) }
}
