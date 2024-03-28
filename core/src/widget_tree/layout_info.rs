use std::{collections::HashMap, rc::Rc};

use ribir_geom::ZERO_SIZE;

use super::{WidgetId, WidgetTree};
use crate::{
  context::{AppCtx, LayoutCtx, WidgetCtx, WidgetCtxImpl},
  prelude::{Point, Size, INFINITY_SIZE},
  widget::TreeArena,
  window::{DelayEvent, Window, WindowId},
};

/// boundary limit of the render object's layout
#[derive(Debug, Clone, PartialEq, Copy)]
pub struct BoxClamp {
  pub min: Size,
  pub max: Size,
}

impl BoxClamp {
  /// clamp use to expand the width to max
  pub const EXPAND_X: BoxClamp =
    BoxClamp { min: Size::new(f32::INFINITY, 0.), max: Size::new(f32::INFINITY, f32::INFINITY) };

  /// clamp use to expand the height to max
  pub const EXPAND_Y: BoxClamp =
    BoxClamp { min: Size::new(0., f32::INFINITY), max: Size::new(f32::INFINITY, f32::INFINITY) };

  /// clamp use to expand the size to max
  pub const EXPAND_BOTH: BoxClamp = BoxClamp {
    min: Size::new(f32::INFINITY, f32::INFINITY),
    max: Size::new(f32::INFINITY, f32::INFINITY),
  };

  /// clamp use fixed width and unfixed height
  pub const fn fixed_width(width: f32) -> Self {
    BoxClamp { min: Size::new(width, 0.), max: Size::new(width, f32::INFINITY) }
  }

  /// clamp use fixed height and unfixed width
  pub const fn fixed_height(height: f32) -> Self {
    BoxClamp { min: Size::new(0., height), max: Size::new(f32::INFINITY, height) }
  }

  /// clamp use fixed size
  pub const fn fixed_size(size: Size) -> Self { BoxClamp { min: size, max: size } }

  pub const fn min_width(width: f32) -> Self {
    let mut clamp = BoxClamp::EXPAND_BOTH;
    clamp.min.width = width;
    clamp
  }

  pub const fn min_height(height: f32) -> Self {
    let mut clamp = BoxClamp::EXPAND_BOTH;
    clamp.min.height = height;
    clamp
  }

  pub fn with_fixed_height(mut self, height: f32) -> Self {
    self.min.height = height;
    self.max.height = height;
    self
  }

  pub fn with_fixed_width(mut self, width: f32) -> Self {
    self.min.width = width;
    self.max.width = width;
    self
  }
}

/// render object's layout box, the information about layout, including box
/// size, box position, and the clamp of render object layout.
#[derive(Debug, Default, Clone)]
pub struct LayoutInfo {
  /// Box bound is the bound of the layout can be place. it will be set after
  /// render object computing its layout. It's passed by render object's parent.
  pub clamp: BoxClamp,
  /// object's layout size, Some value after the render
  /// object has been layout, otherwise is none value.
  pub size: Option<Size>,
  /// The position render object to place, default is zero
  pub pos: Point,
}

/// Store the render object's place relative to parent coordinate and the
/// clamp passed from parent.
#[derive(Default)]
pub(crate) struct LayoutStore {
  data: HashMap<WidgetId, LayoutInfo, ahash::RandomState>,
}

pub struct Layouter<'a> {
  pub(crate) id: WidgetId,
  pub(crate) wnd_id: WindowId,
  pub(crate) is_layout_root: bool,
  pub(crate) tree: &'a mut WidgetTree,
}

impl<'a> WidgetCtxImpl for Layouter<'a> {
  #[inline]
  fn id(&self) -> WidgetId { self.id }
  #[inline]
  fn current_wnd(&self) -> Rc<Window> { AppCtx::get_window_assert(self.wnd_id) }
  fn with_tree<F: FnOnce(&WidgetTree) -> R, R>(&self, f: F) -> R { f(self.tree) }
}

impl LayoutStore {
  /// Remove the layout info of the `wid`
  pub(crate) fn force_layout(&mut self, id: WidgetId) -> Option<LayoutInfo> { self.remove(id) }

