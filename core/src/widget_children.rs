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
pub use child_convert::{ChildFrom, FromAnother};
/// Trait to tell Ribir a object can have one child.
pub trait SingleChild {}

/// A boxed render widget that support accept one child.
pub trait BoxedSingleParent {
  fn into_parent(self: Box<Self>, ctx: &mut BuildCtx) -> WidgetId;
}

/// Trait to tell Ribir a object that has multi children.
pub trait MultiChild {}

/// A boxed render widget that support accept multi children.
pub trait BoxMultiParent {
  fn into_parent(self: Box<Self>, ctx: &mut BuildCtx) -> WidgetId;
}

/// Trait mark widget can have one child and also have compose logic for widget
/// and its child.
pub trait ComposeChild: Sized {
  type Child;
  fn compose_child(this: State<Self>, child: Self::Child) -> Widget;
}

/// A alias of `WidgetPair<W, Widget>`, means `Widget` is the
/// child of the generic type.
pub type WidgetOf<W> = SinglePair<W, Widget>;

impl SingleChild for Box<dyn BoxedSingleParent> {}
impl MultiChild for Box<dyn BoxMultiParent> {}

impl RenderParent for Box<dyn BoxedSingleParent> {
  fn into_render_parent(self, ctx: &mut BuildCtx) -> WidgetId { self.into_parent(ctx) }
}

impl RenderParent for Box<dyn BoxMultiParent> {
  fn into_render_parent(self, ctx: &mut BuildCtx) -> WidgetId { self.into_parent(ctx) }
}

pub(crate) trait RenderParent {
  fn into_render_parent(self, ctx: &mut BuildCtx) -> WidgetId;
}

impl<T: Into<Box<dyn Render>>> RenderParent for T {
  #[inline]
  fn into_render_parent(self, ctx: &mut BuildCtx) -> WidgetId { ctx.alloc_widget(self.into()) }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_helper::*;
  use crate::widget::WidgetBuilder;
  use ribir_dev_helper::*;
  use std::{cell::RefCell, rc::Rc};

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
      type Child = Option<SinglePair<Child, Widget>>;

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
        widget::from(Void)
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
      fn compose_child(_: State<Self>, _: Self::Child) -> Widget { Void.into() }
    }

    let _ = FnWidget::new(|ctx| {
      let child = MockBox { size: ZERO_SIZE }.with_child(Void, ctx);
      X.with_child(child, ctx).build(ctx)
    });
  }

  fn dyns_compose_child() -> Widget {
    #[derive(Declare)]
    struct X;

    impl ComposeChild for X {
      type Child = MockBox;
      fn compose_child(_: State<Self>, child: Self::Child) -> Widget { child.into() }
    }

    let dyns = Stateful::new(DynWidget { dyns: Some(X) });
    let size = Size::new(100., 200.);

    ComposeChild::compose_child(State::<X>::from(dyns), MockBox { size })
  }
  widget_layout_test!(dyns_compose_child, width == 100., height == 200.,);

  const COMPOSE_DYNS_CHILD_SIZE: Size = Size::new(100., 200.);
  fn compose_dyns_child() -> Widget {
    #[derive(Declare)]
    struct AcceptStateChild;

    impl ComposeChild for AcceptStateChild {
      type Child = State<MockBox>;
      fn compose_child(_: State<Self>, child: Self::Child) -> Widget { child.into() }
    }

    let trigger = Stateful::new(true);

    widget! {
      states { trigger: trigger }
      AcceptStateChild {
        DynWidget {
          dyns: if *trigger {
            MockBox { size: COMPOSE_DYNS_CHILD_SIZE }
          } else {
            MockBox { size: ZERO_SIZE }
          }
        }
      }
    }
    .into()
  }
  widget_layout_test!(compose_dyns_child, size == COMPOSE_DYNS_CHILD_SIZE,);

  const FIX_OPTION_TEMPLATE_EXPECT_SIZE: Size = Size::new(100., 200.);
  fn fix_option_template() -> impl Into<Widget> {
    struct Field(String);

    #[derive(Template, Default)]
    pub struct ConfigTml {
      _field: Option<Field>,
    }
    #[derive(Declare)]
    struct Host {}

    impl ComposeChild for Host {
      type Child = Option<ConfigTml>;
      fn compose_child(_: State<Self>, _: Self::Child) -> Widget {
        widget! { MockBox { size: FIX_OPTION_TEMPLATE_EXPECT_SIZE } }.into()
      }
    }

    widget! { Host { Field("test".into()) }}
  }
  widget_layout_test!(
    fix_option_template,
    { path = [0], size == FIX_OPTION_TEMPLATE_EXPECT_SIZE, }
  );

  #[test]
  fn compose_dyn_multi_child() {
    let _guard = unsafe { AppCtx::new_lock_scope() };

    struct A;

    impl ComposeChild for A {
      type Child = Vec<Widget>;

      fn compose_child(_: State<Self>, child: Self::Child) -> Widget {
        FnWidget::new(move |ctx| MockMulti.with_child(Multi::new(child), ctx).build(ctx)).into()
      }
    }

    let child = DynWidget { dyns: Some(Multi::new([Void])) };
    let child = Stateful::new(child);
    let cnt = Rc::new(RefCell::new(0));
    let c_cnt = cnt.clone();
    child
      .modifies()
      .subscribe(move |_| *c_cnt.borrow_mut() += 1);

    let _ = TestWindow::new(FnWidget::new(|ctx| A.with_child(child, ctx).build(ctx)));
    assert_eq!(*cnt.borrow(), 0);
  }

  #[test]
  fn compose_multi_with_stateful_option() {
    struct M;
    impl ComposeChild for M {
      type Child = Vec<Widget>;
      fn compose_child(_: State<Self>, _: Self::Child) -> Widget { Void.into() }
    }

    let _ = FnWidget::new(|ctx| {
      let c = Stateful::new(DynWidget { dyns: Some(Some(Void)) });
      M.with_child(c, ctx).build(ctx)
    });
  }
}
