use std::{
  cell::{RefCell, UnsafeCell},
  convert::Infallible,
};

use ribir_algo::Rc;
use rxrust::{
  context::LocalCtx,
  observable::boxed::LocalBoxedObservable,
  scheduler::LocalScheduler,
  subscription::{BoxedSubscription, IntoBoxedSubscription},
};
use smallvec::{SmallVec, smallvec};
use widget_id::RenderQueryable;

use crate::{prelude::*, render_helper::PureRender, ticker::FrameMsg};

pub type ValueStream<V> = LocalBoxedObservable<'static, V, Infallible>;

/// A continuous state stream that drives dynamic widget updates.
///
/// `Pipe` enables reactive programming patterns by converting observable
/// streams into widget properties or structures that automatically update when
/// the stream emits new values.
pub struct Pipe<V> {
  // Internal subscription configuration
  subscribe_info: Rc<RefCell<SubscribeInfo>>,
  // The underlying observable stream
  observable: LocalBoxedObservable<'static, V, Infallible>,
}

impl<V: 'static> Pipe<V> {
  /// Creates a new Pipe from an observable trigger and mapping function.
  ///
  /// # Parameters
  /// - `trigger`: Observable stream of modification scopes
  /// - `map_handler`: Function converting ModifyScope to output values
  pub fn new(
    trigger: LocalBoxedObservable<'static, ModifyInfo, Infallible>,
    map_handler: impl FnMut(ModifyInfo) -> V + 'static,
  ) -> Self {
    let info = SubscribeInfo { effect: ModifyEffect::DATA, priority: None };
    let info: Rc<RefCell<SubscribeInfo>> = Rc::new(RefCell::new(info));
    let (trigger_core, scheduler) = trigger.into_parts();
    let trigger = Local::from_parts(trigger_core, scheduler);
    let observable = Local::from_parts(PipeOp { source: trigger, info: info.clone() }, scheduler)
      .map(map_handler)
      .box_it();

    Pipe { subscribe_info: info, observable }
  }

  /// Creates a new Pipe from a `StateWatcher`.
  ///
  /// The pipe will emit the initial value of the watcher and then emit new
  /// values whenever the watcher modifies.
  pub fn from_watcher<W>(writer: W) -> Self
  where
    W: StateWatcher<Value = V> + 'static,
    V: Clone,
  {
    pipe!((*$read(writer)).clone())
  }

  /// Sets the modification effect filter for this pipe.
  ///
  /// Only modifications matching this effect will trigger updates.
  /// Returns the modified Pipe for chaining.
  pub fn with_effect(self, effect: ModifyEffect) -> Self {
    self.subscribe_info.borrow_mut().effect = effect;
    self
  }

  /// Sets the update priority for widgets created by this pipe.
  ///
  /// Returns the modified Pipe for chaining.
  pub fn with_priority(self, priority: PipeWidgetPriority) -> Self {
    self.subscribe_info.borrow_mut().priority = Some(priority);
    self
  }

  /// Converts this Pipe into its underlying observable stream.
  pub fn into_observable(self) -> LocalBoxedObservable<'static, V, Infallible> { self.observable }

  /// Transforms pipe values using a mapping function.
  ///
  /// Creates a new Pipe with transformed output type.
  pub fn map<U: 'static>(self, f: impl FnMut(V) -> U + 'static) -> Pipe<U> {
    self.transform(move |o| o.map(f))
  }

  /// Applies a transformation to the underlying observable stream, creating a
  /// new Pipe.
  ///
  /// This provides low-level access to chain Rx operators while maintaining
  /// existing pipe configuration (effect and priority).
  ///
  /// # Example
  ///
  /// ``` ignore
  /// pipe.transform(|obs| obs.distinct_until_changed())
  /// ```
  pub fn transform<'a, R>(
    self, transform_op: impl FnOnce(ValueStream<V>) -> Local<R> + 'static,
  ) -> Pipe<R::Item<'a>>
  where
    R: ObservableType<Err = Infallible>,
    R: IntoBoxedCoreObservable<
      <Local<R> as Context>::BoxedCoreObservable<'static, R::Item<'a>, Infallible>,
    >,
  {
    Pipe { subscribe_info: self.subscribe_info, observable: transform_op(self.observable).box_it() }
  }

  /// Provides initial value for the pipe, creating a PipeValue.
  ///
  /// Useful for cold observables that need an initial state.
  pub fn with_init_value(self, init_value: V) -> PipeValue<V> {
    PipeValue::Pipe { pipe: self, init_value }
  }

  /// Builds a single widget that updates from the pipe's stream.
  ///
  /// The widget will be completely replaced when new values are emitted.
  /// Requires the output type to be convertible to a widget.
  pub(crate) fn build_single<K>(self) -> Widget<'static>
  where
    V: RInto<OptionWidget<'static>, K>,
  {
    let pipe_node = PipeNode::empty_node(GenRange::Single(BuildCtx::get().tree().dummy_id()));

    let priority = PipeWidgetPriority::new(pipe_node.clone(), BuildCtx::get().window());
    let pipe = self
      .with_effect(ModifyEffect::FRAMEWORK)
      .with_priority(priority);

    let tree_ptr = BuildCtx::get().tree_ptr();
    Void::default().into_widget().on_build(move |w| {
      pipe_node.init_for_single(w);

      let c_pipe_node = pipe_node.clone();
      let u = pipe.observable.subscribe(move |w| {
        let old = pipe_node.dyn_info().host_id();
        let old_node = pipe_node.take_data();
        let _guard = BuildCtx::try_get()
          .is_none()
          .then(|| BuildCtx::init_for(old, tree_ptr));
        let ctx = BuildCtx::get_mut();
        let new = ctx.build(w.r_into().unwrap_or_void());
        let tree = ctx.tree_mut();
        pipe_node.transplant_to_new(old_node, new, tree);

        query_outside_infos(new, &pipe_node, tree)
          .for_each(|node| node.dyn_info_mut().replace(old, new));

        old.insert_after(new, tree);
        old.dispose_subtree(tree);
        new.on_mounted_subtree(tree);

        tree.dirty_marker().mark(new, DirtyPhase::Layout);
      });
      c_pipe_node.attach_subscription(u);
    })
  }

  /// Builds multiple widgets from a stream of iterable values.
  ///
  /// Returns a list of widgets that will be updated in-place when new values
  /// are emitted.
  pub(crate) fn build_multi<K>(self) -> Vec<Widget<'static>>
  where
    V: IntoIterator<Item: IntoWidget<'static, K>>,
  {
    let dummy_id = BuildCtx::get().tree().dummy_id();
    let node = PipeNode::empty_node(GenRange::Multi { first: dummy_id, last: dummy_id });
    let priority = PipeWidgetPriority::new(node.clone(), BuildCtx::get().window());
    let pipe = self
      .with_effect(ModifyEffect::FRAMEWORK)
      .with_priority(priority);

    let tree_ptr = BuildCtx::get().tree_ptr();
    let pipe_node = node.clone();
    let first = Void::default().into_widget().on_build(move |id| {
      pipe_node.init(id, GenRange::Multi { first: id, last: id });

      let c_pipe_node = pipe_node.clone();

      let u = pipe.observable.subscribe(move |m| {
        let GenRange::Multi { first, last } = pipe_node.dyn_info().gen_range else {
          unreachable!()
        };

        let old_node = pipe_node.take_data();
        let _guard = BuildCtx::try_get()
          .is_none()
          .then(|| BuildCtx::init_for(pipe_node.dyn_info().host_id(), tree_ptr));

        let ctx = BuildCtx::get_mut();
        let mut new = vec![];
        for w in m.into_iter().map(IntoWidget::into_widget) {
          let id = ctx.build(w);
          new.push(id);
        }

        if new.is_empty() {
          new.push(ctx.build(Void::default().into_widget()));
        }
        let n_first = new[0];
        let n_last = new[new.len() - 1];

        let dummy = ctx.build(Void::default().into_widget());
        let tree = ctx.tree_mut();

        if n_first != first {
          pipe_node.transplant_to_new(old_node, n_first, tree);
        } else {
          pipe_node.replace_data(old_node);
        }

        // Attach PipeNode to the last child if different from first and contains
        // PipeNode. We need this attachment because the widget might be
        // regenerated by inner pipes, but avoid external pipe replacement as
        // pipe's multi_child can't be wrapped by others.
        if n_last != n_first && n_last.query_ref::<PipeNode>(tree).is_some() {
          n_last.attach_data(Box::new(QueryFilter::only_self(pipe_node.clone())), tree);
        }

        last.insert_after(dummy, tree);
        new
          .iter()
          .rev()
          .for_each(|w| dummy.insert_after(*w, tree));

        let mut c = Some(last);
        while let Some(w) = c {
          c = w.prev_sibling(tree);
          w.dispose_subtree(tree);
          if w == first {
            break;
          }
        }

        new.iter().for_each(|id| {
          id.on_mounted_subtree(tree);
          tree.dirty_marker().mark(*id, DirtyPhase::Layout);
        });

        dummy.dispose_subtree(tree);

        pipe_node.dyn_info_mut().gen_range = GenRange::Multi { first: n_first, last: n_last };
      });

      c_pipe_node.attach_subscription(u);
    });

    vec![first]
  }

  pub(crate) fn with_children<'w>(self, mut children: Vec<Widget<'w>>) -> Widget<'w>
  where
    V: XParent,
  {
    fn bind_update<P: XParent + 'static>(
      node: PipeNode, observable: LocalBoxedObservable<'static, P, Infallible>,
    ) {
      let pipe_node = node.clone();
      let tree = BuildCtx::get().tree_ptr();
      let u = observable.subscribe(move |w| {
        let GenRange::ParentOnly { parent: old_p, first_leaf } = pipe_node.dyn_info().gen_range
        else {
          unreachable!();
        };

        let old_node = pipe_node.take_data();

        let _guard = BuildCtx::try_get()
          .is_none()
          .then(|| BuildCtx::init_for(pipe_node.dyn_info().host_id(), tree));

        let ctx = BuildCtx::get_mut();

        let mut children = vec![];
        let mut child = Some(first_leaf);
        while let Some(c) = child {
          child = c.next_sibling(ctx.tree_mut());
          children.push(Widget::from_id(c));
        }

        let p = ctx.build(w.x_with_children(children));

        let tree = ctx.tree_mut();
        pipe_node.transplant_to_new(old_node, p, tree);
        query_outside_infos(p, &pipe_node, tree).for_each(|node| {
          node.dyn_info_mut().replace(old_p, p);
        });

        old_p.insert_after(p, tree);
        old_p.dispose_subtree(tree);

        let mut stack: SmallVec<[WidgetId; 1]> = smallvec![p];
        while let Some(c) = stack.pop() {
          if Some(first_leaf) != c.first_child(tree) {
            stack.extend(c.children(tree));
          }
          c.on_widget_mounted(tree);
        }

        tree.dirty_marker().mark(p, DirtyPhase::Layout);
      });

      node.attach_subscription(u);
    }

    let dummy = BuildCtx::get().tree().dummy_id();
    let node = PipeNode::empty_node(GenRange::ParentOnly { parent: dummy, first_leaf: dummy });

    let priority = PipeWidgetPriority::new(node.clone(), BuildCtx::get().window());
    let pipe = self
      .with_effect(ModifyEffect::FRAMEWORK)
      .with_priority(priority);

    assert!(!children.is_empty());
    let first_child = std::mem::replace(&mut children[0], Void::default().into_widget());
    let first_child = first_child.on_build({
      let node = node.clone();
      move |leaf| {
        // We need to associate the parent information with the pipe of leaf widget,
        // when the leaf pipe is regenerated, it can update the parent pipe information
        // accordingly.
        leaf.attach_data(Box::new(node.clone()), BuildCtx::get_mut().tree_mut());
        let GenRange::ParentOnly { first_leaf, .. } = &mut node.dyn_info_mut().gen_range else {
          unreachable!()
        };
        assert_eq!(*first_leaf, dummy);
        *first_leaf = leaf;
        bind_update(node, pipe.observable);
      }
    });
    let _ = std::mem::replace(&mut children[0], first_child);

    #[derive(MultiChild)]
    struct TmpParent;
    impl Render for TmpParent {
      fn measure(&self, clamp: BoxClamp, ctx: &mut MeasureCtx) -> Size {
        let (ctx, children) = ctx.split_children();
        for c in children {
          ctx.layout_child(c, clamp);
        }
        Size::new(0., 0.)
      }
    }

    TmpParent
      .x_with_children(children)
      .on_build(move |p| {
        let GenRange::ParentOnly { parent, first_leaf } = node.dyn_info_mut().gen_range else {
          unreachable!()
        };
        assert_eq!(parent, dummy);
        node.init(p, GenRange::ParentOnly { parent: p, first_leaf });
      })
  }
}

