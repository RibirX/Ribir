use painter::ZERO_SIZE;

use super::{widget_id::split_arena, DirtySet, WidgetId, WidgetTree};
use crate::{
  builtin_widgets::PerformedLayoutListener,
  context::{AppContext, LayoutCtx},
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

/// render object's layout box, the information about layout, including box
/// size, box position, and the clamp of render object layout.
#[derive(Debug, Default)]
pub struct BoxLayout {
  /// Box bound is the bound of the layout can be place. it will be set after
  /// render object computing its layout. It's passed by render object's parent.
  pub clamp: BoxClamp,
  /// The position and size render object to place, relative to its parent
  /// coordinate. Some value after the relative render object has been layout,
  /// otherwise is none value.
  pub rect: Option<Rect>,
}

/// Store the render object's place relative to parent coordinate and the
/// clamp passed from parent.
#[derive(Default)]
pub(crate) struct LayoutStore {
  data: HashMap<WidgetId, BoxLayout, ahash::RandomState>,
  performed: Vec<WidgetId>,
}

impl LayoutStore {
  /// Remove the layout info of the `wid`
  pub(crate) fn force_layout(&mut self, id: WidgetId) -> Option<BoxLayout> { self.remove(id) }

  pub(crate) fn remove(&mut self, id: WidgetId) -> Option<BoxLayout> { self.data.remove(&id) }

  /// Return the box rect of layout result of render widget, if it's a
  /// combination widget return None.
  pub(crate) fn layout_box_rect(&self, id: WidgetId) -> Option<Rect> {
    self.layout_info(id).and_then(|info| info.rect)
  }

  pub(crate) fn layout_info(&self, id: WidgetId) -> Option<&BoxLayout> { self.data.get(&id) }

  /// Return the mut reference of box rect of the layout widget(create if not
  /// have), notice
  ///
  /// Caller should guarantee `id` is a layout widget.
  pub(crate) fn layout_box_rect_mut(&mut self, id: WidgetId) -> &mut Rect {
    self
      .layout_info_or_default(id)
      .rect
      .get_or_insert_with(Rect::zero)
  }

  // pub(crate) fn need_layout(&self) -> bool { !self.needs_layout.is_empty() }

  /// return a mutable reference of the layout info  of `id`, if it's not exist
  /// insert a default value before return
  pub(crate) fn layout_info_or_default(&mut self, id: WidgetId) -> &mut BoxLayout {
    self.data.entry(id).or_insert_with(BoxLayout::default)
  }

  pub(crate) fn map_to_parent(&self, id: WidgetId, pos: Point) -> Point {
    // todo: should effect by transform widget.
    self
      .layout_box_rect(id)
      .map_or(pos, |rect| pos + rect.min().to_vector())
  }

  pub(crate) fn map_from_parent(&self, id: WidgetId, pos: Point) -> Point {
    self
      .layout_box_rect(id)
      .map_or(pos, |rect| pos - rect.min().to_vector())
    // todo: should effect by transform widget.
  }

  pub(crate) fn map_to_global(&self, pos: Point, widget: WidgetId, arena: &TreeArena) -> Point {
    widget
      .ancestors(arena)
      .fold(pos, |pos, p| self.map_to_parent(p, pos))
  }

  pub(crate) fn map_from_global(&self, pos: Point, widget: WidgetId, arena: &TreeArena) -> Point {
    let stack = widget.ancestors(arena).collect::<Vec<_>>();
    stack
      .iter()
      .rev()
      .fold(pos, |pos, p| self.map_from_parent(*p, pos))
  }

  /// Compute layout of the render widget `id`, and store its result in the
  /// store.
  pub(crate) fn perform_widget_layout(
    &mut self,
    id: WidgetId,
    out_clamp: BoxClamp,
    arena: &mut TreeArena,
    app_ctx: &AppContext,
    dirty_set: &DirtySet,
  ) -> Size {
    self
      .layout_info(id)
      .and_then(|BoxLayout { clamp, rect }| {
        rect.and_then(|r| (&out_clamp == clamp).then(|| r.size))
      })
      .unwrap_or_else(|| {
        // Safety: `LayoutCtx` will never mutable access widget tree, so split a node is
        // safe.
        let (arena1, arena2) = unsafe { split_arena(arena) };

        let layout = id.assert_get(arena1);
        let mut ctx = LayoutCtx {
          id,
          arena: arena2,
          store: self,
          app_ctx,
          dirty_set,
        };
        let size = layout.perform_layout(out_clamp, &mut ctx);
        let size = out_clamp.clamp(size);
        let info = self.layout_info_or_default(id);
        info.clamp = out_clamp;
        info.rect.get_or_insert_with(Rect::zero).size = size;

        id.assert_get(arena).query_all_type(
          |_: &PerformedLayoutListener| {
            self.performed.push(id);
            false
          },
          QueryOrder::OutsideFirst,
        );
        size
      })
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
        info.rect.take();
      }

      // All ancestors of this render widget should relayout until the one which only
      // sized by parent.
      for p in id.0.ancestors(&self.arena).skip(1).map(WidgetId) {
        if self.store.layout_box_rect(p).is_none() {
          break;
        }
        let r = self.arena.get(p.0).unwrap().get();
        if r.only_sized_by_parent() {
          break;
        }

        if let Some(info) = self.store.data.get_mut(&p) {
          info.rect.take();
        }

        relayout_root = p
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
  use super::*;
  use crate::{prelude::*, test::*};

  #[test]
  fn fix_incorrect_relayout_root() {
    // Can't use layout info of dirty widget to detect if the ancestors path have
    // in relayout list. Because new widget insert by `DynWidget` not have layout
    // info, but its parent have.
    let child_box = MockBox { size: Size::zero() }.into_stateful();
    let root_layout_cnt = 0.into_stateful();
    let w = widget! {
      track {
        child_box: child_box.clone(),
        root_layout_cnt: root_layout_cnt.clone(),
      }
      MockMulti {
        performed_layout: move |_| *root_layout_cnt += 1,
        DynWidget {
          dyns: if child_box.size.is_empty() {
            MockBox { size: Size::new(1., 1.) }.into_widget()
          } else {
            child_box.clone_stateful().into_widget()
          }
        }
      }
    };

    let mut tree = WidgetTree::new(w, <_>::default());
    tree.layout(Size::zero());
    assert_eq!(*root_layout_cnt.raw_ref(), 1);
    {
      child_box.state_ref().size = Size::new(2., 2.);
    }
    tree.layout(Size::zero());
    assert_eq!(*root_layout_cnt.raw_ref(), 2);
  }

  #[test]
  fn layout_list_from_root_to_leaf() {
    let layout_order = vec![].into_stateful();
    let trigger = Size::zero().into_stateful();
    let w = widget! {
      track {
        layout_order: layout_order.clone(),
        trigger: trigger.clone()
      }
      MockBox {
        size: *trigger,
        performed_layout: move |_| layout_order.push(1),
        MockBox {
          size: *trigger,
          performed_layout: move |_| layout_order.push(2),
          MockBox {
            size: *trigger,
            performed_layout: move |_| layout_order.push(3),
          }
        }
      }
    };

    let mut wnd = Window::default_mock(w, None);
    wnd.draw_frame();
    assert_eq!([3, 2, 1], &**layout_order.raw_ref());
    {
      *trigger.state_ref() = Size::new(1., 1.);
    }
    wnd.draw_frame();
    assert_eq!([3, 2, 1, 3, 2, 1], &**layout_order.raw_ref());
  }
}
