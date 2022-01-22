use std::{
  cmp::Reverse,
  collections::{BinaryHeap, HashMap},
};

use crate::prelude::{render_tree::RenderTree, BoxClamp, Rect, RenderId};

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
  needs_layout: BinaryHeap<Reverse<(usize, RenderId)>>,

  infos: HashMap<RenderId, BoxLayout>,
}

impl LayoutStore {
  /// return a mutable reference of the layout info  of `rid`, if it's not exist
  /// insert a default value before return
  pub fn layout_info_or_default(&mut self, rid: RenderId) -> &mut BoxLayout {
    self.infos.entry(rid).or_insert_with(BoxLayout::default)
  }

  /// Remove the layout info of the `rid`
  #[inline]
  pub fn remove(&mut self, rid: RenderId) -> Option<BoxLayout> { self.infos.remove(&rid) }

  pub fn layout_info(&self, rid: RenderId) -> Option<&BoxLayout> { self.infos.get(&rid) }

  pub fn layout_clamp(&self, rid: RenderId) -> Option<BoxClamp> {
    self.infos.get(&rid).map(|info| info.clamp)
  }

  pub fn layout_box_rect(&self, rid: RenderId) -> Option<Rect> {
    self.infos.get(&rid).and_then(|info| info.rect)
  }

  pub fn layout_clamp_mut(&mut self, rid: RenderId) -> &mut BoxClamp {
    &mut self.layout_info_or_default(rid).clamp
  }

  pub fn layout_box_rect_mut(&mut self, rid: RenderId) -> &mut Rect {
    self
      .layout_info_or_default(rid)
      .rect
      .get_or_insert_with(Rect::zero)
  }

  pub fn has_need_layout(&self) -> bool { !self.needs_layout.is_empty() }

  pub fn take_needs_layout(&mut self) -> BinaryHeap<Reverse<(usize, RenderId)>> {
    let ret = self.needs_layout.clone();
    self.needs_layout.clear();
    ret
  }

  pub fn mark_needs_layout(&mut self, rid: RenderId, r_tree: &RenderTree) {
    let mut relayout_root = rid;
    if self.layout_box_rect(rid).is_some() {
      // All ancestors of this render object should relayout until the one which only
      // sized by parent.
      rid.ancestors(r_tree).all(|rid| {
        self.remove(rid);
        relayout_root = rid;

        let sized_by_parent = rid
          .get(r_tree)
          .map_or(false, |node| node.only_sized_by_parent());

        !sized_by_parent
      });
    }
    self
      .needs_layout
      .push(Reverse((rid.ancestors(r_tree).count(), rid)));
  }
}