  pub(crate) fn remove(&mut self, id: WidgetId) -> Option<LayoutInfo> { self.data.remove(&id) }

  pub(crate) fn layout_box_size(&self, id: WidgetId) -> Option<Size> {
    self.layout_info(id).and_then(|info| info.size)
  }

  pub(crate) fn layout_box_position(&self, id: WidgetId) -> Option<Point> {
    self.layout_info(id).map(|info| info.pos)
  }

  pub(crate) fn layout_info(&self, id: WidgetId) -> Option<&LayoutInfo> { self.data.get(&id) }

  /// return a mutable reference of the layout info  of `id`, if it's not exist
  /// insert a default value before return
  pub(crate) fn layout_info_or_default(&mut self, id: WidgetId) -> &mut LayoutInfo {
    self.data.entry(id).or_default()
  }

  pub(crate) fn map_to_parent(&self, id: WidgetId, pos: Point, arena: &TreeArena) -> Point {
    self
      .layout_box_position(id)
      .map_or(pos, |offset| {
        let pos = id
          .assert_get(arena)
          .get_transform()
          .map_or(pos, |t| t.transform_point(pos));
        pos + offset.to_vector()
      })
  }

  pub(crate) fn map_from_parent(&self, id: WidgetId, pos: Point, arena: &TreeArena) -> Point {
    self
      .layout_box_position(id)
      .map_or(pos, |offset| {
        let pos = pos - offset.to_vector();
        id.assert_get(arena)
          .get_transform()
          .map_or(pos, |t| {
            t.inverse()
              .map_or(pos, |t| t.transform_point(pos))
          })
      })
  }

  pub(crate) fn map_to_global(&self, pos: Point, widget: WidgetId, arena: &TreeArena) -> Point {
    widget
      .ancestors(arena)
      .fold(pos, |pos, p| self.map_to_parent(p, pos, arena))
  }

  pub(crate) fn map_from_global(&self, pos: Point, widget: WidgetId, arena: &TreeArena) -> Point {
    let stack = widget.ancestors(arena).collect::<Vec<_>>();
    stack
      .iter()
      .rev()
      .fold(pos, |pos, p| self.map_from_parent(*p, pos, arena))
  }
}

impl BoxClamp {
  #[inline]
  pub fn clamp(self, size: Size) -> Size { size.clamp(self.min, self.max) }

  #[inline]
  pub fn expand(mut self) -> Self {
    self.max = INFINITY_SIZE;
    self
  }

  #[inline]
  pub fn loose(mut self) -> Self {
    self.min = ZERO_SIZE;
    self
  }
}

impl<'a> Layouter<'a> {
  /// perform layout of the widget this `ChildLayouter` represent,
  /// reset the widget position back to (0, 0) relative to parent, return the
  /// size result after layout
  pub fn perform_widget_layout(&mut self, clamp: BoxClamp) -> Size {
    let info = self.tree.store.layout_info(self.id);
    let size = info
      .filter(|info| info.clamp == clamp)
      .and_then(|info| info.size)
      .unwrap_or_else(|| {
        // Safety: the `tree` just use to get the widget of `id`, and `tree2` not drop
        // or modify it during perform layout.
        let tree2 = unsafe { &mut *(self.tree as *mut WidgetTree) };

        let Self { id, wnd_id, ref tree, .. } = *self;
        let mut ctx = LayoutCtx { id, wnd_id, tree: tree2 };
        let size = id
          .assert_get(&tree.arena)
          .perform_layout(clamp, &mut ctx);
        // The dynamic widget maybe generate a new widget to instead of self. In that
        // way we needn't add a layout event because it perform layout in another widget
        // and added the event in that widget.
        if id == ctx.id {
          self
            .window()
            .add_delay_event(DelayEvent::PerformedLayout(id));
        } else {
          self.id = ctx.id;
        }

        let info = tree2.store.layout_info_or_default(id);
        let size = clamp.clamp(size);
        info.clamp = clamp;
        info.size = Some(size);

        size
      });

    if !self.is_layout_root {
      self.update_position(Point::zero())
    }

    size
  }

