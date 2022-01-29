use std::{
  cmp::Reverse,
  collections::{BinaryHeap, HashMap},
};

use text::shaper::TextShaper;

use crate::prelude::{
  widget_tree::{WidgetNode, WidgetTree},
  Rect, Size, WidgetId,
};

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
  /// root of sub tree which needed to perform layout, store as min-head by the
  /// node's depth.
  needs_layout: BinaryHeap<Reverse<(usize, WidgetId)>>,

  /// Store the layout info layout widget.
  infos: HashMap<WidgetId, BoxLayout>,
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

  /// Mark layout widget need relayout, caller guarantee `id` is a layout
  /// widget.
  pub(crate) fn mark_needs_layout(&mut self, id: WidgetId, tree: &WidgetTree) {
    let mut relayout_root = id;
    if self.layout_box_rect(id).is_some() {
      self.remove(id);
      // All ancestors of this render object should relayout until the one which only
      // sized by parent.
      id.ancestors(tree)
        .skip(1)
        .all(|id| match id.assert_get(tree) {
          WidgetNode::Combination(_) => true,
          WidgetNode::Render(r) => {
            let sized_by_parent = r.only_sized_by_parent();
            if !sized_by_parent {
              self.remove(id);
              relayout_root = id;
            }

            !sized_by_parent
          }
        });
    }
    self.needs_layout.push(Reverse((
      relayout_root.ancestors(tree).count(),
      relayout_root,
    )));
  }

  pub(crate) fn is_dirty(&self, tree: &WidgetTree) -> bool {
    self
      .needs_layout
      .iter()
      .any(|Reverse((_, id))| !id.is_dropped(tree))
  }

  /// Do the work of computing the layout for all node which need, Return if any
  /// node has really computing the layout.
  pub(crate) fn layout(
    &mut self,
    win_size: Size,
    tree: &WidgetTree,
    shaper: &mut TextShaper,
  ) -> bool {
    let mut performed_layout = false;
    loop {
      if self.needs_layout.is_empty() {
        break;
      }
      let needs_layout = self.take_needs_layout();

      needs_layout.iter().for_each(|Reverse((_, wid))| {
        if !wid.is_dropped(tree) {
          performed_layout = true;
          let clamp = BoxClamp { min: Size::zero(), max: win_size };
          self.perform_layout(*wid, clamp, tree, shaper);
        }
      });
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
    shaper: &mut TextShaper,
  ) -> Size {
    self
      .infos
      .get(&id)
      .and_then(|BoxLayout { clamp, rect }| {
        rect.and_then(|r| (&out_clamp == clamp).then(|| r.size))
      })
      .unwrap_or_else(|| {
        let layout = match id.assert_get(tree) {
          WidgetNode::Combination(_) => unreachable!(),
          WidgetNode::Render(r) => r,
        };
        let size = layout.perform_layout(
          out_clamp,
          &mut LayoutCtx { id, tree, layout_store: self, shaper },
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

  fn take_needs_layout(&mut self) -> BinaryHeap<Reverse<(usize, WidgetId)>> {
    let ret = self.needs_layout.clone();
    self.needs_layout.clear();
    ret
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
