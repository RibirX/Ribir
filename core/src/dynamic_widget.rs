use std::{any::Any, cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
  impl_query_type,
  prelude::{key::Key, widget_tree::WidgetTree, *},
};
use rxrust::{
  prelude::MutRc,
  subscription::{SingleSubscription, SubscriptionGuard},
};
use smallvec::SmallVec;

/// Trait use to update dynamic widgets at real time should present
pub(crate) trait DynamicWidgetGenerator {
  fn parent(&self) -> Option<WidgetId>;
  fn update_generated_widgets(&mut self, ctx: &mut Context);
  fn info(&self) -> &GenerateInfo;
}

/// ExprChild is a virtual child used in `widget!`, which use to generate
/// dynamic widgets and provide ability to keep them up to date in their
/// lifetime.
pub struct ExprChild<G> {
  info: GenerateInfo,
  generator: G,
}

/// A Widget which associated to widget generator and was wrapped information to
/// help detect where to place the widgets generated by generator.
pub struct AssociatedGenerator<W, Info> {
  widget: W,
  info: Info,
}

/// Widget which generated by widget generator.
pub type DynamicWidget<W> = AssociatedGenerator<W, DynamicWidgetInfo>;
/// The widget who is the parent of the widget generator.
pub type GeneratorParent<W> = AssociatedGenerator<W, GeneratorParentInfo>;
/// Widget next to the the last widget generated by generator.
pub type GeneratorNextSibling<W> = AssociatedGenerator<W, PrevSiblingInfo>;
/// Static widget next to the the last widget generated by generator.
pub type GeneratorStaticNextSibling<W> = AssociatedGenerator<W, StaticPrevSibling>;

/// The unique id of widget generator in application
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GeneratorID(usize);

pub struct DynamicWidgetInfo(GenerateInfo);
pub struct PrevSiblingInfo(GenerateInfo);

pub struct StaticPrevSibling(GenerateInfo);
pub struct GeneratorParentInfo(SmallVec<[GeneratorHandler; 1]>);
pub(crate) struct GeneratorHandler {
  info: GenerateInfo,
  subscription: SubscriptionGuard<MutRc<SingleSubscription>>,
}

struct GenerateInfoInner {
  id: GeneratorID,
  /// The parent of the generator.
  parent: Option<WidgetId>,
  /// the id of widget before the first widget generated by generator.
  prev_sibling: Option<WidgetId>,
  /// the id of static widget before the the last widget generated by
  /// generator.
  static_prev_sibling: Option<WidgetId>,
  /// widget generated by the generator.
  generated_widgets: SmallVec<[WidgetId; 1]>,
}

#[derive(Clone)]
pub(crate) struct GenerateInfo(Rc<RefCell<GenerateInfoInner>>);

impl GenerateInfo {
  pub(crate) fn new(id: GeneratorID) -> Self {
    GenerateInfo(Rc::new(RefCell::new(GenerateInfoInner {
      id,
      parent: None,
      static_prev_sibling: None,
      prev_sibling: None,
      generated_widgets: <_>::default(),
    })))
  }

  pub(crate) fn parent(&self) -> Option<WidgetId> { self.0.borrow().parent }

  pub(crate) fn generate_id(&self) -> GeneratorID { self.0.borrow().id }

  fn add_dynamic_widget_tmp_anchor(&self, tree: &mut WidgetTree) -> WidgetId {
    let inner = self.0.borrow_mut();
    let prev_sibling = inner
      .generated_widgets
      .first()
      .cloned()
      .and_then(|id| id.prev_sibling(tree));

    let parent = inner
      .parent
      .expect("parent of expr child should always exist.");
    let prev_sibling = prev_sibling
      .or(inner.prev_sibling)
      .or(inner.static_prev_sibling)
      .or_else(|| parent.first_child(tree));

    let holder = tree.place_holder();
    if let Some(prev_sibling) = prev_sibling {
      prev_sibling.insert_next(holder, tree)
    } else {
      parent.append(holder, tree)
    }
    holder
  }
}

