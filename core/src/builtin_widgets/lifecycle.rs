mod mounted;
pub use mounted::*;
mod performed_layout;
pub use performed_layout::*;
mod disposed;
pub use disposed::*;

#[macro_export]
macro_rules! impl_lifecycle {
  ($name: ident, $field: ident) => {
    impl ComposeChild for $name {
      type Child = Widget;
      fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget {
        compose_child_as_data_widget(child, this)
      }
    }

    impl Query for $name {
      impl_query_self_only!();
    }
  };
}

#[cfg(test)]
mod tests {
  use crate::{prelude::*, test::MockBox, widget_tree::WidgetTree};

  #[test]
  fn full_lifecycle() {
    let trigger = Stateful::new(true);
    let lifecycle = Stateful::new(vec![]);

    let w = widget! {
      track {
        trigger: trigger.clone(),
        lifecycle: lifecycle.clone()
      }
      MockBox {
        size: Size::zero(),
        mounted: move |_| lifecycle.silent().push("static mounted"),
        performed_layout: move |_| lifecycle.silent().push("static performed layout"),
        disposed: move |_| lifecycle.silent().push("static disposed"),
        DynWidget {
          dyns: trigger.then(|| widget! {
            MockBox {
              size: Size::zero(),
              mounted: move |_| lifecycle.silent().push("dyn mounted"),
              performed_layout: move |_| lifecycle.silent().push("dyn performed layout"),
              disposed: move |_| lifecycle.silent().push("dyn disposed")
            }
          })
        }
      }
    };

    let mut tree = WidgetTree::new(w, <_>::default());
    assert_eq!(&**lifecycle.raw_ref(), ["static mounted"]);
    tree.layout(Size::new(100., 100.));
    assert_eq!(
      &**lifecycle.raw_ref(),
      [
        "static mounted",
        "dyn mounted",
        "dyn performed layout",
        "static performed layout",
      ]
    );
    {
      *trigger.state_ref() = false;
    }
    tree.tree_ready(Size::zero());
    assert_eq!(
      &**lifecycle.raw_ref(),
      [
        "static mounted",
        "dyn mounted",
        "dyn performed layout",
        "static performed layout",
        "dyn disposed",
      ]
    );
  }
}
