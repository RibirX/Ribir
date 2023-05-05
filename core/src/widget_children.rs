//! # How parent compost dynamic child (Stateful<Dynamic<_>>).
//!
//! - for `SingleChild` or `MultiChild`, they're not care about if its child is
//!   a dynamic widget. Because the child not effect the result of compose. They
//!   always accept `Widget` and not care about the information. So if the
//!   dynamic return `Widget`, it can be as the child of them.
//!
//! - for `ComposeChild`, it has custom logic to compose child:
//!   - a. if its child accept `Widget`, `Option<Widget>` or `Vec<Widget>`, that
//!     means it not care about the information of its child, so its compose
//!     child logic will not depends on its child information. if the dynamic
//!     child only generate at most one widget, it can be treat as normal child,
//!     because the compose logic work on dynamic child or the return of dynamic
//!     child have not different, because the dynamic child and itself first
//!     generate widget is same object in widget tree.
//!   - b. if it meet a dynamic child generate more than one widget (iterator),
//!     its compose logic need work on the dynamic child generate result.
//!   - c. if its child is accept a specific type and meet a dynamic child which
//!     generate that, means the compose logic maybe depends on the type
//!     information.
//!   - d. Both `b` and `c` need to expand the dynamic scope. The compose logic
//!     should work in dynamic widget.
//!
//! In implementation, I finally decide to remove the partial dynamic
//! child support, partial dynamic child means, partial of element array or
//! partial of `Template` fields is dynamic, for example, if a `ComposeChild`
//! widget accept `Vec<A>` child, it not allow accept a children list like `A,
//! Stateful<DynWidget<W>>, A`. If we allow accept that list, require A support
//! clone, this seems too strict and if `A` is not support clone, the compile
//! error is too complex to diagnostic.

use crate::prelude::*;
mod compose_child_impl;
mod multi_child_impl;
mod single_child_impl;
pub use compose_child_impl::*;
pub use multi_child_impl::*;
pub use single_child_impl::*;
pub mod child_convert;
pub use child_convert::{IntoChild, IntoEnumVariable};
/// Trait to tell Ribir a widget can have one child.
pub trait SingleChild {}

/// Trait to tell Ribir a widget can have multi child.
pub trait MultiChild {}

/// Trait mark widget can have one child and also have compose logic for widget
/// and its child.
pub trait ComposeChild: Sized {
  type Child;
  type Target;
  fn compose_child(this: State<Self>, child: Self::Child) -> Self::Target;
}

/// A alias of `WidgetPair<W, Widget>`, means `Widget` is the child of the
/// generic type.
pub type WidgetOf<W> = WidgetPair<W, Widget>;

#[cfg(test)]
mod tests {
  use std::{cell::RefCell, rc::Rc};

  use super::*;
  use crate::test::*;

  #[test]
  fn compose_template_child() {
    #[derive(Declare)]
    struct Page;
    #[derive(Declare, SingleChild)]
    struct Header;
    #[derive(Declare, SingleChild)]
    struct Content;
    #[derive(Declare, SingleChild)]
    struct Footer;

    #[derive(Template)]
    struct PageTml {
      _header: WidgetOf<Header>,
      _content: WidgetOf<Content>,
      _footer: WidgetOf<Footer>,
    }

    impl ComposeChild for Page {
      type Child = PageTml;
      type Target = Widget;
      fn compose_child(_: State<Self>, _: Self::Child) -> Widget {
        unreachable!("Only for syntax support check");
      }
    }

    widget! {
      Page {
        Header { Void {} }
        Content { Void {} }
        Footer { Void {} }
      }
    };
  }

  #[test]
  fn compose_option_child() {
    #[derive(Declare)]
    struct Parent;
    #[derive(Declare, SingleChild)]
    struct Child;

    impl ComposeChild for Parent {
      type Child = Option<WidgetPair<Child, Widget>>;
      type Target = Widget;
      fn compose_child(_: State<Self>, _: Self::Child) -> Widget {
        unreachable!("Only for syntax support check");
      }
    }

    widget! {
      Parent {
        Child { Void {} }
      }
    };
  }

  #[test]
  fn compose_option_dyn_parent() {
    widget! {
      DynWidget {
        dyns: widget::then(true, || MockBox { size: Size::zero() }),
        Void {}
      }
    };
  }

  #[test]
  fn tuple_as_vec() {
    #[derive(Declare)]
    struct A;
    #[derive(Declare)]
    struct B;

    impl ComposeChild for A {
      type Child = Vec<B>;
      type Target = Widget;
      fn compose_child(_: State<Self>, _: Self::Child) -> Widget {
        unreachable!("Only for syntax support check");
      }
    }
    widget! {
      A {
        B {}
        B {}
      }
    };
  }

