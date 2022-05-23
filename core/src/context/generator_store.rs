use rxrust::{
  observable::{Observable, SubscribeNext},
  subscription::{SubscriptionGuard, SubscriptionLike},
};
use smallvec::SmallVec;

use crate::{
  dynamic_widget::{DynamicWidgetGenerator, ExprWidget, Generator, GeneratorID, GeneratorInfo},
  prelude::{widget_tree::WidgetTree, WidgetId},
};
use std::{
  cell::RefCell,
  collections::{HashMap, HashSet},
  rc::Rc,
};

#[derive(Default)]
pub(crate) struct GeneratorStore {
  next_generator_id: GeneratorID,
  generators: HashMap<GeneratorID, Generator, ahash::RandomState>,
  needs_regen: Rc<RefCell<HashSet<GeneratorID, ahash::RandomState>>>,
  lifetime: HashMap<WidgetId, SmallVec<[GeneratorHandle; 1]>>,
}

struct GeneratorHandle {
  id: GeneratorID,
  _subscription: SubscriptionGuard<Box<dyn SubscriptionLike>>,
}

impl GeneratorStore {
  pub(crate) fn new_generator(
    &mut self,
    ExprWidget { expr, upstream }: ExprWidget<Box<dyn DynamicWidgetGenerator>>,
    parent: WidgetId,
    generated_widgets: SmallVec<[WidgetId; 1]>,
  ) -> Option<GeneratorID> {
    upstream.map(|upstream| {
      let id = self.next_generator_id;
      self.next_generator_id = id.next_id();
      let info = GeneratorInfo::new(id, parent, generated_widgets);
      let needs_regen = self.needs_regen.clone();
      let _subscription = upstream
        .filter(|b| !b)
        .subscribe(move |_| {
          needs_regen.borrow_mut().insert(id);
        })
        .unsubscribe_when_dropped();
      self.add_generator(Generator { info: info.clone(), expr });
      self
        .lifetime
        .entry(parent)
        .or_default()
        .push(GeneratorHandle { id, _subscription });
      info.generate_id()
    })
  }

  pub(crate) fn add_generator(&mut self, g: Generator) {
    self.generators.insert(g.info().generate_id(), g);
  }

  pub(crate) fn is_dirty(&self) -> bool { !self.generators.is_empty() }

  pub(crate) fn take_needs_regen(&mut self, tree: &WidgetTree) -> Vec<Generator> {
    let mut generators = self
      .needs_regen
      .borrow_mut()
      .drain()
      .filter_map(|id| self.generators.remove(&id))
      .filter(|g| g.info().parent().is_dropped(tree))
      .collect::<Vec<_>>();

    generators.sort_by_cached_key(|g| g.info.parent().ancestors(tree).count());
    generators
  }

  pub(crate) fn on_widget_drop(&mut self, widget: WidgetId) {
    if let Some(ids) = self.lifetime.remove(&widget) {
      ids.iter().for_each(|h| {
        self.generators.remove(&h.id);
      });
    }
  }
}
