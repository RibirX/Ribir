use crate::{prelude::*, widget::widget_tree::*};
use canvas::Canvas;
use indextree::*;
use std::{
  cmp::Reverse,
  collections::{BinaryHeap, HashMap},
  pin::Pin,
};

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

#[derive(Default)]
pub struct RenderTree {
  arena: Arena<Box<dyn RenderObjectSafety + Send + Sync>>,
  root: Option<RenderId>,
  /// A hash map to mapping a render object in render tree to its corresponds
  /// render widget in widget tree.
  render_to_widget: HashMap<RenderId, WidgetId>,
  /// Store the render object's place relative to parent coordinate and the
  /// clamp passed from parent.
  layout_info: HashMap<RenderId, BoxLayout>,
  /// root of sub tree which needed to perform layout, store as min-head by the
  /// node's depth.
  needs_layout: BinaryHeap<Reverse<(usize, RenderId)>>,
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
  pub(crate) fn new_node(
    &mut self,
    owner: WidgetId,
    data: Box<dyn RenderObjectSafety + Send + Sync>,
  ) -> RenderId {
    let rid = RenderId(self.arena.new_node(data));
    self.render_to_widget.insert(rid, owner);
    rid
  }

  /// Do the work of computing the layout for all node which need, always layout
  /// from the root to leaf. Return if any node has really computing the layout.
  pub fn layout(&mut self, win_size: Size, mut canvas: Pin<&mut Canvas>) -> bool {
    let needs_layout = self.needs_layout.clone();
    needs_layout.iter().for_each(|Reverse((_depth, rid))| {
      let clamp = rid
        .layout_clamp(self)
        .unwrap_or_else(|| BoxClamp { min: Size::zero(), max: win_size });
      rid.perform_layout(clamp, canvas.as_mut(), unsafe { Pin::new_unchecked(self) });
    });

    self.needs_layout.clear();
    !needs_layout.is_empty()
  }

  #[cfg(test)]
  pub(crate) fn render_to_widget(&self) -> &HashMap<RenderId, WidgetId> { &self.render_to_widget }

  #[cfg(test)]
  pub fn layout_info(&self) -> &HashMap<RenderId, BoxLayout> { &self.layout_info }

  fn push_relayout_sub_root(&mut self, rid: RenderId) {
    self
      .needs_layout
      .push(std::cmp::Reverse((rid.ancestors(self).count(), rid)));
  }
}

impl RenderId {
  /// Translates the global window coordinate pos to widget coordinates.
  pub fn map_to_global(self, pos: Point, tree: &RenderTree) -> Point {
    self
      .ancestors(&tree)
      .fold(pos, |pos, id| id.map_to_parent(pos, &tree))
  }

  /// Translates the global screen coordinate pos to widget coordinates.
  pub fn map_from_global(self, pos: Point, tree: &RenderTree) -> Point {
    self
      .ancestors(tree)
      .fold(pos, |pos, id| id.map_from_parent(pos, &tree))
  }

  /// Translates the render object coordinate pos to the coordinate system of
  /// `parent`.
  pub fn map_to_parent(self, mut pos: Point, tree: &RenderTree) -> Point {
    let obj = self.get(tree).expect("Access a invalid render object");

    if let Some(t) = obj.transform() {
      pos = t.transform_point(pos);
    }

    self
      .layout_box_rect(tree)
      .map_or(pos, |rect| pos + rect.min().to_vector())
  }