struct SubscribeInfo {
  effect: ModifyEffect,
  priority: Option<PipeWidgetPriority>,
}

/// `PipeNode` just use to wrap a `Box<dyn Render>`, and provide a choice to
/// change the inner `Box<dyn Render>` by `UnsafeCell` at a safe time --
/// although it is stored using a reference counting pointer, its logic
/// guarantees that it is uniquely accessed in the entire call stack and use it
/// locally that not worry about the borrow conflict.
///
/// It's transparent except the `Pipe` widget.
///
/// We use a `PipeNode` wrap the widget node of the pipe, so we can only
/// replace the dynamic part come from the pipe, and keep the static data
/// attached to this node. For example, we attached the unsubscribe handle of
/// the pipe to the first node, and user can attach `key` or `listener` to the
/// widget after `Pipe::build_widget` call.
#[derive(Clone)]
pub(crate) struct PipeNode(Rc<UnsafeCell<InnerPipeNode>>);

struct InnerPipeNode {
  data: Box<dyn RenderQueryable>,
  dyn_info: DynWidgetsInfo,
}

#[derive(Debug)]
pub(crate) struct DynWidgetsInfo {
  /// The generation range of widgets managed by this node
  pub(crate) gen_range: GenRange,
}

/// Represents different widget hierarchy patterns managed by a Pipe
#[derive(Debug, Clone)]
pub(crate) enum GenRange {
  /// Single widget replacement pattern
  Single(WidgetId),
  /// Multiple widgets in a contiguous range
  Multi { first: WidgetId, last: WidgetId },
  /// Parent widget with dynamic children structure
  ParentOnly { parent: WidgetId, first_leaf: WidgetId },
}

