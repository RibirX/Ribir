use std::cell::RefCell;
#[doc(hidden)]
pub use std::{
  any::{Any, TypeId},
  marker::PhantomData,
  ops::Deref,
};

use ribir_algo::Sc;
use smallvec::SmallVec;
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
  fn hit_test(&self, ctx: &HitTestCtx, pos: Point) -> HitTest {
    let hit = ctx.box_hit_test(pos);
    // If the widget is not solely sized by the parent, indicating it is not a
    // fixed-size container, we permit the child to receive hits even if it
    // extends beyond its parent boundaries.
    HitTest { hit, can_hit_child: hit || !self.only_sized_by_parent() }
  }

  /// Return a transform to map the coordinate from its parent to this widget.
  fn get_transform(&self) -> Option<Transform> { None }
}

/// The common type of all widget can convert to.
pub struct Widget<'w>(InnerWidget<'w>);

enum InnerWidget<'w> {
  Node(Node<'w>),
  Lazy(LazyNode<'w>),
}

enum Node<'w> {
  Leaf(Box<dyn FnOnce() -> WidgetId + 'w>),
  Tree { parent: Box<dyn FnOnce() -> WidgetId + 'w>, children: Vec<Widget<'w>> },
}

/// This serves as a wrapper for `Box<dyn FnOnce(&BuildCtx) -> Node<'w> +
/// 'w>`, but does not utilize the `'w` in the return type to prevent the
/// `LazyWidget` from becoming **invariant**. This approach allows `Widget<'w>`
/// to remain **covariant** with the lifetime `'w`.
///
/// This approach should be acceptable since `LazyWidget` is private and not
/// accessed externally. Additionally, the lifetime will shorten once we consume
/// it to obtain the `Widget<'w>`.
struct LazyNode<'w>(Box<dyn FnOnce() -> Widget<'static> + 'w>);

impl<'w> LazyNode<'w> {
  fn new(f: impl FnOnce() -> Widget<'w> + 'w) -> Self {
    let f: Box<dyn FnOnce() -> Widget<'w> + 'w> = Box::new(f);
    // Safety: the lifetime will shorten once we consume it to obtain the
    // `Widget<'w>`.
    let f: Box<dyn FnOnce() -> Widget<'static> + 'w> = unsafe { std::mem::transmute(f) };
    Self(f)
  }

  fn consume(self) -> Widget<'w> { (self.0)() }
}

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
}

impl<'w> IntoWidget<'w, FN> for Widget<'w> {
  #[inline(always)]
  fn into_widget(self) -> Widget<'w> { self }
}

impl<'w, const M: usize, T: IntoWidgetStrict<'w, M>> IntoWidget<'w, M> for T {
  #[inline(always)]
  fn into_widget(self) -> Widget<'w> { self.into_widget_strict() }
}

impl<C: Compose + 'static> IntoWidgetStrict<'static, COMPOSE> for C {
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
  fn into_widget_strict(self) -> Widget<'w> {
    let lazy = LazyNode::new(self);
    Widget(InnerWidget::Lazy(lazy))
  }
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
    let lazy = move || {
      let node = self.into_node().on_build(f);
      Widget(InnerWidget::Node(node))
    };

    Widget(InnerWidget::Lazy(LazyNode::new(lazy)))
  }

  pub(crate) fn from_render(r: Box<dyn RenderQueryable>) -> Widget<'static> {
    Widget(InnerWidget::Node(Node::Leaf(Box::new(|| BuildCtx::get_mut().alloc(r)))))
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

  pub(crate) fn build(self) -> WidgetId {
    let mut subtrees = vec![];
    let root = self.into_node().build(&mut subtrees);
    let ctx = BuildCtx::get_mut();
    while let Some((p, child)) = subtrees.pop() {
      if ctx.providers.last() == Some(&p) && subtrees.last().map(|(p, _)| p) != Some(&p) {
        ctx.providers.pop();
      } else if ctx.providers.last() != Some(&p) && p.queryable(ctx.tree()) {
        ctx.providers.push(p);
      }
      let c = child.into_node().build(&mut subtrees);
      p.append(c, ctx.tree_mut());
    }
    root
  }

  pub(crate) fn directly_compose_children(self, children: Vec<Widget<'w>>) -> Widget<'w> {
    let mut list: SmallVec<[_; 1]> = SmallVec::default();
    let mut node = Some(self);
    while let Some(n) = node.take() {
      match n.into_node() {
        Node::Leaf(r) => list.push(r),
        Node::Tree { parent, mut children } => {
          list.push(parent);
          if let Some(p) = children.pop() {
            node = Some(p)
          }
          assert!(children.is_empty(), "As a parent widget, it should have exactly one child.")
        }
      }
    }

    let mut node = Node::Tree { parent: list.pop().unwrap(), children };
    while let Some(n) = list.pop() {
      node = Node::Tree { parent: n, children: vec![node.into_widget()] };
    }

    node.into_widget()
  }

  /// Convert an ID back to a widget.
  ///
  /// # Note
  ///
  /// It's important to remember that we construct the tree lazily. In most
  /// cases, you should avoid using this method to create a widget unless you
  /// are certain that the entire logic is suitable for creating this widget
  /// from an ID.
  pub(crate) fn from_id(id: WidgetId) -> Widget<'static> {
    let node = Node::Leaf(Box::new(move || id));
    Widget(InnerWidget::Node(node))
  }

  fn into_node(self) -> Node<'w> {
    let mut w = self;
    loop {
      match w.0 {
        InnerWidget::Node(node) => break node,
        InnerWidget::Lazy(l) => w = l.consume(),
      }
    }
  }
}

impl<'w> Node<'w> {
  fn build(self, subtrees: &mut Vec<(WidgetId, Widget<'w>)>) -> WidgetId {
    match self {
      Node::Leaf(r) => r(),
      Node::Tree { parent, children } => {
        let p = parent();
        for c in children.into_iter().rev() {
          subtrees.push((p, c))
        }
        p
      }
    }
  }

  fn into_widget(self) -> Widget<'w> { Widget(InnerWidget::Node(self)) }

  fn on_build(self, f: impl FnOnce(WidgetId) + 'w) -> Self {
    let on_build_node = |node: Box<dyn FnOnce() -> WidgetId + 'w>| {
      let new_node = move || {
        let id = node();
        f(id);
        id
      };
      Box::new(new_node)
    };

    match self {
      Node::Leaf(n) => Node::Leaf(on_build_node(n)),
      Node::Tree { parent, children } => Node::Tree { parent: on_build_node(parent), children },
    }
  }
}

impl<F: FnMut() -> Widget<'static> + 'static> From<F> for GenWidget {
  #[inline]
  fn from(f: F) -> Self { Self::new(f) }
}
