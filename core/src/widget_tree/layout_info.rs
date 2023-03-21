use ribir_painter::ZERO_SIZE;

use super::{widget_id::split_arena, DirtySet, WidgetId, WidgetTree};
use crate::{
  builtin_widgets::PerformedLayoutListener,
  context::{LayoutCtx, WindowCtx},
  prelude::{Point, Rect, Size, INFINITY_SIZE},
  widget::{QueryOrder, TreeArena},
};
use std::{cmp::Reverse, collections::HashMap};

/// boundary limit of the render object's layout
#[derive(Debug, Clone, PartialEq, Copy)]
pub struct BoxClamp {
  pub min: Size,
  pub max: Size,
}

impl BoxClamp {
  /// clamp use to expand the width to max
  pub const EXPAND_X: BoxClamp = BoxClamp {
    min: Size::new(f32::INFINITY, 0.),
    max: Size::new(f32::INFINITY, f32::INFINITY),
  };

  /// clamp use to expand the height to max
  pub const EXPAND_Y: BoxClamp = BoxClamp {
    min: Size::new(0., f32::INFINITY),
    max: Size::new(f32::INFINITY, f32::INFINITY),
  };

  /// clamp use to expand the size to max
  pub const EXPAND_BOTH: BoxClamp = BoxClamp {
    min: Size::new(f32::INFINITY, f32::INFINITY),
    max: Size::new(f32::INFINITY, f32::INFINITY),
  };

  /// clamp use fixed width and unfixed height
  pub fn fixed_width(width: f32) -> Self {
    BoxClamp {
      min: Size::new(width, 0.),
      max: Size::new(width, f32::INFINITY),
    }
  }

  /// clamp use fixed height and unfixed width
  pub fn fixed_height(height: f32) -> Self {
    BoxClamp {
      min: Size::new(0., height),
      max: Size::new(f32::INFINITY, height),
    }
  }
}

/// render object's layout box, the information about layout, including box
/// size, box position, and the clamp of render object layout.
#[derive(Debug, Default)]
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
  performed: Vec<WidgetId>,
}