impl DynWidgetsInfo {
  pub(crate) fn new(range: GenRange) -> DynWidgetsInfo { DynWidgetsInfo { gen_range: range } }

  pub(crate) fn replace(&mut self, old: WidgetId, new: WidgetId) {
    match &mut self.gen_range {
      GenRange::Single(id) => *id = new,
      GenRange::Multi { first, last } => {
        if first == &old {
          *first = new;
        }
        if last == &old {
          *last = new;
        }
      }
      GenRange::ParentOnly { parent, first_leaf } => {
        if parent == &old {
          *parent = new;
        }
        if first_leaf == &old {
          *first_leaf = new;
        }
      }
    }
  }

  pub(crate) fn host_id(&self) -> WidgetId {
    match &self.gen_range {
      GenRange::Single(id) => *id,
      GenRange::Multi { first, .. } => *first,
      GenRange::ParentOnly { parent, .. } => *parent,
    }
  }
}

impl PipeNode {
  pub(crate) fn empty_node(gen_range: GenRange) -> Self {
    let dyn_info = DynWidgetsInfo::new(gen_range);
    let inner = InnerPipeNode { data: Box::new(PureRender(Void::default())), dyn_info };
    Self(Rc::new(UnsafeCell::new(inner)))
  }

  pub(crate) fn init_for_single(&self, w: WidgetId) { self.init(w, GenRange::Single(w)); }

  pub(crate) fn init(&self, id: WidgetId, range: GenRange) {
    self.dyn_info_mut().gen_range = range;
    let node = self.clone();
    id.wrap_node(BuildCtx::get_mut().tree_mut(), |r| {
      node.as_mut().data = r;
      Box::new(node)
    });
  }

  pub(crate) fn dyn_info(&self) -> &DynWidgetsInfo { &self.as_ref().dyn_info }

  pub(crate) fn dyn_info_mut(&self) -> &mut DynWidgetsInfo { &mut self.as_mut().dyn_info }

  // Remove the old widget so that the new widget build logic cannot access it
  // anymore.
  pub(crate) fn take_data(&self) -> Box<dyn RenderQueryable> {
    self.replace_data(Box::new(PureRender(Void::default())))
  }

  pub(crate) fn replace_data(&self, new: Box<dyn RenderQueryable>) -> Box<dyn RenderQueryable> {
    std::mem::replace(&mut self.as_mut().data, new)
  }

  fn transplant_to_new(
    &self, old_node: Box<dyn RenderQueryable>, new_id: WidgetId, tree: &mut WidgetTree,
  ) {
    let old = self.dyn_info().host_id();

    let [old, new] = tree.get_many_mut(&[old, new_id]);
    std::mem::swap(old, new);

    new.update_track_id(new_id);

    std::mem::swap(&mut self.as_mut().data, old);
    *old = old_node;
  }

