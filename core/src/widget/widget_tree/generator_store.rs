use rxrust::{
  observable::SubscribeNext,
  prelude::Observable,
  subscription::{SubscriptionGuard, SubscriptionLike},
};
use smallvec::SmallVec;

use crate::{
  dynamic_widget::{ExprWidget, Generator, GeneratorID, GeneratorInfo},
  prelude::{ChangeScope, WidgetId},
};
use std::{
  cell::RefCell,
  collections::{HashMap, HashSet},
  rc::Rc,
};

#[derive(Default)]
pub(crate) struct GeneratorStore {
  next_generator_id: GeneratorID,
  // todo: use id_map
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
    ExprWidget { expr, upstream }: ExprWidget<()>,
    parent: Option<WidgetId>,
    generated_widgets: SmallVec<[WidgetId; 1]>,
  ) -> GeneratorID {
    let id = self.next_generator_id;
    self.next_generator_id = id.next_id();
    let info = GeneratorInfo::new(id, parent, generated_widgets);
    let needs_regen = self.needs_regen.clone();
    needs_regen.borrow_mut().insert(id);
    let _subscription = upstream
      .filter(|scope| scope.contains(ChangeScope::FRAMEWORK))
      .subscribe(move |_| {
        needs_regen.borrow_mut().insert(id);
      })
      .unsubscribe_when_dropped();
    self.add_generator(Generator { info, expr });

    if let Some(p) = parent {
      self
        .lifetime
        .entry(p)
        .or_default()
        .push(GeneratorHandle { id, _subscription });
    }
    id
  }

  pub(crate) fn add_generator(&mut self, g: Generator) {
    self.generators.insert(g.info().generate_id(), g);
  }

  pub(crate) fn is_dirty(&self) -> bool { !self.needs_regen.borrow().is_empty() }

  pub(crate) fn take_needs_regen_generator(&mut self) -> Option<Vec<Generator>> {
    (self.is_dirty()).then(|| {
      self
        .needs_regen
        .borrow_mut()
        .drain()
        .filter_map(|id| self.generators.remove(&id))
        .collect::<Vec<_>>()
    })
  }

  pub(crate) fn on_widget_drop(&mut self, widget: WidgetId) {
    if let Some(ids) = self.lifetime.remove(&widget) {
      ids.iter().for_each(|h| {
        self.generators.remove(&h.id);
        self.needs_regen.borrow_mut().remove(&h.id);
      });
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::prelude::{widget_tree::WidgetTree, *};

  #[test]
  fn perf_silent_ref_should_not_dirty_expr_widget() {
    let trigger = Stateful::new(1);
    let widget = widget! {
      track { trigger: trigger.clone() }
      Row {
        ExprWidget {
          expr: (0..3).map(|_| if *trigger > 0 {
            SizedBox { size: Size::new(1., 1.)}
          } else {
            SizedBox { size: Size::zero()}
          })
        }
      }
    };

    let mut tree = WidgetTree::new(widget, <_>::default());
    tree.tree_repair();
    tree.layout(Size::new(100., 100.));
    {
      *trigger.silent_ref() = 2;
    }
    assert!(!tree.generator_store.is_dirty())
  }
}