  #[test]
  fn expr_with_child() {
    let size = Stateful::new(Size::zero());
    // with single child
    let _e = widget! {
      states { size: size.clone() }
      DynWidget {
        dyns: if size.area() > 0. {
           MockBox { size: *size }
        } else {
          MockBox { size: Size::new(1., 1.) }
        },
        MockBox { size: *size }
      }
    };
    // with multi child
    let _e = widget! {
      DynWidget {
        dyns: MockMulti {},
        MockBox { size: Size::zero() }
        MockBox { size: Size::zero() }
        MockBox { size: Size::zero() }
      }
    };

    // option with single child
    let _e = widget! {
      states { size: size.clone() }
      DynWidget {
        dyns: widget::then(size.area() > 0., || MockBox { size: Size::zero() }),
        MockBox { size: Size::zero() }
      }
    };

    // option with `Widget`
    let _e = widget! {
      states { size: size }
      DynWidget {
        dyns: widget::then(size.area() > 0., || MockBox { size: Size::zero() }),
        DynWidget { dyns: Void.into_widget() }
      }
    };
  }

  #[test]
  fn compose_const_dyn_option_widget() {
    let _ = widget! {
      MockBox {
        size: ZERO_SIZE,
        widget::then(true, || MockBox { size: Size::zero() })
      }
    };
  }

  #[test]
  fn pair_to_pair() {
    #[derive(Declare)]
    struct P;

    impl ComposeChild for P {
      type Child = WidgetOf<MockBox>;
      type Target = Widget;
      fn compose_child(_: State<Self>, _: Self::Child) -> Widget { unreachable!() }
    }

    let _ = widget! {
      P { MockBox { Void {} } }
    };
  }

  #[test]
  fn fix_multi_fill_for_pair() {
    struct X;
    impl ComposeChild for X {
      type Child = WidgetOf<MockBox>;
      type Target = Widget;
      fn compose_child(_: State<Self>, _: Self::Child) -> Widget { Void.into_widget() }
    }

    let child = MockBox { size: ZERO_SIZE }.with_child(Void.into_widget());
    X.with_child(child);
  }

  #[test]
  fn dyns_compose_child() {
    #[derive(Declare)]
    struct X;

    impl ComposeChild for X {
      type Child = MockBox;
      type Target = Widget;
      fn compose_child(_: State<Self>, child: Self::Child) -> Widget { child.into_widget() }
    }

    let dyns = Stateful::new(DynWidget { dyns: Some(X) });
    let size = Size::new(100., 200.);

    let w = ComposeChild::compose_child(State::<X>::from(dyns), MockBox { size });
    expect_layout_result(
      w,
      None,
      &[LayoutTestItem {
        path: &[0],
        expect: ExpectRect::from_size(size),
      }],
    );
  }

  #[test]
  fn compose_dyns_child() {
    #[derive(Declare)]
    struct X;

    impl ComposeChild for X {
      type Child = State<MockBox>;
      type Target = Widget;
      fn compose_child(_: State<Self>, child: Self::Child) -> Widget { child.into_widget() }
    }

    let trigger = Stateful::new(true);
    let size = Size::new(100., 200.);
    let w = widget! {
      states { trigger: trigger }
      X {
        DynWidget {
          dyns: if *trigger {
            MockBox { size }
          } else {
            MockBox { size: ZERO_SIZE }
          }
        }
      }
    };
    expect_layout_result(
      w,
      None,
      &[LayoutTestItem {
        path: &[0],
        expect: ExpectRect::from_size(size),
      }],
    );
  }

  #[test]
  fn fix_option_template() {
    struct Field(String);

    #[derive(Template, Default)]
    pub struct ConfigTml {
      _field: Option<Field>,
    }
    #[derive(Declare)]
    struct Host {}

    const EXPECT_SIZE: Size = Size::new(100., 200.);
    impl ComposeChild for Host {
      type Child = Option<ConfigTml>;
      type Target = Widget;
      fn compose_child(_: State<Self>, _: Self::Child) -> Widget {
        widget! { MockBox { size: EXPECT_SIZE } }.into_widget()
      }
    }

    expect_layout_result(
      widget! {
        Host { Field("test".into()) }
      }
      .into_widget(),
      None,
      &[LayoutTestItem {
        path: &[0],
        expect: ExpectRect::from_size(EXPECT_SIZE),
      }],
    );
  }

  #[test]
  fn compose_dyn_multi_child() {
    struct A;

    impl ComposeChild for A {
      type Child = Vec<Widget>;
      type Target = Widget;
      fn compose_child(_: State<Self>, child: Self::Child) -> Widget {
        MockMulti.with_child(child).into_widget()
      }
    }

    let child = DynWidget { dyns: Some([Void]) };
    let child = Stateful::new(child);
    let cnt = Rc::new(RefCell::new(0));
    let c_cnt = cnt.clone();
    child
      .modifies()
      .subscribe(move |_| *c_cnt.borrow_mut() += 1);

    let _ = Window::default_mock(A.with_child(child).into_widget(), None);
    assert_eq!(*cnt.borrow(), 0);
  }

  #[test]
  fn compose_multi_with_stateful_option() {
    struct M;
    impl ComposeChild for M {
      type Child = Vec<Widget>;
      type Target = Widget;
      fn compose_child(_: State<Self>, _: Self::Child) -> Widget { Void.into_widget() }
    }

    let c = Stateful::new(DynWidget { dyns: Some(Some(Void)) });
    let _ = M.with_child(c).into_widget();
  }
}
