pub mod embed_post;
pub mod key_embed_post;
pub mod recursive_row;

use crate::prelude::*;

// return the flex box rect, and rect of its children.
pub fn widget_and_its_children_box_rect(root: BoxedWidget, window_size: Size) -> (Rect, Vec<Rect>) {
  let mut wnd = Window::without_render(root, window_size);
  wnd.render_ready();

  root_and_children_rect(&mut wnd)
}

pub fn root_and_children_rect(wnd: &Window) -> (Rect, Vec<Rect>) {
  let ctx = wnd.context();
  let tree = &ctx.widget_tree;
  let layout = &ctx.layout_store;
  let r_root = tree.root().render_widget(tree).unwrap();
  let rect = layout.layout_box_rect(r_root).unwrap();
  let children_box_rect = r_root
    .children(tree)
    .map(|c| {
      let rid = c.render_widget(tree).unwrap();
      layout.layout_box_rect(rid).unwrap()
    })
    .collect();

  (rect, children_box_rect)
}
