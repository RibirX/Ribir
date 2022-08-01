use std::{cmp::Reverse, collections::HashMap};

use crate::prelude::{widget_tree::WidgetTree, Rect, Size, WidgetId, INFINITY_SIZE};

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
  infos: HashMap<WidgetId, BoxLayout, ahash::RandomState>,
}

impl LayoutStore {
  /// Remove the layout info of the `wid`
  pub(crate) fn remove(&mut self, id: WidgetId) -> Option<BoxLayout> { self.infos.remove(&id) }

  /// Return the box rect of layout result of render widget, if it's a
  /// combination widget return None.
  pub(crate) fn layout_box_rect(&self, id: WidgetId) -> Option<Rect> {
    self.layout_info(id).and_then(|info| info.rect)
  }

  pub(crate) fn layout_info(&self, id: WidgetId) -> Option<&BoxLayout> { self.infos.get(&id) }

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
    self.infos.entry(id).or_insert_with(BoxLayout::default)
  }

  pub(crate) fn layout_list(&mut self, tree: &WidgetTree) -> Option<Vec<WidgetId>> {
    if tree.state_changed.borrow().is_empty() {
      return None;
    }

    let mut needs_layout = vec![];
    tree
      .state_changed
      .borrow_mut()
      .drain()
      .filter(|id| !id.is_dropped(tree))
      .for_each(|id| {
        let mut relayout_root = id;
        if self.layout_box_rect(id).is_some() {
          self.remove(id);
          // All ancestors of this render object should relayout until the one which only
          // sized by parent.
          id.ancestors(tree).skip(1).all(|id| {
            let r = id.assert_get(tree);
            let sized_by_parent = r.only_sized_by_parent();
            if !sized_by_parent {
              self.remove(id);
              relayout_root = id;
            }

            !sized_by_parent
          });
        }
        needs_layout.push(relayout_root);
      });

    needs_layout.sort_by_cached_key(|w| Reverse(w.ancestors(tree).count()));
    Some(needs_layout)
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
}

impl Default for BoxClamp {
  fn default() -> Self {
    Self {
      min: Size::new(0., 0.),
      max: Size::new(f32::INFINITY, f32::INFINITY),
    }
  }
}
