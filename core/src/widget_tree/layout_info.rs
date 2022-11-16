use painter::ZERO_SIZE;

use super::{WidgetId, WidgetTree};
use crate::prelude::{Point, Rect, Size, INFINITY_SIZE};
use std::cmp::Reverse;

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
  /// Remove the layout info of the `wid`
  pub(crate) fn force_layout(&mut self, id: WidgetId) -> Option<BoxLayout> {
    self.layout_store.remove(&id)
  }

  /// Return the box rect of layout result of render widget, if it's a
  /// combination widget return None.
  pub(crate) fn layout_box_rect(&self, id: WidgetId) -> Option<Rect> {
    self.layout_info(id).and_then(|info| info.rect)
  }

  pub(crate) fn layout_info(&self, id: WidgetId) -> Option<&BoxLayout> {
    self.layout_store.get(&id)
  }

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
    self
      .layout_store
      .entry(id)
      .or_insert_with(BoxLayout::default)
  }

  pub(crate) fn layout_list(&mut self) -> Option<Vec<WidgetId>> {
    if self.state_changed.borrow().is_empty() {
      return None;
    }

    let mut needs_layout = vec![];

    let dirty_widgets = {
      let mut state_changed = self.state_changed.borrow_mut();
      let dirty_widgets = state_changed.clone();
      state_changed.clear();
      dirty_widgets
    };

    for id in dirty_widgets.iter() {
      if id.is_dropped(self) {
        continue;
      }

      let mut relayout_root = *id;
      if let Some(info) = self.layout_store.get_mut(id) {
        info.rect.take();
      }

      // All ancestors of this render widget should relayout until the one which only
      // sized by parent.
      for p in id.0.ancestors(&self.arena).skip(1).map(WidgetId) {
        if self.layout_box_rect(p).is_none() {
          break;
        }
        let r = self.arena.get(p.0).unwrap().get();
        if r.only_sized_by_parent() {
          break;
        }

        if let Some(info) = self.layout_store.get_mut(&p) {
          info.rect.take();
        }

        relayout_root = p
      }
      needs_layout.push(relayout_root);
    }

    (!needs_layout.is_empty()).then(|| {
      needs_layout.sort_by_cached_key(|w| Reverse(w.ancestors(self).count()));
      needs_layout
    })
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

  pub(crate) fn map_to_global(&self, id: WidgetId, pos: Point) -> Point {
    id.ancestors(self)
      .fold(pos, |pos, p| self.map_to_parent(p, pos))
  }

  pub(crate) fn map_from_global(&self, id: WidgetId, pos: Point) -> Point {
    let stack = id.ancestors(self).collect::<Vec<_>>();
    stack
      .iter()
      .rev()
      .fold(pos, |pos, p| self.map_from_parent(*p, pos))
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
    // in relayout list. Because new widget insert by `ExprWidget` not have layout
    // info, but its parent have.
    let child_box = MockBox { size: Size::zero() }.into_stateful();
    let w = widget! {
      track { child_box: child_box.clone() }
      MockMulti {
        ExprWidget {
          expr: if child_box.size.is_empty() {
            MockBox { size: Size::new(1., 1.) }.into_widget()
          } else {
            child_box.clone_stateful().into_widget()
          }
        }
      }
    };

    let mut tree = WidgetTree::new(w, <_>::default());
    tree.layout(Size::zero());
    {
      child_box.state_ref().size = Size::new(2., 2.);
    }
    tree.tree_repair();
    assert_eq!(tree.layout_list().unwrap().first(), Some(&tree.root()));
  }
}