  /// Get layouter of the next sibling of this layouter, panic if self is not
  /// performed layout.
  pub fn into_next_sibling(mut self) -> Option<Self> {
    assert!(
      self.box_rect().is_some(),
      "Before try to layout next sibling, self must performed layout."
    );
    let next = self.id.next_sibling(&self.tree.arena);
    next.map(move |sibling| {
      self.id = sibling;
      self
    })
  }

  /// Return layouter of the first child of this widget.
  #[inline]
  pub fn into_first_child_layouter(mut self) -> Option<Self> {
    self.first_child().map(|id| {
      self.id = id;
      self
    })
  }

  /// Update the position of the child render object should place. Relative to
  /// parent.
  #[inline]
  pub fn update_position(&mut self, pos: Point) {
    self
      .tree
      .store
      .layout_info_or_default(self.id)
      .pos = pos;
  }

  /// Update the size of layout widget. Use this method to directly change the
  /// size of a widget, in most cast you needn't call this method, use clamp to
  /// limit the child size is enough. Use this method only it you know what you
  /// are doing.
  #[inline]
  pub fn update_size(&mut self, child: WidgetId, size: Size) {
    self.tree.store.layout_info_or_default(child).size = Some(size);
  }
}

impl<'a> Layouter<'a> {
  pub(crate) fn new(
    id: WidgetId, wnd_id: WindowId, is_layout_root: bool, tree: &'a mut WidgetTree,
  ) -> Self {
    Self { id, wnd_id, is_layout_root, tree }
  }
}

impl Default for BoxClamp {
  fn default() -> Self {
    Self { min: Size::new(0., 0.), max: Size::new(f32::INFINITY, f32::INFINITY) }
  }
}

impl std::ops::Deref for LayoutStore {
  type Target = HashMap<WidgetId, LayoutInfo, ahash::RandomState>;
  fn deref(&self) -> &Self::Target { &self.data }
}

impl std::ops::DerefMut for LayoutStore {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.data }
}

#[cfg(test)]
mod tests {
  use std::cell::Cell;

  use ribir_dev_helper::*;

  use super::*;
  use crate::{prelude::*, reset_test_env, test_helper::*};

  #[derive(Declare, Clone, Query, SingleChild)]
  struct OffsetBox {
    pub offset: Point,
    pub size: Size,
  }

  impl Render for OffsetBox {
    fn perform_layout(&self, mut clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
      clamp.max = clamp.max.min(self.size);
      let mut layouter = ctx.assert_single_child_layouter();
      layouter.perform_widget_layout(clamp);
      layouter.update_position(self.offset);
      self.size
    }
    #[inline]
    fn only_sized_by_parent(&self) -> bool { true }

    #[inline]
    fn paint(&self, _: &mut PaintingCtx) {}
  }

  #[test]
  fn fix_incorrect_relayout_root() {
    reset_test_env!();

    // Can't use layout info of dirty widget to detect if the ancestors path have
    // in relayout list. Because new widget insert by `DynWidget` not have layout
    // info, but its parent have.
    let child_box = Stateful::new(MockBox { size: Size::zero() });
    let root_layout_cnt = Stateful::new(0);
    let c_child_box = child_box.clone_writer();
    let c_root_layout_cnt = root_layout_cnt.clone_reader();
    let w = fn_widget! {
      @MockMulti {
        on_performed_layout: move |_| *$root_layout_cnt.write() += 1,
        @ { pipe!($child_box.size.is_empty()).map(move|b| if b {
            MockBox { size: Size::new(1., 1.) }.build(ctx!())
          } else {
            child_box.clone_writer().into_inner().build(ctx!())
          })
        }
      }
    };

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();
    assert_eq!(*c_root_layout_cnt.read(), 1);
    {
      c_child_box.write().size = Size::new(2., 2.);
    }
    wnd.draw_frame();
    assert_eq!(*c_root_layout_cnt.read(), 2);
  }

