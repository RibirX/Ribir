use std::{
  cmp::Reverse,
  collections::HashMap,
  sync::{Arc, RwLock},
};

use text::{font_db::FontDB, shaper::TextShaper, TextReorder, TypographyStore};

use crate::prelude::{widget_tree::WidgetTree, Point, Rect, Size, WidgetId};

use super::LayoutCtx;

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
pub struct LayoutStore {
  infos: HashMap<WidgetId, BoxLayout, ahash::RandomState>,
}

impl LayoutStore {
  /// Remove the layout info of the `wid`
  pub(crate) fn remove(&mut self, id: WidgetId) -> Option<BoxLayout> { self.infos.remove(&id) }

  /// Return the box rect of layout result of render widget, if it's a
  /// combination widget return None.
  pub(crate) fn layout_box_rect(&self, id: WidgetId) -> Option<Rect> {
    self.infos.get(&id).and_then(|info| info.rect)
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

  /// Do the work of computing the layout for all node which need, Return if any
  /// node has really computing the layout.
  pub(crate) fn layout(
    &mut self,
    win_size: Size,
    tree: &WidgetTree,
    shaper: &TextShaper,
    text_reorder: &TextReorder,
    typography_store: &TypographyStore,
    font_db: &Arc<RwLock<FontDB>>,
  ) -> bool {
    let mut performed_layout = false;

    loop {
      if let Some(needs_layout) = self.layout_list(tree) {
        performed_layout = performed_layout || !needs_layout.is_empty();
        needs_layout.iter().for_each(|wid| {
          let clamp = BoxClamp { min: Size::zero(), max: win_size };
          self.perform_layout(
            *wid,
            clamp,
            tree,
            shaper,
            text_reorder,
            typography_store,
            font_db,
          );
        });
      } else {
        break;
      }
    }
    performed_layout
  }

  /// Compute layout of the render widget `id`, and store its result in the
  /// store.
  ///
  /// # Safety
  /// Panic if `id` is not a render widget.
  pub(crate) fn perform_layout(
    &mut self,
    id: WidgetId,
    out_clamp: BoxClamp,
    tree: &WidgetTree,
    shaper: &TextShaper,
    text_reorder: &TextReorder,
    typography_store: &TypographyStore,
    font_db: &Arc<RwLock<FontDB>>,
  ) -> Size {
    self
      .infos
      .get(&id)
      .and_then(|BoxLayout { clamp, rect }| {
        rect.and_then(|r| (&out_clamp == clamp).then(|| r.size))
      })
      .unwrap_or_else(|| {
        // children's position is decided by parent, no matter itself relayout or not.
        // here before parent perform layout, we reset children's position.
        // todo: why?
        id.children(&tree).for_each(|child| {
          self
            .layout_info_or_default(child)
            .rect
            .as_mut()
            .map(|mut rc| rc.origin = Point::default());
        });
        let layout = id.assert_get(tree);
        let size = layout.perform_layout(
          out_clamp,
          &mut LayoutCtx {
            id,
            tree,
            layout_store: self,
            shaper,
            text_reorder,
            typography_store,
            font_db,
          },
        );
        let size = out_clamp.clamp(size);
        let info = self.layout_info_or_default(id);
        info.clamp = out_clamp;
        info.rect.get_or_insert_with(Rect::zero).size = size;
        size
      })
  }

  // pub(crate) fn need_layout(&self) -> bool { !self.needs_layout.is_empty() }

  /// return a mutable reference of the layout info  of `id`, if it's not exist
  /// insert a default value before return
  fn layout_info_or_default(&mut self, id: WidgetId) -> &mut BoxLayout {
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
}

impl Default for BoxClamp {
  fn default() -> Self {
    Self {
      min: Size::new(0., 0.),
      max: Size::new(f32::INFINITY, f32::INFINITY),
    }
  }
}