  fn as_ref(&self) -> &InnerPipeNode {
    // safety: see the `PipeNode` document.
    unsafe { &*self.0.get() }
  }

  #[allow(clippy::mut_from_ref)]
  fn as_mut(&self) -> &mut InnerPipeNode {
    // safety: see the `PipeNode` document.
    unsafe { &mut *self.0.get() }
  }

  /// Attach a subscription to host widget of the `PipeNode`, and the
  /// subscription will be unsubscribed when the `PipeNode` dropped.
  fn attach_subscription(self, u: impl Subscription + 'static) {
    let tree = BuildCtx::get_mut().tree_mut();
    let node = self.as_mut();
    let id = node.dyn_info.host_id();
    // if the subscription is closed, we can cancel and unwrap the `PipeNode`
    // immediately.
    if u.is_closed() {
      let v = std::mem::replace(&mut node.data, Box::new(PureRender(Void::default())));
      *id.get_node_mut(tree).unwrap() = v;
    } else {
      id.attach_anonymous_data(u.unsubscribe_when_dropped(), tree)
    }
  }
}

fn query_outside_infos<'l>(
  id: WidgetId, to: &'l PipeNode, tree: &'l WidgetTree,
) -> impl Iterator<Item = QueryRef<'l, PipeNode>> {
  let mut hit = false;
  id.query_all_iter::<PipeNode>(tree)
    .rev()
    .take_while(move |info| {
      if hit {
        false
      } else {
        hit = Rc::ptr_eq(&info.0, &to.0);
        true
      }
    })
}

impl Query for PipeNode {
  fn query_all<'q>(&'q self, query_id: &QueryId, out: &mut SmallVec<[QueryHandle<'q>; 1]>) {
    let p = self.as_ref();
    p.data.query_all(query_id, out);
    if query_id == &QueryId::of::<Self>() {
      out.push(QueryHandle::new(self))
    }
  }

  fn query_all_write<'q>(&'q self, query_id: &QueryId, out: &mut SmallVec<[QueryHandle<'q>; 1]>) {
    let p = self.as_ref();
    p.data.query_all_write(query_id, out);
  }

  fn query<'q>(&'q self, query_id: &QueryId) -> Option<QueryHandle<'q>> {
    let p = self.as_ref();
    if query_id == &QueryId::of::<Self>() {
      Some(QueryHandle::new(self))
    } else {
      p.data.query(query_id)
    }
  }

  fn query_write<'q>(&'q self, query_id: &QueryId) -> Option<QueryHandle<'q>> {
    self.as_ref().data.query_write(query_id)
  }

  fn queryable(&self) -> bool { true }
}

impl Render for PipeNode {
  fn measure(&self, clamp: BoxClamp, ctx: &mut MeasureCtx) -> Size {
    self.as_ref().data.measure(clamp, ctx)
  }

  fn place_children(&self, size: Size, ctx: &mut PlaceCtx) {
    self.as_ref().data.place_children(size, ctx)
  }

  fn visual_box(&self, ctx: &mut VisualCtx) -> Option<Rect> { self.as_ref().data.visual_box(ctx) }

  fn paint(&self, ctx: &mut PaintingCtx) { self.as_ref().data.paint(ctx) }

  fn size_affected_by_child(&self) -> bool {
    // A pipe node's size is always affected by its child because it can generate
    // any widget.
    true
  }

  fn hit_test(&self, ctx: &mut HitTestCtx, pos: Point) -> HitTest {
    self.as_ref().data.hit_test(ctx, pos)
  }

  fn get_transform(&self) -> Option<Transform> { self.as_ref().data.get_transform() }

  fn dirty_phase(&self) -> DirtyPhase { self.as_ref().data.dirty_phase() }

  fn adjust_position(&self, pos: Point, ctx: &mut PlaceCtx) -> Point {
    self.as_ref().data.adjust_position(pos, ctx)
  }

  #[cfg(feature = "debug")]
  fn debug_name(&self) -> std::borrow::Cow<'static, str> { self.as_ref().data.debug_name() }

  #[cfg(feature = "debug")]
  fn debug_properties(&self) -> serde_json::Value { self.as_ref().data.debug_properties() }
}

#[derive(Clone)]
pub struct PipeWidgetPriority {
  node: PipeNode,
  wnd: Rc<Window>,
}

impl PipeWidgetPriority {
  fn new(node: PipeNode, wnd: Rc<Window>) -> Self { Self { node, wnd } }

  fn tree(&self) -> &WidgetTree { self.wnd.tree() }
}

impl Priority for PipeWidgetPriority {
  fn priority(&mut self) -> i64 {
    let tree = self.tree();

    let id = self.node.dyn_info().host_id();
    let depth = id.ancestors(tree).count() as i64;
    let embed = query_outside_infos(id, &self.node, tree).count() as i64;

    depth << 32 | embed
  }

  fn queue(&mut self) -> Option<&PriorityTaskQueue> {
    let wnd = self.tree().window();
    let queue = wnd.priority_task_queue();
    // Safety: This trait is only used within this crate, and we can ensure that
    // the window is valid when utilizing the `PriorityTaskQueue`.
    unsafe { std::mem::transmute(queue) }
  }
}

struct PipeOp {
  source: LocalBoxedObservable<'static, ModifyInfo, Infallible>,
  info: Rc<RefCell<SubscribeInfo>>,
}

struct PipeWidgetContextObserver<O> {
  observer: O,
  pipe_node: PipeNode,
  wnd: Rc<Window>,
}