pub struct Layouter<'a> {
  pub(crate) wid: WidgetId,
  pub(crate) arena: &'a mut TreeArena,
  pub(crate) store: &'a mut LayoutStore,
  pub(crate) wnd_ctx: &'a WindowCtx,
  pub(crate) dirty_set: &'a DirtySet,
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

  // pub(crate) fn need_layout(&self) -> bool { !self.needs_layout.is_empty() }

  /// return a mutable reference of the layout info  of `id`, if it's not exist
  /// insert a default value before return
  pub(crate) fn layout_info_or_default(&mut self, id: WidgetId) -> &mut LayoutInfo {
    self.data.entry(id).or_insert_with(LayoutInfo::default)
  }

  pub(crate) fn map_to_parent(&self, id: WidgetId, pos: Point, arena: &TreeArena) -> Point {
    self.layout_box_position(id).map_or(pos, |offset| {
      let pos = id
        .assert_get(arena)
        .get_transform()
        .map_or(pos, |t| t.transform_point(pos));
      pos + offset.to_vector()
    })
  }

  pub(crate) fn map_from_parent(&self, id: WidgetId, pos: Point, arena: &TreeArena) -> Point {
    self.layout_box_position(id).map_or(pos, |offset| {
      let pos = pos - offset.to_vector();
      id.assert_get(arena)
        .get_transform()
        .map_or(pos, |t| t.inverse().map_or(pos, |t| t.transform_point(pos)))
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

  pub(crate) fn take_performed(&mut self) -> Vec<WidgetId> {
    let performed = self.performed.clone();
    self.performed.clear();
    performed
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
  /// perform layout of the widget this `ChildLayouter` represent, return the
  /// size result after layout
  pub fn perform_widget_layout(&mut self, clamp: BoxClamp) -> Size {
    let Self {
      wid: child,
      arena,
      store,
      wnd_ctx,
      dirty_set,
    } = self;

    store
      .layout_info(*child)
      .filter(|info| clamp == info.clamp)
      .and_then(|info| info.size)
      .unwrap_or_else(|| {
        // Safety: `arena1` and `arena2` access different part of `arena`;
        let (arena1, arena2) = unsafe { split_arena(arena) };

        let layout = child.assert_get(arena1);
        let mut ctx = LayoutCtx {
          id: *child,
          arena: arena2,
          store,
          wnd_ctx,
          dirty_set,
        };
        let size = layout.perform_layout(clamp, &mut ctx);
        let size = clamp.clamp(size);
        let info = store.layout_info_or_default(*child);
        info.clamp = clamp;
        info.size = Some(size);

        layout.query_all_type(
          |_: &PerformedLayoutListener| {
            store.performed.push(*child);
            false
          },
          QueryOrder::OutsideFirst,
        );
        size
      })
  }

  /// Get layouter of the next sibling of this layouter, panic if self is not
  /// performed layout.
  pub fn into_next_sibling(self) -> Option<Self> {
    assert!(
      self.layout_rect().is_some(),
      "Before try to layout next sibling, self must performed layout."
    );
    self
      .wid
      .next_sibling(self.arena)
      .map(move |sibling| self.into_new_layouter(sibling))
  }

  /// Return layouter of the first child of this widget.
  pub fn into_first_child_layouter(self) -> Option<Self> {
    self
      .wid
      .first_child(self.arena)
      .map(move |wid| self.into_new_layouter(wid))
  }

  /// Return the rect of this layouter if it had performed layout.
  #[inline]
  pub fn layout_rect(&self) -> Option<Rect> {
    self
      .store
      .layout_info(self.wid)
      .and_then(|info| info.size.map(|size| Rect::new(info.pos, size)))
  }

  /// Return the position of this layouter if it had performed layout.
  #[inline]
  pub fn layout_pos(&self) -> Option<Point> {
    self.store.layout_info(self.wid).map(|info| info.pos)
  }

  /// Return the size of this layouter if it had performed layout.
  #[inline]
  pub fn layout_size(&self) -> Option<Size> {
    self.store.layout_info(self.wid).and_then(|info| info.size)
  }

  /// Update the position of the child render object should place. Relative to
  /// parent.
  #[inline]
  pub fn update_position(&mut self, pos: Point) {
    self.store.layout_info_or_default(self.wid).pos = pos;
  }

  /// Update the size of layout widget. Use this method to directly change the
  /// size of a widget, in most cast you needn't call this method, use clamp to
  /// limit the child size is enough. Use this method only it you know what you
  /// are doing.
  #[inline]
  pub fn update_size(&mut self, child: WidgetId, size: Size) {
    self.store.layout_info_or_default(child).size = Some(size);
  }

  #[inline]
  pub fn query_widget_type<W: 'static>(&self, callback: impl FnOnce(&W)) {
    self
      .wid
      .assert_get(self.arena)
      .query_on_first_type(QueryOrder::OutsideFirst, callback);
  }

  fn into_new_layouter(mut self, wid: WidgetId) -> Self {
    self.wid = wid;
    self
  }

  /// reset the child layout position to Point::zero()
  pub fn reset_children_position(&mut self) {
    let Self { wid, arena, store, .. } = self;
    wid.children(arena).for_each(move |id| {
      store.layout_info_or_default(id).pos = Point::zero();
    });
  }
}

impl WidgetTree {
  pub(crate) fn layout_list(&mut self) -> Option<Vec<WidgetId>> {
    if self.dirty_set.borrow().is_empty() {
      return None;
    }

    let mut needs_layout = vec![];

    let dirty_widgets = {
      let mut state_changed = self.dirty_set.borrow_mut();
      let dirty_widgets = state_changed.clone();
      state_changed.clear();
      dirty_widgets
    };

    for id in dirty_widgets.iter() {
      if id.is_dropped(&self.arena) {
        continue;
      }

      let mut relayout_root = *id;
      if let Some(info) = self.store.data.get_mut(id) {
        info.size.take();
      }

      // All ancestors of this render widget should relayout until the one which only
      // sized by parent.
      for p in id.0.ancestors(&self.arena).skip(1).map(WidgetId) {
        if self.store.layout_box_size(p).is_none() {
          break;
        }

        relayout_root = p;
        if let Some(info) = self.store.data.get_mut(&p) {
          info.size.take();
        }

        let r = self.arena.get(p.0).unwrap().get();
        if r.only_sized_by_parent() {
          break;
        }
      }
      needs_layout.push(relayout_root);
    }

    (!needs_layout.is_empty()).then(|| {
      needs_layout.sort_by_cached_key(|w| Reverse(w.ancestors(&self.arena).count()));
      needs_layout
    })
  }
}

impl Default for BoxClamp {
  fn default() -> Self {
    Self {
      min: Size::new(0., 0.),
      max: Size::new(f32::INFINITY, f32::INFINITY),
    }
  }
}

#[cfg(test)]
mod tests {
  use std::{cell::RefCell, rc::Rc};

  use super::*;
  use crate::{impl_query_self_only, prelude::*, test::*};

  #[derive(Declare, Clone, SingleChild)]
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

  impl Query for OffsetBox {
    impl_query_self_only!();
  }
  #[test]
  fn fix_incorrect_relayout_root() {
    // Can't use layout info of dirty widget to detect if the ancestors path have
    // in relayout list. Because new widget insert by `DynWidget` not have layout
    // info, but its parent have.
    let child_box = Stateful::new(MockBox { size: Size::zero() });
    let root_layout_cnt = Stateful::new(0);
    let w = widget! {
      states {
        child_box: child_box.clone(),
        root_layout_cnt: root_layout_cnt.clone(),
      }
      MockMulti {
        on_performed_layout: move |_| *root_layout_cnt += 1,
        DynWidget {
          dyns: if child_box.size.is_empty() {
            MockBox { size: Size::new(1., 1.) }.into_widget()
          } else {
            child_box.clone_stateful().into_widget()
          }
        }
      }
    };

    let app_ctx = <_>::default();
    let scheduler = FuturesLocalSchedulerPool::default().spawner();
    let mut tree = WidgetTree::new(w, WindowCtx::new(app_ctx, scheduler));
    tree.layout(Size::zero());
    assert_eq!(*root_layout_cnt.state_ref(), 1);
    {
      child_box.state_ref().size = Size::new(2., 2.);
    }
    tree.layout(Size::zero());
    assert_eq!(*root_layout_cnt.state_ref(), 2);
  }

  #[test]
  fn layout_list_from_root_to_leaf() {
    let layout_order = Stateful::new(vec![]);
    let trigger = Stateful::new(Size::zero());
    let w = widget! {
      states {
        layout_order: layout_order.clone(),
        trigger: trigger.clone()
      }
      MockBox {
        size: *trigger,
        on_performed_layout: move |_| layout_order.push(1),
        MockBox {
          size: *trigger,
          on_performed_layout: move |_| layout_order.push(2),
          MockBox {
            size: *trigger,
            on_performed_layout: move |_| layout_order.push(3),
          }
        }
      }
    };

    let mut wnd = Window::default_mock(w, None);
    wnd.draw_frame();
    assert_eq!([3, 2, 1], &**layout_order.state_ref());
    {
      *trigger.state_ref() = Size::new(1., 1.);
    }
    wnd.draw_frame();
    assert_eq!([3, 2, 1, 3, 2, 1], &**layout_order.state_ref());
  }

  #[test]
  fn relayout_size() {
    let trigger = Stateful::new(Size::zero());
    let w = widget! {
      states {trigger: trigger.clone()}
      OffsetBox {
        size: Size::new(100., 100.),
        offset: Point::new(50., 50.),
        MockBox {
          size: Size::new(50., 50.),
          MockBox {
            size: *trigger,
          }
        }
      }
    };

    let mut wnd = Window::default_mock(w, None);
    wnd.draw_frame();
    assert_layout_result(&wnd, &[0, 0], &ExpectRect::new(50., 50., 50., 50.));
    assert_layout_result(&wnd, &[0, 0, 0], &ExpectRect::new(0., 0., 0., 0.));

    {
      *trigger.state_ref() = Size::new(10., 10.);
    }

    wnd.draw_frame();
    assert_layout_result(&wnd, &[0, 0], &ExpectRect::new(50., 50., 50., 50.));
    assert_layout_result(&wnd, &[0, 0, 0], &ExpectRect::new(0., 0., 10., 10.));
  }

  #[test]
  fn relayout_from_parent() {
    let trigger = Stateful::new(Size::zero());
    let cnt = Rc::new(RefCell::new(0));
    let cnt2 = cnt.clone();
    let w = widget! {
      states {trigger: trigger.clone()}
      init { let cnt = cnt2; }
      MockBox {
        size: Size::new(50., 50.),
        on_performed_layout: move |_| *cnt.borrow_mut() += 1,
        MockBox {
          size: *trigger,
        }
      }
    };

    let mut wnd = Window::default_mock(w, None);
    wnd.draw_frame();
    assert_eq!(*cnt.borrow(), 1);

    {
      *trigger.state_ref() = Size::new(10., 10.);
    }
    wnd.draw_frame();
    assert_eq!(*cnt.borrow(), 2);
  }
}
