use ribir_geom::{Point, Rect, Size};

use super::{LayoutCtx, WidgetCtxImpl};
use crate::{
  prelude::ProviderCtx,
  widget::{VisualBox, WidgetTree},
  widget_tree::WidgetId,
};
pub struct VisualCtx<'a> {
  id: WidgetId,
  tree: &'a mut WidgetTree,
  provider_ctx: &'a mut ProviderCtx,
  clip_area: Option<Rect>,
}

impl<'a> WidgetCtxImpl for VisualCtx<'a> {
  #[inline]
  fn id(&self) -> WidgetId { self.id }

  #[inline]
  fn tree(&self) -> &WidgetTree { self.tree }
}

impl<'a> VisualCtx<'a> {
  pub(crate) fn from_layout_ctx<'c: 'a, 'b: 'a>(ctx: &'c mut LayoutCtx<'b>) -> Self {
    let id = ctx.id();
    let LayoutCtx { provider_ctx, tree, .. } = ctx;
    Self { id, tree: *tree, provider_ctx, clip_area: None }
  }

  pub fn update_visual_box(&mut self) -> VisualBox {
    let id = self.id();

    let tree = unsafe { &*(self.tree as *mut WidgetTree) };
    let w = id.assert_get(tree);
    let rect = w.visual_box(self);

    let mut subtree = self.descendants_bounds();
    if let Some(clip) = &self.clip_area {
      subtree = subtree.and_then(|rect| clip.intersection(&rect));
    }
    if let Some(transform) = w.get_transform() {
      subtree = subtree.map(|rect| transform.outer_transformed_rect(&rect));
    }

    let visual_box = VisualBox { rect, subtree };

    let info = self.tree.store.layout_info_or_default(id);
    info.visual_box = visual_box;
    info.visual_box
  }

  pub fn descendants_bounds(&self) -> Option<Rect> {
    let id = self.id;
    let children = id.children(self.tree);

    let mut rect = None;
    for child in children {
      if let Some(mut child_view) = self.visual_rect(child) {
        let pos = self
          .position(child)
          .expect("child position should be set");

        child_view.origin += pos.to_vector();
        if rect.is_none() {
          rect = Some(child_view);
        } else {
          rect = rect.map(|rect| rect.union(&child_view));
        }
      }
    }
    rect
  }

  pub fn clip(&mut self, rect: Rect) {
    self.clip_area = self
      .clip_area
      .and_then(|r| {
        r.intersection(&rect)
          .or(Some(Rect::from_size(Size::zero())))
      })
      .or(Some(rect));
  }

  fn visual_rect(&self, id: WidgetId) -> Option<Rect> {
    self
      .tree
      .store
      .layout_info(id)
      .and_then(|info| info.visual_box.bounds_rect())
  }

  fn position(&self, id: WidgetId) -> Option<Point> {
    self
      .tree
      .store
      .layout_info(id)
      .map(|info| info.pos)
  }
}

impl<'w> AsRef<ProviderCtx> for VisualCtx<'w> {
  fn as_ref(&self) -> &ProviderCtx { self.provider_ctx }
}

impl<'w> AsMut<ProviderCtx> for VisualCtx<'w> {
  fn as_mut(&mut self) -> &mut ProviderCtx { self.provider_ctx }
}
