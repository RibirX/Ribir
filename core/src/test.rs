#![cfg(test)]
pub mod embed_post;
pub mod key_embed_post;
pub mod recursive_row;
use widget::window::MockRender;

use crate::prelude::*;

// return the flex box rect, and rect of its children.
pub fn widget_and_its_children_box_rect<W: AttachAttr>(
  root: W,
  window_size: Size,
) -> (Rect, Vec<Rect>) {
  let mut wnd = window::Window::without_render(root, window_size);
  wnd.render_ready();

  root_and_children_rect(&mut wnd)
}

pub fn root_and_children_rect(wnd: &mut window::Window<MockRender>) -> (Rect, Vec<Rect>) {
  let r_tree = wnd.render_tree();
  let root = r_tree.root().unwrap();
  let rect = root.layout_box_rect(&*r_tree).unwrap();
  let children_box_rect = root
    .children(&*r_tree)
    .map(|rid| rid.layout_box_rect(&*r_tree).unwrap())
    .collect();

  (rect, children_box_rect)
}