  #[test]
  fn layout_list_from_root_to_leaf() {
    reset_test_env!();

    let layout_order = Stateful::new(vec![]);
    let trigger = Stateful::new(Size::zero());
    let order = layout_order.clone_writer();
    let size = trigger.clone_watcher();
    let w = fn_widget! {
      @MockBox {
        size: pipe!(*$size),
        on_performed_layout: move |_| $order.write().push(1),
        @MockBox {
          size: pipe!(*$size),
          on_performed_layout: move |_| $order.write().push(2),
          @MockBox {
            size: pipe!(*$size),
            on_performed_layout: move |_| $order.write().push(3),
          }
        }
      }
    };

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();
    assert_eq!([3, 2, 1], &**layout_order.read());
    {
      *trigger.write() = Size::new(1., 1.);
    }
    wnd.draw_frame();
    assert_eq!([3, 2, 1, 3, 2, 1], &**layout_order.read());
  }

  #[test]
  fn relayout_size() {
    reset_test_env!();

    let trigger = Stateful::new(Size::zero());
    let size = trigger.clone_watcher();
    let w = fn_widget! {
      @OffsetBox {
        size: Size::new(100., 100.),
        offset: Point::new(50., 50.),
        @MockBox {
          size: Size::new(50., 50.),
          @MockBox { size: pipe!(*$size) }
        }
      }
    };

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();
    assert_layout_result_by_path!(
      wnd,
      { path = [0, 0], rect == ribir_geom::rect(50., 50., 50., 50.),}
    );
    assert_layout_result_by_path!(
      wnd,
      {path = [0, 0, 0], rect == ribir_geom::rect(0., 0., 0., 0.),}
    );

    {
      *trigger.write() = Size::new(10., 10.);
    }

    wnd.draw_frame();
    assert_layout_result_by_path!(
      wnd,
      {path = [0, 0], rect == ribir_geom::rect(50., 50., 50., 50.),}
    );
    assert_layout_result_by_path!(
      wnd,
      {path = [0, 0, 0], rect == ribir_geom::rect(0., 0., 10., 10.),}
    );
  }

  #[test]
  fn relayout_from_parent() {
    reset_test_env!();

    let trigger = Stateful::new(Size::zero());
    let cnt = Rc::new(Cell::new(0));
    let cnt2 = cnt.clone();
    let size = trigger.clone_watcher();
    let w = fn_widget! {
      @MockBox {
        size: Size::new(50., 50.),
        on_performed_layout: move |_| cnt2.set(cnt2.get() + 1),
        @MockBox { size: pipe!(*$size) }
      }
    };

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();
    assert_eq!(cnt.get(), 1);

    {
      *trigger.write() = Size::new(10., 10.);
    }
    wnd.draw_frame();
    assert_eq!(cnt.get(), 2);
  }

  #[test]
  fn layout_visit_prev_position() {
    reset_test_env!();

    #[derive(Declare, Query)]
    struct MockWidget {
      pos: Cell<Point>,
      size: Size,
    }

    impl Render for MockWidget {
      fn perform_layout(&self, _: BoxClamp, ctx: &mut LayoutCtx) -> Size {
        self.pos.set(ctx.box_pos().unwrap_or_default());
        self.size
      }
      #[inline]
      fn only_sized_by_parent(&self) -> bool { true }

      #[inline]
      fn paint(&self, _: &mut PaintingCtx) {}
    }

    let pos = Rc::new(Cell::new(Point::zero()));
    let pos2 = pos.clone();
    let trigger = Stateful::new(Size::zero());
    let size = trigger.clone_watcher();
    let w = fn_widget! {
      let w = @MockWidget {
        size: pipe!(*$size),
        pos: Cell::new(Point::zero()),
      };
      @MockMulti {
        @MockBox{ size: Size::new(50., 50.) }
        @$w {
          on_performed_layout: move |_| pos2.set($w.pos.get())
        }
      }
    };
    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();

    *trigger.write() = Size::new(1., 1.);
    wnd.draw_frame();
    assert_eq!(pos.get(), Point::new(50., 0.));
  }
}