impl<W: Render, Info> Render for AssociatedGenerator<W, Info> {
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    self.widget.perform_layout(clamp, ctx)
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { self.widget.only_sized_by_parent() }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) { self.widget.paint(ctx) }
}

impl<C: Compose, Info> Compose for AssociatedGenerator<C, Info> {
  #[inline]
  fn compose(self, ctx: &mut BuildCtx) -> BoxedWidget { self.widget.compose(ctx) }
}

impl<W, Info> QueryType for AssociatedGenerator<W, Info>
where
  Self: Any,
{
  impl_query_type!(info, widget);
}

impl<G: FnMut() -> W, W> ExprChild<G> {
  pub fn new(ctx: &mut BuildCtx, generator: G) -> Self {
    Self {
      info: ctx.new_generator_info(),
      generator,
    }
  }

  #[inline]
  fn generator(&mut self) -> W
  where
    W: IntoIterator<Item = BoxedWidget>,
  {
    (self.generator)()
  }
}

impl<G, W> DynamicWidgetGenerator for ExprChild<G>
where
  G: FnMut() -> W,
  W: IntoIterator<Item = BoxedWidget>,
{
  #[inline]
  fn update_generated_widgets(&mut self, ctx: &mut Context) {
    let new_widgets_iter = self.generator();

    let tmp_anchor = self.info.add_dynamic_widget_tmp_anchor(ctx.tree_mut());
    let info = self.info.0.borrow_mut();
    let mut key_widgets = info
      .generated_widgets
      .iter()
      .filter_map(|id| {
        let tree = ctx.tree_mut();
        if let Some(key) = id
          .assert_get(tree)
          .query_first_type::<Key>(QueryOrder::OutsideFirst)
          .cloned()
        {
          id.detach(tree);
          Some((key.clone(), *id))
        } else {
          id.remove_subtree(tree);
          None
        }
      })
      .collect::<HashMap<_, _, ahash::RandomState>>();

    let parent = info.parent.unwrap();
    let mut insert_at = tmp_anchor;
    new_widgets_iter.into_iter().for_each(|c| {
      insert_at = parent.insert_child(
        c,
        |node, tree| {
          let old = node
            .query_first_type::<Key>(QueryOrder::OutsideFirst)
            .and_then(|k| key_widgets.remove(k));
          let id = match old {
            Some(c_id) => {
              *c_id.assert_get_mut(tree) = node;
              c_id
            }
            None => tree.new_node(node),
          };
          insert_at.insert_next(id, tree);
          id
        },
        |wid, child, ctx| {
          wid.append_widget(child, ctx);
        },
        ctx,
      );
    });

    let tree = ctx.tree_mut();
    key_widgets
      .into_iter()
      .for_each(|(_, k)| k.remove_subtree(tree));
    tmp_anchor.remove_subtree(tree);
  }

  fn parent(&self) -> Option<WidgetId> { self.info.parent() }

  fn info(&self) -> &GenerateInfo { &self.info }
}

impl GeneratorParentInfo {
  pub(crate) fn assign_parent(&self, parent: WidgetId) {
    debug_assert!(self.0.iter().all(|handler| handler.info.parent().is_none()));
    self
      .0
      .iter()
      .for_each(|handler| handler.info.0.borrow_mut().parent = Some(parent))
  }
}

impl DynamicWidgetInfo {
  pub(crate) fn assign_dynamic_widget_id(&self, id: WidgetId) {
    self.0.0.borrow_mut().generated_widgets.push(id);
  }
}

impl StaticPrevSibling {
  pub(crate) fn assign_static_prev_sibling(&self, id: WidgetId) {
    debug_assert!(self.0.0.borrow().static_prev_sibling.is_none());
    self.0.0.borrow_mut().static_prev_sibling = Some(id);
  }
}

impl PrevSiblingInfo {
  pub(crate) fn assign_next_sibling(&self, id: WidgetId) {
    debug_assert!(self.0.0.borrow().prev_sibling.is_none());
    self.0.0.borrow_mut().prev_sibling = Some(id);
  }
}

impl GeneratorID {
  #[inline]
  pub(crate) fn next_id(self) -> Self { Self(self.0 + 1) }
}
