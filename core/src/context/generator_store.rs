use crate::{
  dynamic_widget::{DynamicWidgetGenerator, GenerateInfo, GeneratorID},
  prelude::widget_tree::WidgetTree,
};
use std::{
  cell::Cell,
  cmp::Reverse,
  collections::{BinaryHeap, HashMap},
};

#[derive(Default)]
pub(crate) struct GeneratorStore {
  next_generator_id: Cell<GeneratorID>,
  generators: HashMap<GeneratorID, Box<dyn DynamicWidgetGenerator>, ahash::RandomState>,
  needs_regen: BinaryHeap<Reverse<(usize, GeneratorID)>>,
}

impl GeneratorStore {
  pub(crate) fn new_generator_info(&self) -> GenerateInfo {
    let id = self.next_generator_id.get();
    let next = id.next_id();
    self.next_generator_id.set(next);
    GenerateInfo::new(id)
  }

  pub(crate) fn add_widget_generator(&mut self, g: Box<dyn DynamicWidgetGenerator>) {
    self.generators.insert(g.info().generate_id(), g);
  }

  pub(crate) fn remove_generator(
    &mut self,
    id: GeneratorID,
  ) -> Option<Box<dyn DynamicWidgetGenerator>> {
    self.generators.remove(&id)
  }

  pub(crate) fn is_dirty(&self) -> bool { !self.generators.is_empty() }

  pub(crate) fn need_regen(&mut self, id: GeneratorID, tree: &WidgetTree) {
    let depth = self
      .generators
      .get(&id)
      .and_then(|g| g.parent())
      .map(|wid| wid.ancestors(tree).count());
    if let Some(depth) = depth {
      self.needs_regen.push(Reverse((depth, id)));
    }
  }

  pub(crate) fn take_needs_regen(&mut self) -> BinaryHeap<Reverse<(usize, GeneratorID)>> {
    let ret = self.needs_regen.clone();
    self.needs_regen.clear();
    ret
  }
}
