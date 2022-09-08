use rxrust::{observable::SubscribeNext, prelude::Observable};

use crate::{
  dynamic_widget::{ExprWidget, Generator, GeneratorID, GeneratorInfo},
  prelude::{ChangeScope, WidgetId},
  widget::{BuildCtx, ExprResult},
};
use std::{
  cell::RefCell,
  collections::{HashMap, HashSet},
  rc::Rc,
};

use super::WidgetTree;

#[derive(Default)]
pub(crate) struct GeneratorStore {
  next_generator_id: GeneratorID,
  // todo: use id_map
  generators: HashMap<GeneratorID, Generator, ahash::RandomState>,
  needs_regen: Rc<RefCell<HashSet<GeneratorID, ahash::RandomState>>>,
}

impl GeneratorStore {
  pub(crate) fn new_generator(
    &mut self,
    ExprWidget { expr, upstream }: ExprWidget<Box<dyn FnMut(&mut BuildCtx) -> ExprResult>>,
    parent: Option<WidgetId>,
    road_sign: WidgetId,
    has_child: bool,
  ) -> GeneratorID {
    let id = self.next_generator_id;
    self.next_generator_id = id.next_id();
    let info = GeneratorInfo::new(id, parent, road_sign, has_child);
    let needs_regen = self.needs_regen.clone();
    needs_regen.borrow_mut().insert(id);
    let _upstream_handle = upstream
      .filter(|scope| scope.contains(ChangeScope::FRAMEWORK))
      .subscribe(move |_| {
        needs_regen.borrow_mut().insert(id);
      })
      .unsubscribe_when_dropped();
    self.add_generator(Generator { info, expr, _upstream_handle });

    id
  }

  pub(crate) fn add_generator(&mut self, g: Generator) {
    self.generators.insert(g.info().generate_id(), g);
  }

  pub(crate) fn is_dirty(&self) -> bool { !self.needs_regen.borrow().is_empty() }
}

impl WidgetTree {
  pub(crate) fn take_needs_regen_generator(&mut self) -> Option<Vec<Generator>> {
    let store = &mut self.generator_store;
    if !store.is_dirty() {
      return None;
    }
    let g = store
      .needs_regen
      .borrow_mut()
      .drain()
      .filter_map(|id| store.generators.remove(&id))
      .filter(|g| {
        g.info
          .parent()
          .map_or(true, |p| !p.0.is_removed(&mut self.arena))
      })
      .collect::<Vec<_>>();

    (!g.is_empty()).then(|| g)
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