  /// Translates the render object coordinate pos from the coordinate system of
  /// parent to this render object coordinate system.
  pub fn map_from_parent(self, pos: Point, tree: &RenderTree) -> Point {
    let pos = self
      .layout_box_rect(tree)
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
  pub fn map_to(self, pos: Point, ancestor: RenderId, tree: &RenderTree) -> Point {
    self
      .ancestors(&tree)
      .take_while(|id| *id == ancestor)
      .fold(pos, |pos, id| id.map_to_parent(pos, &tree))
  }

  /// Translates the render object coordinate pos from the coordinate system of
  /// ancestor to this render object coordinate system. The parent must be a
  /// parent of the calling render object.
  pub fn map_from(self, pos: Point, ancestor: RenderId, tree: &RenderTree) -> Point {
    self
      .ancestors(&tree)
      .take_while(|id| *id == ancestor)
      .fold(pos, |pos, id| id.map_from_parent(pos, &tree))
  }

  /// Returns a reference to the node data.
  pub(crate) fn get(self, tree: &RenderTree) -> Option<&(dyn RenderObjectSafety + Send + Sync)> {
    tree.arena.get(self.0).map(|node| &**node.get())
  }

  /// Returns a mutable reference to the node data.
  pub(crate) fn get_mut(
    self,
    tree: &mut RenderTree,
  ) -> &mut (dyn RenderObjectSafety + Send + Sync) {
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
  pub(crate) fn drop(self, tree: &mut RenderTree) {
    let RenderTree {
      render_to_widget, arena, layout_info, ..
    } = tree;
    self.0.descendants(arena).for_each(|id| {
      let rid = RenderId(id);
      render_to_widget.remove(&rid);
      layout_info.remove(&rid);
    });

    tree.layout_info.remove(&self);
    self.0.remove_subtree(&mut tree.arena);
    if tree.root == Some(self) {
      tree.root = None;
    }
  }

  pub(crate) fn relative_to_widget(&self, tree: &RenderTree) -> Option<WidgetId> {
    tree.render_to_widget.get(&self).copied()
  }

  pub(crate) fn layout_clamp(self, tree: &RenderTree) -> Option<BoxClamp> {
    tree.layout_info.get(&self).map(|info| info.clamp)
  }

  pub(crate) fn layout_box_rect(self, tree: &RenderTree) -> Option<Rect> {
    tree.layout_info.get(&self).and_then(|info| info.rect)
  }

  pub(crate) fn layout_clamp_mut(self, tree: &mut RenderTree) -> &mut BoxClamp {
    &mut self.layout_info_mut(&mut tree.layout_info).clamp
  }

  pub(crate) fn layout_box_rect_mut(self, tree: &mut RenderTree) -> &mut Rect {
    self
      .layout_info_mut(&mut tree.layout_info)
      .rect
      .get_or_insert_with(Rect::zero)
  }

  pub(crate) fn mark_needs_layout(self, tree: &mut RenderTree) {
    if self.layout_box_rect(tree).is_some() {
      let mut relayout_root = self;
      let RenderTree { arena, layout_info, .. } = tree;
      // All ancestors of this render object should relayout until the one which only
      // sized by parent.
      self.0.ancestors(arena).all(|id| {
        let rid = RenderId(id);
        layout_info.remove(&rid);
        relayout_root = rid;

        let sized_by_parent = arena
          .get(id)
          .map_or(false, |node| node.get().only_sized_by_parent());

        !sized_by_parent
      });
      tree.push_relayout_sub_root(relayout_root);
    } else {
      tree.push_relayout_sub_root(self);
    }
  }

  pub(crate) fn perform_layout(
    self,
    clamp: BoxClamp,
    canvas: Pin<&mut Canvas>,
    mut tree: Pin<&mut RenderTree>,
  ) -> Size {
    let lay_outed = self.layout_box_rect(&*tree);
    match lay_outed {
      Some(rect) if self.layout_clamp(&*tree) == Some(clamp) => rect.size,
      _ => {
        // Safety: only split tree from ctx to access the render object instance.
        let tree_mut = unsafe {
          let ptr = tree.as_mut().get_unchecked_mut() as *mut RenderTree;
          &mut *ptr
        };
        let size = self
          .get_mut(tree_mut)
          .perform_layout(clamp, &mut RenderCtx::new(tree, canvas, self));
        *self.layout_clamp_mut(tree_mut) = clamp;
        self.layout_box_rect_mut(tree_mut).size = size;
        size
      }
    }
  }

  fn layout_info_mut(self, layout_info: &mut HashMap<RenderId, BoxLayout>) -> &mut BoxLayout {
    layout_info.entry(self).or_insert_with(BoxLayout::default)
  }
}

impl !Unpin for RenderTree {}

#[cfg(test)]
mod tests {
  use super::*;
  use std::sync::{Arc, Mutex};

  #[derive(Debug, Clone)]
  struct MockRenderObj {
    records: Arc<Mutex<Vec<RenderId>>>,
  }

  impl RenderObjectSafety for MockRenderObj {
    fn update(&mut self, _: Box<dyn Any>, _: &mut UpdateCtx) {}
    fn perform_layout(&mut self, clamp: BoxClamp, ctx: &mut RenderCtx) -> Size {
      self.records.lock().unwrap().push(ctx.render_id());
      ctx.children().for_each(|mut child| {
        child.perform_layout(clamp);
      });
      Size::zero()
    }
    fn only_sized_by_parent(&self) -> bool { false }
    fn paint<'a>(&'a self, _: &mut PaintingContext<'a>) {}
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
    #[derive(Debug, AttachAttr)]
    struct DoubleSize;

    impl CombinationWidget for DoubleSize {
      fn build(&self, _: &mut BuildCtx) -> BoxedWidget {
        let stateful = SizedBox::from_size(Size::new(100., 100.)).into_stateful();
        let mut state = stateful.ref_cell();
        stateful
          .on_pointer_move(move |_| {
            let mut sized_box = state.borrow_mut();
            sized_box.size *= 2.;
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
