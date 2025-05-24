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
use crate::{context::*, prelude::*};

/// Defines how a type composes its user interface representation from its state
///
/// Implement this trait for types that need to create widget hierarchies based
/// on their internal state.
pub trait Compose {
  fn compose(state: impl StateWriter<Value = Self>) -> Widget<'static>
  where
    Self: Sized;
}

/// Core rendering interface for visual widgets
///
/// Implement this trait for widgets that need custom layout calculation,
/// painting logic, or hit testing behavior. The framework calls these methods
/// during different phases of the rendering pipeline.
pub trait Render: 'static {
  /// Calculate widget layout within constraints
  ///
  /// # Parameters
  /// - `clamp`: Size constraints from parent
  /// - `ctx`: Layout context and child management
  ///
  /// # Implementation Guide
  /// 1. **Constraint Handling**:
  ///    - Respect clamp.min/max boundaries
  /// 2. **Child Management**:
  ///    - Call `ctx.perform_layout()` for each child
  ///    - Set child positions via `LayoutCtx`
  /// 3. **Size Safety**:
  ///    - Never return infinite/NaN sizes
  ///    - Fall back to clamp limits for invalid calculations
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size;

  /// Custom painting implementation
  ///
  /// Use `PaintingCtx::painter` for drawing operations. Child widgets are
  /// painted automatically by the framework after parent painting completes.
  fn paint(&self, _: &mut PaintingCtx) {}

  /// Calculates the visual bounding box of the widget's painting effects
  ///
  /// # Parameters
  /// - `ctx`: Provides access to layout and visual context information
  ///
  /// # Returns
  /// - `Some(Rect)`: Bounding box in local coordinates if the widget paints
  ///   content
  /// - `None`: Default value indicating no visual representation
  #[allow(unused_variables)]
  fn visual_box(&self, ctx: &mut VisualCtx) -> Option<Rect> { None }

  /// Child size dependency flag
  ///
  /// Return `false` for fixed-size containers to optimize layout passes.
  /// Default implementation assumes child-dependent sizing.
  fn size_affected_by_child(&self) -> bool { true }

  /// Hit testing implementation
  ///
  /// # Parameters
  /// - `ctx`: Hit test context and helpers
  /// - `pos`: Test position in local coordinates
  ///
  /// Return `HitTest` with:
  /// - `hit`: Whether position intersects widget
  /// - `can_hit_child`: Whether to test child widgets
  fn hit_test(&self, ctx: &mut HitTestCtx, pos: Point) -> HitTest {
    let hit = ctx.box_hit_test(pos);
    HitTest { hit, can_hit_child: hit || self.size_affected_by_child() }
  }

  /// Dirty state propagation control
  ///
  /// Return `DirtyPhase::Layout` (default) to mark as dirty on modifications.
  /// Use `DirtyPhase::Visual` for paint-only updates.
  fn dirty_phase(&self) -> DirtyPhase { DirtyPhase::Layout }

  /// Custom coordinate transformation
  ///
  /// Return `Some(Transform)` to apply local-to-parent transformation.
  /// Used for widgets with custom positioning or transformation effects.
  fn get_transform(&self) -> Option<Transform> { None }
}

/// Result of a hit testing operation
///
/// Contains both the hit status and child hit testing policy:
/// - `hit`: True if the widget itself was hit
/// - `can_hit_child`: Whether to continue testing child widgets
pub struct HitTest {
  pub hit: bool,
  pub can_hit_child: bool,
}

/// Primary widget handle type
///
/// Contains either static content or dynamic generator function.
/// All widget composition operations eventually produce this type.
pub struct Widget<'w>(InnerWidget<'w>);

/// Internal widget representation
pub(crate) struct InnerWidget<'w>(Box<dyn FnOnce(&mut BuildCtx) -> WidgetId + 'w>);

/// Conversion interface for widget-like types
///
/// Automatically implemented for all types that can be converted to [`Widget`]
/// through the [`RInto`] trait system.
pub trait IntoWidget<'a, K> {
  fn into_widget(self) -> Widget<'a>;
}

/// Reusable widget generator
///
/// Contains a boxed closure that can produce new widget instances on demand.
#[derive(Clone)]
pub struct GenWidget(InnerGenWidget);
type InnerGenWidget = Sc<RefCell<Box<dyn FnMut() -> Widget<'static>>>>;

/// Single-use widget generator
///
/// Wraps a closure that produces a widget when called.
pub struct FnWidget<W, F: FnOnce() -> W>(pub(crate) F);
pub type BoxFnWidget<'w> = Box<dyn FnOnce() -> Widget<'w> + 'w>;

impl<W, F> FnWidget<W, F>
where
  F: FnOnce() -> W,
{
  pub fn new<'w, K>(f: F) -> Self
  where
    W: IntoWidget<'w, K>,
  {
    Self(f)
  }

  pub fn into_inner(self) -> F { self.0 }

  pub fn call(self) -> W { (self.0)() }

  pub fn boxed<'w, K>(self) -> BoxFnWidget<'w>
  where
    W: IntoWidget<'w, K> + 'w,
    F: 'w,
  {
    Box::new(move || self.call().into_widget())
  }
}

impl GenWidget {
  pub fn new<W, K>(mut f: impl FnMut() -> W + 'static) -> Self
  where
    W: IntoWidget<'static, K>,
  {
    Self(Sc::new(RefCell::new(Box::new(move || f().into_widget()))))
  }

  pub fn from_fn_widget<F, W, K>(f: FnWidget<W, F>) -> Self
  where
    F: FnMut() -> W + 'static,
    W: IntoWidget<'static, K>,
  {
    Self::new(f.into_inner())
  }

  pub fn gen_widget(&self) -> Widget<'static> { self.0.borrow_mut()() }
}

impl<W: ComposeChild<'static, Child = Option<C>>, C> Compose for W {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    ComposeChild::compose_child(this, None)
  }
}

impl<'w> Widget<'w> {
  /// Register build completion callback
  ///
  /// The provided closure receives the final [`WidgetId`] after this widget
  /// has been built.
  pub fn on_build(self, f: impl FnOnce(WidgetId) + 'w) -> Self {
    Widget::from_fn(move |ctx| {
      let id = self.call(ctx);
      f(id);
      id
    })
  }

  /// Establish reactive dirtiness tracking
  pub fn dirty_on(
    self, upstream: CloneableBoxOp<'static, ModifyInfo, Infallible>, dirty: DirtyPhase,
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

  pub(crate) fn from_render(r: Box<dyn RenderQueryable>) -> Widget<'static> {
    Widget::from_fn(|_| BuildCtx::get_mut().tree_mut().alloc_node(r))
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

impl From<GenWidget> for Widget<'static> {
  fn from(widget: GenWidget) -> Self { FnWidget::new(move || widget.gen_widget()).into_widget() }
}

// ----- Into Widget --------------

impl<'w, W, K> IntoWidget<'w, K> for W
where
  W: RInto<Widget<'w>, K>,
{
  fn into_widget(self) -> Widget<'w> { self.r_into() }
}