impl<O> Observer<ModifyInfo, Infallible> for PipeWidgetContextObserver<O>
where
  O: Observer<ModifyInfo, Infallible>,
{
  fn next(&mut self, value: ModifyInfo) {
    let wid = self.pipe_node.dyn_info().host_id();

    // Initialize the build context for the duration of the observer callback.
    // This matches the pre-rxrust-update behavior where pipe updates scheduled
    // via priority always execute with a valid `BuildCtx`.
    let old = self.pipe_node.take_data();
    let _guard = BuildCtx::init_for(wid, self.wnd.tree);
    self.pipe_node.replace_data(old);

    self.observer.next(value);
  }

  fn error(self, err: Infallible) { self.observer.error(err); }

  fn complete(self) { self.observer.complete(); }

  fn is_closed(&self) -> bool { self.observer.is_closed() }
}

impl ObservableType for PipeOp {
  type Item<'a>
    = ModifyInfo
  where
    Self: 'a;

  type Err = Infallible;
}

impl<O> CoreObservable<LocalCtx<O, LocalScheduler>> for PipeOp
where
  O: Observer<ModifyInfo, Infallible> + 'static,
{
  type Unsub = BoxedSubscription;

  fn subscribe(self, observer: LocalCtx<O, LocalScheduler>) -> Self::Unsub {
    let (observer, _) = observer.into_parts();

    let mut info = self.info.borrow_mut();
    let scope = info.effect;
    let priority = info.priority.take();

    let source = self.source.filter(move |s| s.contains(scope));

    if let Some(priority) = priority {
      let pipe_node = priority.node.clone();
      let wnd = priority.wnd.clone();

      let sampler = wnd
        .frame_tick_stream()
        .filter_map(|msg| matches!(msg, FrameMsg::NewFrame(_)).then_some(()))
        .merge(Local::of(()));

      let stream = source.sample(sampler).priority(priority);
      let observer = PipeWidgetContextObserver { observer, pipe_node, wnd };
      stream.subscribe_with(observer).into_boxed()
    } else {
      source.subscribe_with(observer).into_boxed()
    }
  }
}

#[cfg(test)]
mod tests {
  use std::{cell::Cell, rc::Rc};

  use ahash::HashSet;

