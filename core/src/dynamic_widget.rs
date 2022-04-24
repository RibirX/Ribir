use std::{any::Any, cell::RefCell, collections::HashMap, pin::Pin, rc::Rc};

use crate::prelude::{widget_tree::WidgetTree, *};
use rxrust::{
  prelude::MutRc,
  subscription::{SingleSubscription, SubscriptionGuard},
};
use smallvec::SmallVec;

/// Trait use to update dynamic widgets at real time should present
pub(crate) trait DynamicWidgetGenerator {
  fn parent(&self) -> Option<WidgetId>;
  fn update_generated_widgets(&mut self, tree: Pin<&mut WidgetTree>);
  fn info(&self) -> &GenerateInfo;
}
/// Widget which have some child widget generate by `WidgetGenerator`
pub struct WidgetWithGenerator<W> {
  widget: W,
  /// keep generator have same lifetime with `W`
  _generators_handle: SmallVec<[GeneratorHandler; 1]>,
}

/// ExprChild is a virtual child used in `widget!`, which use to generate
/// dynamic widgets and provide ability to keep them up to date in their
/// lifetime.
pub struct ExprChild<G> {
  info: GenerateInfo,
  generator: G,
}

/// The unique id of widget generator in application
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GeneratorID(usize);

pub(crate) struct GeneratorHandler {
  info: GenerateInfo,
  subscription: SubscriptionGuard<MutRc<SingleSubscription>>,
}
struct GenerateInfoInner {
  id: GeneratorID,
  parent: Option<WidgetId>,
  generated_widgets: SmallVec<[WidgetId; 1]>,
}

#[derive(Clone)]
pub(crate) struct GenerateInfo(Rc<RefCell<GenerateInfoInner>>);

pub struct DynamicWidget<W> {
  generate_by: GenerateInfo,
  widget: W,
}

impl GenerateInfo {
  pub(crate) fn new(id: GeneratorID) -> Self {
    GenerateInfo(Rc::new(RefCell::new(GenerateInfoInner {
      id,
      parent: None,
      generated_widgets: <_>::default(),
    })))
  }

  pub(crate) fn parent(&self) -> Option<WidgetId> { self.0.borrow().parent }

  pub(crate) fn generate_id(&self) -> GeneratorID { self.0.borrow().id }

  pub(crate) fn add_generated_widget_id(&self, id: WidgetId) {
    self.0.borrow_mut().generated_widgets.push(id);
  }
}

impl<W: RenderWidget> RenderWidget for WidgetWithGenerator<W> {
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    self.widget.perform_layout(clamp, ctx)
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { self.widget.only_sized_by_parent() }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) { self.widget.paint(ctx) }
}

impl<W: Compose> Compose for WidgetWithGenerator<W> {
  #[inline]
  fn compose(&self, ctx: &mut BuildCtx) -> BoxedWidget { self.widget.compose(ctx) }
}

impl<W: RenderWidget> RenderWidget for DynamicWidget<W> {
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    self.widget.perform_layout(clamp, ctx)
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { self.widget.only_sized_by_parent() }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) { self.widget.paint(ctx) }
}

impl<W: Compose> Compose for DynamicWidget<W> {
  #[inline]
  fn compose(&self, ctx: &mut BuildCtx) -> BoxedWidget { self.widget.compose(ctx) }
}

impl<W> QueryType for DynamicWidget<W>
where
  Self: Any,
{
  fn query_any(&self, type_id: std::any::TypeId) -> Option<&dyn Any> {
    self.widget.query_any(type_id)
  }

  fn query_any_mut(&mut self, type_id: std::any::TypeId) -> Option<&mut dyn Any> {
    self.widget.query_any_mut(type_id)
  }

  fn query_all_inner_any(&self, type_id: std::any::TypeId, callback: &dyn Fn(&dyn Any) -> bool) {
    let Self { generate_by, widget } = self;
    if generate_by.type_id() == type_id && callback(generate_by) {
      widget.query_all_inner_any(type_id, callback)
    }
  }

  fn query_all_inner_any_mut(
    &mut self,
    type_id: std::any::TypeId,
    callback: &mut dyn FnMut(&mut dyn Any) -> bool,
  ) {
    let Self { generate_by, widget } = self;
    if (&*generate_by).type_id() == type_id && callback(generate_by) {
      widget.query_all_inner_any_mut(type_id, callback)
    }
  }
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
  fn update_generated_widgets(&mut self, mut tree: Pin<&mut WidgetTree>) {
    let new_widgets_iter = self.generator();
    let info = self.info.0.borrow_mut();
    let parent = info.parent.unwrap();
    let mut insert_at = info.generated_widgets.first().cloned();

    let mut key_widgets = info
      .generated_widgets
      .iter()
      .filter_map(|id| {
        if let Some(key) = id.assert_get(&*tree).get_key().cloned() {
          id.detach(&mut *tree);
          Some((key.clone(), *id))
        } else {
          id.remove_subtree(&mut *tree);
          None
        }
      })
      .collect::<HashMap<_, _, ahash::RandomState>>();

    new_widgets_iter.into_iter().for_each(|c| {
      insert_at = match c.0.get_key().and_then(|k| key_widgets.remove(&*k)) {
        Some(c_id) => {
          // parent.insert_after();
          // todo: we need repair sub tree
          // tree.repair_subtree(c_id);
          Some(c_id)
        }
        None => {
          todo!(" insert new widget and inflate");
          // parent.insert_after()
          // tree.inflate_append(c, parent)
        }
      }
    });

    key_widgets
      .into_iter()
      .for_each(|(_, k)| k.remove_subtree(&mut tree));
  }

  fn parent(&self) -> Option<WidgetId> { self.info.parent() }

  fn info(&self) -> &GenerateInfo { &self.info }
}

impl GeneratorHandler {
  pub(crate) fn assign_parent(&self, parent: WidgetId) {
    assert!(self.info.parent().is_none());
    self.info.0.borrow_mut().parent = Some(parent);
  }
}

impl GeneratorID {
  #[inline]
  pub(crate) fn next_id(self) -> Self { Self(self.0 + 1) }
}
