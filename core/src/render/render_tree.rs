use crate::{prelude::*, widget::widget_tree::*};
use indextree::*;
use std::collections::HashMap;

use super::layout_store::LayoutStore;

/// The id of the render object. Should not hold it.
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Hash)]
pub struct RenderId(NodeId);
pub enum RenderEdge {
  Start(RenderId),
  End(RenderId),
}

/// boundary limit of the render object's layout
#[derive(Debug, Clone, PartialEq, Copy)]
pub struct BoxClamp {
  pub min: Size,
  pub max: Size,
}

#[derive(Default)]
pub struct RenderTree {
  arena: Arena<Box<dyn RenderObject>>,
  root: Option<RenderId>,
  /// A hash map to mapping a render object in render tree to its corresponds
  /// render widget in widget tree.
  render_to_widget: HashMap<RenderId, WidgetId>,
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

impl RenderTree {
  #[inline]
  pub fn root(&self) -> Option<RenderId> { self.root }

  pub(crate) fn set_root(&mut self, root: RenderId) {
    debug_assert!(self.root.is_none());
    self.root = Some(root);
  }

  #[inline]
  pub(crate) fn new_node(&mut self, owner: WidgetId, data: Box<dyn RenderObject>) -> RenderId {
    let rid = RenderId(self.arena.new_node(data));
    self.render_to_widget.insert(rid, owner);
    rid
  }

  #[cfg(test)]
  pub(crate) fn render_to_widget(&self) -> &HashMap<RenderId, WidgetId> { &self.render_to_widget }
}

impl RenderId {
  /// Translates the global window coordinate pos to widget coordinates.
  pub fn map_to_global(self, pos: Point, tree: &RenderTree, layout_infos: &LayoutStore) -> Point {
    self
      .ancestors(&tree)
      .fold(pos, |pos, id| id.map_to_parent(pos, &tree, layout_infos))
  }

  /// Translates the global screen coordinate pos to widget coordinates.
  pub fn map_from_global(self, pos: Point, tree: &RenderTree, layout_infos: &LayoutStore) -> Point {
    self
      .ancestors(tree)
      .fold(pos, |pos, id| id.map_from_parent(pos, &tree, layout_infos))
  }

  /// Translates the render object coordinate pos to the coordinate system of
  /// `parent`.
  pub fn map_to_parent(
    self,
    mut pos: Point,
    tree: &RenderTree,
    layout_infos: &LayoutStore,
  ) -> Point {
    let obj = self.get(tree).expect("Access a invalid render object");

    if let Some(t) = obj.transform() {
      pos = t.transform_point(pos);
    }

    layout_infos
      .layout_box_rect(self)
      .map_or(pos, |rect| pos + rect.min().to_vector())
  }

  /// Translates the render object coordinate pos from the coordinate system of
  /// parent to this render object coordinate system.
  pub fn map_from_parent(self, pos: Point, tree: &RenderTree, layout_infos: &LayoutStore) -> Point {
    let pos = layout_infos
      .layout_box_rect(self)
      .map_or(pos, |rect| pos - rect.min().to_vector());

    let obj = self.get(tree).expect("Access a invalid render object");
    obj
      .transform()
      .and_then(|t| t.inverse())
      .map_or(pos, |t| t.transform_point(pos))
  }

  /// Translates the render object coordinate pos to the coordinate system of
  /// `ancestor`. The `ancestor` must be a ancestor of the calling render
  /// object.
  pub fn map_to(
    self,
    pos: Point,
    ancestor: RenderId,
    tree: &RenderTree,
    layout_infos: &LayoutStore,
  ) -> Point {
    self
      .ancestors(&tree)
      .take_while(|id| *id == ancestor)
      .fold(pos, |pos, id| id.map_to_parent(pos, &tree, layout_infos))
  }

  /// Translates the render object coordinate pos from the coordinate system of
  /// ancestor to this render object coordinate system. The parent must be a
  /// parent of the calling render object.
  pub fn map_from(
    self,
    pos: Point,
    ancestor: RenderId,
    tree: &RenderTree,
    layout_infos: &LayoutStore,
  ) -> Point {
    self
      .ancestors(&tree)
      .take_while(|id| *id == ancestor)
      .fold(pos, |pos, id| id.map_from_parent(pos, &tree, layout_infos))
  }

  /// Returns a reference to the node data.
  pub(crate) fn get(self, tree: &RenderTree) -> Option<&dyn RenderObject> {
    tree.arena.get(self.0).map(|node| &**node.get())
  }

  /// Returns a mutable reference to the node data.
  pub(crate) fn get_mut(self, tree: &mut RenderTree) -> &mut dyn RenderObject {
    &mut **tree
      .arena
      .get_mut(self.0)
      .expect("Access a removed render object")
      .get_mut()
  }

  /// A delegate for [NodeId::append](indextree::NodeId.preend)
  #[inline]
  pub(crate) fn append(self, new_child: RenderId, tree: &mut RenderTree) {
    self.0.append(new_child.0, &mut tree.arena);
  }

  /// A delegate for [NodeId::remove](indextree::NodeId.remove)
  #[allow(dead_code)]
  #[inline]
  pub(crate) fn remove(self, tree: &mut RenderTree) { self.0.remove(&mut tree.arena); }

