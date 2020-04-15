use crate::{prelude::*, render::render_tree::*, widget::widget_tree::*};
use indextree::*;
use std::collections::HashSet;

#[derive(Default)]
pub struct Application<'a> {
  pub(crate) render_tree: RenderTree,
  pub(crate) widget_tree: WidgetTree<'a>,

  dirty_layouts: HashSet<NodeId>,
  dirty_layout_roots: HashSet<NodeId>,
}

impl<'a> Application<'a> {
  #[inline]
  pub fn new() -> Application<'a> { Default::default() }

  pub fn run(mut self, w: Widget<'a>) {
    self.widget_tree.set_root(w, &mut self.render_tree);

    todo!(
      "
      1. start a event loop to handle event.
      2. run layout and paint for it.
    "
    );

    let Self {
      widget_tree,
      render_tree,
      ..
    } = self;

    widget_tree.repair(&mut render_tree);
  }
}

#[cfg(test)]
mod test {
  extern crate test;

  // fn assert_root_bound(app: &mut Application, bound: Option<Size>) {
  //   let root = app.r_arena.get_mut(app.render_tree.unwrap()).unwrap();
  //   let render_box = root.get_mut().to_render_box().unwrap();
  //   assert_eq!(render_box.bound(), bound);
  // }

  // fn layout_app(app: &mut Application) {
  //   let mut_ptr = &mut app.r_arena as *mut Arena<Box<(dyn RenderObject + Send
  // + Sync)>>;   let mut ctx = RenderCtx::new(&mut app.r_arena, &mut
  // app.dirty_layouts, &mut app.dirty_layout_roots);   unsafe {
  //       let root =
  // mut_ptr.as_mut().unwrap().get_mut(app.render_tree.unwrap()).unwrap();
  //       root.get_mut().perform_layout(app.render_tree.unwrap(), &mut ctx);
  //   }
  // }

  // fn mark_dirty(app: &mut Application, node_id: NodeId) {
  //   let mut_ptr = &mut app.r_arena as *mut Arena<Box<(dyn RenderObject + Send
  // + Sync)>>;   let mut ctx = RenderCtx::new(&mut app.r_arena, &mut
  // app.dirty_layouts, &mut app.dirty_layout_roots);

  //   unsafe {
  //      mut_ptr
  //       .as_mut()
  //       .unwrap()
  //       .get_mut(node_id)
  //       .unwrap()
  //       .get_mut()
  //       .mark_dirty(node_id, &mut ctx);
  //   }
  // }

  // #[test]
  // fn test_layout() {
  //   let post = EmbedPost {
  //     title: "Simple demo",
  //     author: "Adoo",
  //     content: "Recursive 5 times",
  //     level: 5,
  //   };
  //   let mut app = Application::new();
  //   app.inflate(post.to_widget());
  //   app.construct_render_tree(app.widget_tree.unwrap());

  //   let root_id = app.render_tree.unwrap();
  //   mark_dirty(&mut app, root_id);
  //   layout_app(&mut app);
  //   assert_root_bound(
  //     &mut app,
  //     Some(Size {
  //       width: 192,
  //       height: 1,
  //     }),
  //   );

  //   let last_child_id = app
  //     .r_arena
  //     .get(app.render_tree.unwrap())
  //     .unwrap()
  //     .last_child()
  //     .unwrap();
  //   mark_dirty(&mut app, last_child_id);
  //   assert_eq!(app.dirty_layouts.contains(&root_id), true);

  //   layout_app(&mut app);
  //   assert_eq!(app.dirty_layouts.contains(&root_id), false);
  //   assert_root_bound(
  //     &mut app,
  //     Some(Size {
  //       width: 192,
  //       height: 1,
  //     }),
  //   );
  // }
}
