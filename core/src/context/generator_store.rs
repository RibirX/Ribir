use crate::{
  dynamic_widget::{DynamicWidgetGenerator, GenerateInfo, GeneratorID},
  prelude::widget_tree::WidgetTree,
};
use std::{
  cell::Cell,
  cmp::Reverse,
  collections::{BinaryHeap, HashMap},
  pin::Pin,
};

#[derive(Default)]
pub(crate) struct GeneratorStore {
  next_generator_id: Cell<GeneratorID>,
  generators: HashMap<GeneratorID, Box<dyn DynamicWidgetGenerator>, ahash::RandomState>,
  need_update_generators: BinaryHeap<Reverse<(usize, GeneratorID)>>,
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

  pub(crate) fn is_dirty(&self) -> bool { !self.generators.is_empty() }

  pub(crate) fn need_regenerate(&mut self, id: GeneratorID, tree: &WidgetTree) {
    let depth = self
      .generators
      .get(&id)
      .and_then(|g| g.parent())
      .map(|wid| wid.ancestors(tree).count());
    if let Some(depth) = depth {
      self.need_update_generators.push(Reverse((depth, id)));
    }
  }

  pub(crate) fn update_dynamic_widgets(&mut self, mut tree: Pin<&mut WidgetTree>) {
    let updating_generators = self.need_update_generators.clone();
    self.need_update_generators.clear();

    for Reverse((_, gid)) in updating_generators {
      let generator = self.generators.get_mut(&gid);
      let is_dropped = generator
        .as_ref()
        .and_then(|g| g.parent())
        .map_or(true, |p| p.is_dropped(&*tree.as_ref()));
      if is_dropped {
        self.generators.remove(&gid);
        continue;
      } else {
        generator.unwrap().update_generated_widgets(tree.as_mut());
      }
    }
  }
}