  /// Returns an iterator of references to this node’s children.
  #[inline]
  pub(crate) fn children(self, tree: &RenderTree) -> impl Iterator<Item = RenderId> + '_ {
    self.0.children(&tree.arena).map(RenderId)
  }

  /// Returns an iterator of RenderId of this RenderObject’s children, in
  /// reverse order.
  pub(crate) fn reverse_children(self, tree: &RenderTree) -> impl Iterator<Item = RenderId> + '_ {
    self.0.reverse_children(&tree.arena).map(RenderId)
  }

  /// Returns an iterator of references to this node and its descendants, in
  /// tree order.
  pub(crate) fn traverse(self, tree: &RenderTree) -> impl Iterator<Item = RenderEdge> + '_ {
    self.0.traverse(&tree.arena).map(|edge| match edge {
      NodeEdge::Start(id) => RenderEdge::Start(RenderId(id)),
      NodeEdge::End(id) => RenderEdge::End(RenderId(id)),
    })
  }

  /// A delegate for [NodeId::ancestors](indextree::NodeId.ancestors)
  pub(crate) fn ancestors(self, tree: &RenderTree) -> impl Iterator<Item = RenderId> + '_ {
    self.0.ancestors(&tree.arena).map(RenderId)
  }

  /// Drop the subtree
  pub(crate) fn drop(self, tree: &mut RenderTree, layout_info: &mut layout_store::LayoutStore) {
    let RenderTree { render_to_widget, arena, .. } = tree;
    self.0.descendants(arena).for_each(|id| {
      let rid = RenderId(id);
      render_to_widget.remove(&rid);
      layout_info.remove(rid);
    });

    layout_info.remove(self);
    self.0.remove_subtree(&mut tree.arena);
    if tree.root == Some(self) {
      tree.root = None;
    }
  }

  pub(crate) fn relative_to_widget(&self, tree: &RenderTree) -> Option<WidgetId> {
    tree.render_to_widget.get(&self).copied()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::sync::{Arc, Mutex};

  #[derive(Debug, Clone)]
  struct MockRenderObj {
    records: Arc<Mutex<Vec<RenderId>>>,
  }

  impl RenderObject for MockRenderObj {
    fn perform_layout(&mut self, clamp: BoxClamp, ctx: &mut RenderCtx) -> Size {
      self.records.lock().unwrap().push(ctx.render_id());
      ctx.children().for_each(|mut child| {
        child.perform_layout(clamp);
      });
      Size::zero()
    }
    fn only_sized_by_parent(&self) -> bool { false }
    fn paint<'a>(&'a self, _: &mut PaintingCtx<'a>) {}
    fn transform(&self) -> Option<Transform> { None }
  }

  #[test]
  fn relayout_always_from_top_to_down() {
    let records = Arc::new(Mutex::new(vec![]));
    let mut tree = RenderTree::default();
    let obj = Box::new(MockRenderObj { records: records.clone() });
    let grand_parent = tree.new_node(unsafe { WidgetId::dummy() }, obj.clone());
    tree.set_root(grand_parent);

    let parent = tree.new_node(unsafe { WidgetId::dummy() }, obj.clone());
    grand_parent.append(parent, &mut tree);

    let son = tree.new_node(unsafe { WidgetId::dummy() }, obj);
    parent.append(son, &mut tree);

    parent.mark_needs_layout(&mut tree);
    grand_parent.mark_needs_layout(&mut tree);
    son.mark_needs_layout(&mut tree);
    let mut canvas = Box::pin(Canvas::new(None));
    tree.layout(Size::zero(), canvas.as_mut());

    assert_eq!(&*records.lock().unwrap(), &[grand_parent, parent, son]);
    assert!(tree.needs_layout.is_empty());
  }

  #[test]
  fn fix_ensure_relayout() {
    #[derive(Debug)]
    struct DoubleSize;

    impl CombinationWidget for DoubleSize {
      fn build(&self, _: &mut BuildCtx) -> BoxedWidget {
        let stateful = SizedBox { size: Size::new(100., 100.) }.into_stateful();
        let mut state = stateful.state_ref();
        stateful
          .on_pointer_move(move |_| {
            state.size *= 2.;
          })
          .box_it()
      }
    }

    let mut wnd = window::Window::without_render(DoubleSize.box_it(), Size::new(500., 500.));
    wnd.render_ready();

    {
      let r_tree = wnd.render_tree();
      let r_root = r_tree.root().unwrap();

      assert_eq!(
        r_root
          .layout_box_rect(unsafe { r_tree.get_unchecked_mut() })
          .unwrap()
          .size,
        Size::new(100., 100.)
      );
    }

    wnd.processes_native_event(winit::event::WindowEvent::CursorMoved {
      device_id: unsafe { winit::event::DeviceId::dummy() },
      position: (1, 1).into(),
      modifiers: ModifiersState::default(),
    });

    wnd.render_ready();

    let r_tree = wnd.render_tree();
    let r_root = r_tree.root().unwrap();
    assert_eq!(
      r_root
        .layout_box_rect(unsafe { r_tree.get_unchecked_mut() })
        .unwrap()
        .size,
      Size::new(200., 200.)
    );
  }
}
