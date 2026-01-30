use ribir_geom::{Point, Size, Transform};
use smallvec::SmallVec;
use widget_id::RenderQueryable;

use crate::prelude::*;

/// This trait is for a render widget that does not need to be an independent
/// node in the widget tree. It can serve as a wrapper for another render
/// widget.
///
/// # Which widgets should implement this trait?
///
/// If a render widget accepts a single child and its layout size matches its
/// child size, it can be implemented as a `WrapRender` instead of `Render`,
/// eliminating the need to allocate a node in the widget tree.
pub trait WrapRender {
  fn measure(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut MeasureCtx) -> Size {
    host.measure(clamp, ctx)
  }

  fn place_children(&self, size: Size, host: &dyn Render, ctx: &mut PlaceCtx) {
    host.place_children(size, ctx)
  }

  fn paint(&self, host: &dyn Render, ctx: &mut PaintingCtx) { host.paint(ctx) }

  fn size_affected_by_child(&self, host: &dyn Render) -> bool {
    // Detected by its host by default, so we return true here.
    host.size_affected_by_child()
  }

  fn hit_test(&self, host: &dyn Render, ctx: &mut HitTestCtx, pos: Point) -> HitTest {
    host.hit_test(ctx, pos)
  }

  fn get_transform(&self, host: &dyn Render) -> Option<Transform> { host.get_transform() }

  fn visual_box(&self, host: &dyn Render, ctx: &mut VisualCtx) -> Option<Rect> {
    host.visual_box(ctx)
  }

  fn dirty_phase(&self, host: &dyn Render) -> DirtyPhase { host.dirty_phase() }

  fn adjust_position(&self, host: &dyn Render, pos: Point, ctx: &mut PlaceCtx) -> Point {
    host.adjust_position(pos, ctx)
  }

  fn wrapper_dirty_phase(&self) -> DirtyPhase;

  fn combine_x_multi_child(this: impl StateWriter<Value = Self>, x: XMultiChild) -> XMultiChild
  where
    Self: Sized + 'static,
  {
    let combine = combine_method::<Self>(this);
    let parent = CombinedParent { combine: Box::new(combine), parent: x.0 };
    XMultiChild(Box::new(parent))
  }

  fn combine_x_single_child(this: impl StateWriter<Value = Self>, x: XSingleChild) -> XSingleChild
  where
    Self: Sized + 'static,
  {
    let combine = combine_method::<Self>(this);
    let parent = CombinedParent { combine: Box::new(combine), parent: x.0 };
    XSingleChild(Box::new(parent))
  }

  fn combine_child(this: impl StateWriter<Value = Self>, child: Widget) -> Widget
  where
    Self: Sized + 'static,
  {
    let combine = combine_method::<Self>(this);
    combine(child)
  }
}

struct RenderPair {
  wrapper: Box<dyn WrapRender>,
  host: Box<dyn RenderQueryable>,
}

impl Query for RenderPair {
  fn query_all<'q>(&'q self, query_id: &QueryId, out: &mut SmallVec<[QueryHandle<'q>; 1]>) {
    self.host.query_all(query_id, out)
  }

  fn query_all_write<'q>(&'q self, query_id: &QueryId, out: &mut SmallVec<[QueryHandle<'q>; 1]>) {
    self.host.query_all_write(query_id, out)
  }

  fn query<'q>(&'q self, query_id: &QueryId) -> Option<QueryHandle<'q>> {
    self.host.query(query_id)
  }

  fn query_write<'q>(&'q self, query_id: &QueryId) -> Option<QueryHandle<'q>> {
    self.host.query_write(query_id)
  }

  fn queryable(&self) -> bool { self.host.queryable() }
}

impl Render for RenderPair {
  fn measure(&self, clamp: BoxClamp, ctx: &mut MeasureCtx) -> Size {
    self.wrapper.measure(clamp, &*self.host, ctx)
  }

  fn place_children(&self, size: Size, ctx: &mut PlaceCtx) {
    self
      .wrapper
      .place_children(size, &*self.host, ctx)
  }