  use crate::{prelude::*, reset_test_env, test_helper::*};

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn pipe_widget_as_root() {
    reset_test_env!();

    let size = Stateful::new(Size::zero());
    let c_size = size.clone_writer();
    let w = fn_widget! {
      let p = pipe! { fn_widget!{ @MockBox { size: *$read(size) }}};
      @(p) { @Void {} }
    };
    let wnd = TestWindow::from_widget(w);
    let tree = wnd.tree_mut();
    let mut queue = vec![];
    tree.layout(Size::zero(), &mut queue);
    let ids = tree
      .content_root()
      .descendants(tree)
      .collect::<Vec<_>>();
    assert_eq!(ids.len(), 2);
    {
      *c_size.write() = Size::new(1., 1.);
    }
    let mut queue = vec![];
    tree.layout(Size::zero(), &mut queue);
    let new_ids = tree
      .content_root()
      .descendants(tree)
      .collect::<Vec<_>>();
    assert_eq!(new_ids.len(), 2);

    assert_eq!(ids[1], new_ids[1]);
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn expr_widget_with_declare_child() {
    reset_test_env!();

    let size = Stateful::new(Size::zero());
    let c_size = size.clone_writer();
    let w = fn_widget! {
      @MockBox {
        size: Size::zero(),
        @ {
          let p = pipe! { fn_widget! {MockBox { size: *$read(size) }}};
          @(p) { @Void {} }
        }
      }
    };
    let wnd = TestWindow::from_widget(w);
    let tree = wnd.tree_mut();
    let mut queue = vec![];
    tree.layout(Size::zero(), &mut queue);
    let ids = tree
      .content_root()
      .descendants(tree)
      .collect::<Vec<_>>();
    assert_eq!(ids.len(), 3);
    {
      *c_size.write() = Size::new(1., 1.);
    }
    let mut queue = vec![];
    tree.layout(Size::zero(), &mut queue);
    let new_ids = tree
      .content_root()
      .descendants(tree)
      .collect::<Vec<_>>();
    assert_eq!(new_ids.len(), 3);

    assert_eq!(ids[0], new_ids[0]);
    assert_eq!(ids[2], new_ids[2]);
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn pipe_widget_mounted_new() {
    reset_test_env!();

    let v = Stateful::new(vec![1, 2, 3]);
    let new_cnt = Stateful::new(0);
    let drop_cnt = Stateful::new(0);

    let c_v = v.clone_writer();
    let c_new_cnt = new_cnt.clone_reader();
    let c_drop_cnt = drop_cnt.clone_reader();
    let w = fn_widget! {
      @MockMulti {
        @ {
          pipe!($read(v).clone()).map(move |v| {
            v.into_iter().map(move |_| {
              @MockBox{
                size: Size::zero(),
                on_mounted: move |_| *$write(new_cnt) += 1,
                on_disposed: move |_| *$write(drop_cnt) += 1
              }
            })
          })
        }
      }
    };

    let wnd = TestWindow::from_widget(w);
    wnd.draw_frame();
    assert_eq!(*c_new_cnt.read(), 3);
    assert_eq!(*c_drop_cnt.read(), 0);

    c_v.write().push(4);
    wnd.draw_frame();
    assert_eq!(*c_new_cnt.read(), 7);
    assert_eq!(*c_drop_cnt.read(), 3);

    c_v.write().pop();
    wnd.draw_frame();
    assert_eq!(*c_new_cnt.read(), 10);
    assert_eq!(*c_drop_cnt.read(), 7);
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn pipe_widget_in_pipe() {
    reset_test_env!();

    let (p, w_p) = split_value(false);
    let (c, w_c) = split_value(false);
    let (mnt_cnt, w_mnt_cnt) = split_value(0);

    let w = fn_widget! {
      pipe!(*$read(p)).map(move |_| fn_widget!{
        @MockBox {
          size: Size::zero(),
          on_mounted: move |_| *$write(w_mnt_cnt) +=1,
          @{
            pipe!(*$read(c)).map(move |_| fn_widget!{
              @MockBox {
                size: Size::zero(),
                on_mounted: move |_| *$write(w_mnt_cnt) +=1,
              }
            })
          }
        }
      })
    };
    {
      let wnd = TestWindow::from_widget(w);
      wnd.draw_frame();
      assert_eq!(*mnt_cnt.read(), 2);

      // trigger the parent update
      *w_p.write() = true;
      // then trigger the child update.
      *w_c.write() = true;

      wnd.draw_frame();
      assert_eq!(*mnt_cnt.read(), 4);

      // old pipe should be unsubscribed.
      *w_p.write() = true;
      *w_c.write() = true;
      wnd.draw_frame();
      assert_eq!(*mnt_cnt.read(), 6);
    }
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn delay_drop_widgets() {
    reset_test_env!();

    #[derive(Default, Clone)]
    struct Task {
      mounted: u32,
      pin: bool,
      paint_cnt: Rc<Cell<u32>>,
      layout_cnt: Rc<Cell<u32>>,
      trigger: u32,
      wid: Option<WidgetId>,
    }

    fn build(task: Stateful<Task>) -> Widget<'static> {
      fn_widget! {
       @TaskWidget {
          keep_alive: pipe!($read(task).pin),
          layout_cnt: pipe!($read(task).layout_cnt.clone()),
          paint_cnt: pipe!($read(task).paint_cnt.clone()),
          trigger: pipe!($read(task).trigger),
          on_mounted: move |ctx| {
            $write(task).mounted += 1;
            $write(task).wid = Some(ctx.id);
          },
          on_disposed: move |ctx| {
            let wid = $write(task).wid.take();
            assert_eq!(wid, Some(ctx.id));
          }
        }
      }
      .into_widget()
    }

    #[declare]
    struct TaskWidget {
      trigger: u32,
      paint_cnt: Rc<Cell<u32>>,
      layout_cnt: Rc<Cell<u32>>,
    }

    impl Render for TaskWidget {
      fn measure(&self, _: BoxClamp, _: &mut MeasureCtx) -> Size {
        self.layout_cnt.set(self.layout_cnt.get() + 1);
        Size::new(1., 1.)
      }

      fn paint(&self, _: &mut PaintingCtx) { self.paint_cnt.set(self.paint_cnt.get() + 1); }
    }

    fn child_count(wnd: &Window) -> usize {
      let tree = wnd.tree();
      let root = tree.content_root();
      root.children(tree).count()
    }

    let tasks = (0..3)
      .map(|_| Stateful::new(Task::default()))
      .collect::<Vec<_>>();
    let tasks = Stateful::new(tasks);
    let c_tasks = tasks.clone_watcher();
    let w = fn_widget! {
      @MockMulti {
        @pipe!{
          $read(c_tasks).iter().map(|t| build(t.clone_writer())).collect::<Vec<_>>()
        }
      }
    };

    let wnd = TestWindow::from_widget(w);
    let mut removed = vec![];

    wnd.draw_frame();
    assert_eq!(child_count(&wnd), 3);

    // the first pined widget will still paint it
    tasks.read()[0].write().pin = true;
    removed.push(tasks.write().remove(0));
    wnd.draw_frame();
    assert_eq!(child_count(&wnd), 2);
    assert_eq!(removed[0].read().paint_cnt.get(), 2);

    // the remove pined widget will paint and no layout when no changed
    let first_layout_cnt = removed[0].read().layout_cnt.get();
    tasks.read().first().unwrap().write().pin = true;
    removed.push(tasks.write().remove(0));
    wnd.draw_frame();
    assert_eq!(child_count(&wnd), 1);
    assert_eq!(removed[0].read().paint_cnt.get(), 3);
    assert_eq!(removed[1].read().paint_cnt.get(), 3);
    assert_eq!(removed[0].read().layout_cnt.get(), first_layout_cnt);

    // the remove pined widget only mark self dirty
    let first_layout_cnt = removed[0].read().layout_cnt.get();
    let second_layout_cnt = removed[1].read().layout_cnt.get();
    let host_layout_cnt = tasks.read()[0].read().layout_cnt.get();
    removed[0].write().trigger += 1;
    wnd.draw_frame();
    assert_eq!(removed[0].read().layout_cnt.get(), first_layout_cnt + 1);
    assert_eq!(removed[0].read().paint_cnt.get(), 4);
    assert_eq!(removed[1].read().layout_cnt.get(), second_layout_cnt);
    assert_eq!(tasks.read()[0].read().layout_cnt.get(), host_layout_cnt);

    // when unpined, it will no paint anymore
    removed[0].write().pin = false;
    wnd.draw_frame();
    assert_eq!(removed[0].read().paint_cnt.get(), 4);
    assert_eq!(removed[1].read().paint_cnt.get(), 5);

    // after removed, it will no paint and layout anymore
    let first_layout_cnt = removed[0].read().layout_cnt.get();
    removed[0].write().trigger += 1;
    wnd.draw_frame();
    assert_eq!(removed[0].read().paint_cnt.get(), 4);
    assert_eq!(removed[1].read().paint_cnt.get(), 5);
    assert_eq!(removed[0].read().layout_cnt.get(), first_layout_cnt);

    // other pined widget is work fine.
    let first_layout_cnt = removed[0].read().layout_cnt.get();
    let second_layout_cnt = removed[1].read().layout_cnt.get();
    removed[1].write().trigger += 1;
    wnd.draw_frame();
    assert_eq!(removed[0].read().paint_cnt.get(), 4);
    assert_eq!(removed[1].read().paint_cnt.get(), 6);
    assert_eq!(removed[0].read().layout_cnt.get(), first_layout_cnt);
    assert_eq!(removed[1].read().layout_cnt.get(), second_layout_cnt + 1,);
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn remove_delay_drop_widgets() {
    reset_test_env!();

    let child = Stateful::new(Some(()));
    let child_destroy_until = Stateful::new(false);
    let grandson = Stateful::new(Some(()));
    let grandson_destroy_until = Stateful::new(false);
    let c_child = child.clone_writer();
    let c_child_destroy_until = child_destroy_until.clone_writer();

    let w = fn_widget! {
      @MockMulti {
        @ { pipe!(*$read(child)).map(move |_| fn_widget!{
          @MockMulti {
            keep_alive: pipe!(!*$read(child_destroy_until)),
            @ { pipe!(*$read(grandson)).map(move |_| fn_widget!{
              @MockBox {
                keep_alive: pipe!(!*$read(grandson_destroy_until)),
                size: Size::zero(),
              }
            })}
          }
        })}
      }
    };
    let wnd = TestWindow::from_widget(w);
    wnd.draw_frame();
    let grandson_id = {
      let tree = wnd.tree();
      let root = tree.content_root();
      root
        .first_child(tree)
        .unwrap()
        .first_child(tree)
        .unwrap()
    };

    wnd.draw_frame();
    assert!(!grandson_id.is_dropped(wnd.tree()));

    c_child.write().take();
    wnd.draw_frame();
    assert!(!grandson_id.is_dropped(wnd.tree()));

    *c_child_destroy_until.write() = true;
    wnd.draw_frame();
    assert!(grandson_id.is_dropped(wnd.tree()));
  }

  #[test]
  fn widget_from_pipe_widget() {
    reset_test_env!();
    let _ = fn_widget! {
      let v = Stateful::new(true);
      let w = pipe!(*$read(v)).map(move |_| fn_widget!{ @ Void {} });
      w.into_widget()
    };
  }

  #[test]
  fn multi_pipe_gen_single_pipe() {
    reset_test_env!();
    let box_count = Stateful::new(1);
    let child_size = Stateful::new(Size::new(1., 1.));
    let c_box_count = box_count.clone_writer();
    let c_child_size = child_size.clone_writer();
    let w = fn_widget! {
      @MockMulti {
        @ {
          pipe!(*$read(box_count)).map(move |v| {
            (0..v).map(move |_| fn_widget!{
              pipe!(*$read(child_size)).map(move |size| fn_widget! { @MockBox { size } })
            })
          })
        }
      }
    };

    let wnd = TestWindow::from_widget(w);
    wnd.draw_frame();
    wnd.assert_root_size(Size::new(1., 1.));

    *c_child_size.write() = Size::new(2., 1.);
    wnd.draw_frame();
    wnd.assert_root_size(Size::new(2., 1.));

    *c_box_count.write() = 2;
    wnd.draw_frame();
    wnd.assert_root_size(Size::new(4., 1.));
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn multi_pipe_gen_pipe_parent_pipe_only() {
    reset_test_env!();
    let box_count = Stateful::new(1);
    let child_size = Stateful::new(Size::new(1., 1.));
    let c_box_count = box_count.clone_writer();
    let c_child_size = child_size.clone_writer();
    let w = fn_widget! {
      @MockMulti {
        @ {
          pipe!(*$read(box_count)).map(move |v| {
            (0..v).map(move |_| {
              let pipe_parent = pipe!(*$read(child_size))
                .map(move |size| fn_widget!{ @MockBox { size } });
              @(pipe_parent) { @Void {} }
            })
          })
        }
      }
    };

    let wnd = TestWindow::from_widget(w);
    wnd.draw_frame();
    wnd.assert_root_size(Size::new(1., 1.));

    *c_child_size.write() = Size::new(2., 1.);
    wnd.draw_frame();
    wnd.assert_root_size(Size::new(2., 1.));

    *c_box_count.write() = 2;
    wnd.draw_frame();
    wnd.assert_root_size(Size::new(4., 1.));
  }

  #[test]
  fn single_pipe_gen_single_pipe() {
    reset_test_env!();
    let pipe_trigger = Stateful::new(0);
    let inner_pipe_trigger = Stateful::new(0);
    let c_pipe_trigger = pipe_trigger.clone_writer();
    let c_inner_pipe_trigger = inner_pipe_trigger.clone_writer();
    let w = fn_widget! {
      @MockMulti {
        @ {
          pipe!(*$read(pipe_trigger)).map(move |w| fn_widget!{
            pipe!(*$read(inner_pipe_trigger))
              .map(move |h| fn_widget! { @MockBox { size: Size::new(w as f32, h as f32) } })
          })
        }
      }
    };

    let wnd = TestWindow::from_widget(w);
    wnd.draw_frame();
    wnd.assert_root_size(Size::new(0., 0.));

    *c_inner_pipe_trigger.write() += 1;
    wnd.draw_frame();
    wnd.assert_root_size(Size::new(0., 1.));

    *c_pipe_trigger.write() += 1;
    wnd.draw_frame();
    wnd.assert_root_size(Size::new(1., 1.));
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn single_pipe_gen_parent_pipe_only() {
    reset_test_env!();
    let (inner, w_inner) = split_value(0);
    let (outer, w_outer) = split_value(0);

    let w = fn_widget! {
      @MockMulti {
        @ {
          pipe!(*$read(outer)).map(move |w| fn_widget!{
            let pipe_parent = pipe!(*$read(inner))
              .map(move |h| fn_widget! {@MockBox { size: Size::new(w as f32, h as f32) } });
            @(pipe_parent) { @Void {} }
          })
        }
      }
    };

    let wnd = TestWindow::from_widget(w);
    wnd.draw_frame();
    wnd.assert_root_size(Size::new(0., 0.));

    // Inner pipe update
    *w_inner.write() += 1;
    wnd.draw_frame();
    wnd.assert_root_size(Size::new(0., 1.));

    // Outer pipe update
    *w_outer.write() += 1;
    wnd.draw_frame();
    wnd.assert_root_size(Size::new(1., 1.));
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn parent_pipe_only_gen_single_pipe() {
    reset_test_env!();
    let pipe_trigger = Stateful::new(0);
    let inner_pipe_trigger = Stateful::new(0);
    let c_pipe_trigger = pipe_trigger.clone_writer();
    let c_inner_pipe_trigger = inner_pipe_trigger.clone_writer();
    let w = fn_widget! {
      @MockMulti {
        @ {
          let p = pipe!(*$read(pipe_trigger)).map(move |w| fn_widget!{
            pipe!(*$read(inner_pipe_trigger))
              .map(move |h| fn_widget! { @MockBox { size: Size::new(w as f32, h as f32) }})
          });

          @(p) { @Void {} }
        }
      }
    };

    let wnd = TestWindow::from_widget(w);
    wnd.draw_frame();
    wnd.assert_root_size(Size::new(0., 0.));

    *c_inner_pipe_trigger.write() += 1;
    wnd.draw_frame();
    wnd.assert_root_size(Size::new(0., 1.));

    *c_pipe_trigger.write() += 1;
    wnd.draw_frame();
    wnd.assert_root_size(Size::new(1., 1.));
  }

  #[test]
  fn fix_pipe_gen_pipe_widget_leak() {
    reset_test_env!();

    let parent = Stateful::new(true);
    let child = Stateful::new(true);
    let hit_count = Stateful::new(0);
    let c_parent = parent.clone_writer();
    let c_child = child.clone_writer();
    let c_hit_count = hit_count.clone_writer();

    let w = fn_widget! {
      pipe!($read(parent);).map(move |_| fn_widget!{
        pipe!($read(child);).map(move |_| fn_widget!{
          *$write(hit_count) += 1;
          Void::default()
        })
      })
    };

    let wnd = TestWindow::from_widget(w);
    wnd.draw_frame();
    assert_eq!(*c_hit_count.read(), 1);

    *c_parent.write() = false;
    wnd.draw_frame();
    assert_eq!(*c_hit_count.read(), 2);

    *c_child.write() = false;
    wnd.draw_frame();
    // if the child pipe not reset, the hit count will be 4.
    assert_eq!(*c_hit_count.read(), 3);
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn two_pipe_gen_same_render_widget() {
    reset_test_env!();

    let (r1, w1) = split_value(1.);
    let (r2, w2) = split_value(1.);

    let widget = fn_widget! {
      @MockMulti {
        @{
          pipe!(*$read(r1)).map(move |_| fn_widget!{
            pipe!(*$read(r2))
              .map(move |r| fn_widget!{
                @MockBox {
                  background: Color::YELLOW,
                  size: Size::new(100.0, 10.0 * r + 100.),
                }
            })
          })
        }
      }
    };

    let wnd = TestWindow::from_widget(widget);
    wnd.draw_frame();
    *w1.write() += 1.;
    *w2.write() += 1.;
    wnd.draw_frame();
    *w2.write() += 1.;
    *w1.write() += 1.;
    wnd.draw_frame();
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn multi_pipe_gen_pipe_in_order() {
    reset_test_env!();
    let w = Stateful::new(vec![]);
    let w2 = w.clone_writer();
    let widget = fn_widget! {
      @MockMulti {
        @ {
          pipe!($read(w);).map(move |_| {
            $write(w).silent().push(0);
            (0..10).map(move |idx| {
              pipe!{
                $write(w).silent().push(idx + 1);
                @MockBox {
                  size: Size::new(1., 1.),
                }
              }
            })
          })
        }
      }
    };

    let wnd = TestWindow::from_widget(widget);
    wnd.draw_frame();
    assert_eq!(w2.read()[0], 0);
    // The children order is not fixed
    assert_eq!(w2.read().iter().collect::<HashSet<_>>().len(), 11);
    w2.write().clear();
    wnd.draw_frame();
    assert_eq!(w2.read()[0], 0);
    assert_eq!(w2.read().iter().collect::<HashSet<_>>().len(), 11);
  }

  #[test]
  fn fix_pipe_in_multi_pipe_not_first() {
    reset_test_env!();
    let (m_watcher, m_writer) = split_value(0);
    let (son_watcher, son_writer) = split_value(0);

    let widget = fn_widget! {
      @MockMulti {
        @ {

          pipe!($read(m_watcher);).map(move |_| {
            [
              Void::default().into_widget() ,
              pipe!($read(son_watcher);).map(|_| fn_widget! {@Void{}}).into_widget(),
            ]
          })
        }
      }
    };

    let wnd = TestWindow::from_widget(widget);
    wnd.draw_frame();
    *son_writer.write() += 1;
    wnd.draw_frame();
    // If the parent pipe is not correctly collected, it may cause a panic when
    // refreshing the child widget of the pipe.
    *m_writer.write() += 1;
    wnd.draw_frame();
  }

  #[test]
  fn fix_pipe_in_parent_only_pipe_at_end() {
    reset_test_env!();
    let (m_watcher, m_writer) = split_value(0);
    let (son_watcher, son_writer) = split_value(0);

    let widget = fn_widget! {
      let p = @ {
        pipe!($read(m_watcher);).map(move |_| fn_widget!{
          // margin is static, but its child MockBox is a pipe.
          let p = pipe!($read(son_watcher);).map(|_| fn_widget! { MockBox { size: Size::zero() }});
          let mut obj = FatObj::new(p);
          obj.with_margin(EdgeInsets::all(1.));
          obj
        })
      };
      @(p) {
        @{ Void::default() }
      }
    };

    let wnd = TestWindow::from_widget(widget);
    wnd.draw_frame();
    *son_writer.write() += 1;
    wnd.draw_frame();

    // If the parent pipe is not correctly collected, it may cause a panic when
    // refreshing the child widget of the pipe.
    *m_writer.write() += 1;
    wnd.draw_frame();
  }
}