  fn visual_box(&self, ctx: &mut VisualCtx) -> Option<Rect> {
    self.wrapper.visual_box(&*self.host, ctx)
  }

  fn paint(&self, ctx: &mut PaintingCtx) { self.wrapper.paint(&*self.host, ctx); }

  fn size_affected_by_child(&self) -> bool { self.wrapper.size_affected_by_child(&*self.host) }

  fn hit_test(&self, ctx: &mut HitTestCtx, pos: Point) -> HitTest {
    self
      .wrapper
      .hit_test(self.host.as_render(), ctx, pos)
  }

  fn dirty_phase(&self) -> DirtyPhase { self.wrapper.dirty_phase(self.host.as_render()) }

  fn get_transform(&self) -> Option<Transform> { self.wrapper.get_transform(self.host.as_render()) }

  fn adjust_position(&self, pos: Point, ctx: &mut PlaceCtx) -> Point {
    self
      .wrapper
      .adjust_position(self.host.as_render(), pos, ctx)
  }
}

impl<R> WrapRender for R
where
  R: StateReader,
  R::Value: WrapRender,
{
  fn measure(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut MeasureCtx) -> Size {
    self.read().measure(clamp, host, ctx)
  }

  fn place_children(&self, size: Size, host: &dyn Render, ctx: &mut PlaceCtx) {
    self.read().place_children(size, host, ctx)
  }

  fn paint(&self, host: &dyn Render, ctx: &mut PaintingCtx) { self.read().paint(host, ctx) }

  fn size_affected_by_child(&self, host: &dyn Render) -> bool {
    self.read().size_affected_by_child(host)
  }

  fn hit_test(&self, host: &dyn Render, ctx: &mut HitTestCtx, pos: Point) -> HitTest {
    self.read().hit_test(host, ctx, pos)
  }

  fn get_transform(&self, host: &dyn Render) -> Option<Transform> {
    self.read().get_transform(host)
  }

  fn visual_box(&self, host: &dyn Render, ctx: &mut VisualCtx) -> Option<Rect> {
    self.read().visual_box(host, ctx)
  }

  /// Returns the dirty phase of the wrapped render, this value should
  /// always be the same.
  fn wrapper_dirty_phase(&self) -> DirtyPhase { self.read().wrapper_dirty_phase() }

  fn adjust_position(&self, host: &dyn Render, pos: Point, ctx: &mut PlaceCtx) -> Point {
    self.read().adjust_position(host, pos, ctx)
  }
}

#[macro_export]
macro_rules! impl_compose_child_for_wrap_render {
  ($name:ty) => {
    impl<'c> ComposeChild<'c> for $name {
      type Child = Widget<'c>;
      fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
        WrapRender::combine_child(this, child)
      }
    }
  };
}

pub(crate) use impl_compose_child_for_wrap_render;

pub struct CombinedParent<'p> {
  combine: Box<dyn FnOnce(Widget) -> Widget>,
  parent: Box<dyn BoxedParent + 'p>,
}

impl<'p> BoxedParent for CombinedParent<'p> {
  fn boxed_with_children<'w>(self: Box<Self>, children: Vec<Widget<'w>>) -> Widget<'w>
  where
    Self: 'w,
  {
    let Self { combine, parent } = *self;
    let widget = parent.boxed_with_children(children);
    combine(widget)
  }
}

fn combine_method<Wrapper: WrapRender + 'static>(
  this: impl StateWriter<Value = Wrapper>,
) -> impl FnOnce(Widget) -> Widget {
  let dirty_phase = this.wrapper_dirty_phase();
  move |mut host| {
    let wrapper: Box<dyn WrapRender> = match this.try_into_value() {
      Ok(this) => Box::new(this),
      Err(this) => {
        let reader = match this.into_reader() {
          Ok(r) => r,
          Err(s) => {
            host = host.dirty_on(s.raw_modifies(), dirty_phase);
            s.clone_reader()
          }
        };
        Box::new(reader)
      }
    };

    host.on_build(|id| {
      id.wrap_node(BuildCtx::get_mut().tree_mut(), move |host| {
        Box::new(RenderPair { wrapper, host })
      });
    })
  }
}
