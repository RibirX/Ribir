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

use crate::{
  prelude::*,
  widget::{StrictBuilder, WidgetBuilder},
};
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
  fn boxed_append_child(self: Box<Self>, child: WidgetId, ctx: &BuildCtx) -> WidgetId;
  fn boxed_build(self: Box<Self>, ctx: &BuildCtx) -> WidgetId;
}

/// Trait to tell Ribir a object that has multi children.
pub trait MultiChild {}

/// A boxed render widget that support accept multi children.
pub trait BoxedMultiParent {
  fn boxed_append_children(self: Box<Self>, children: Vec<WidgetId>, ctx: &BuildCtx) -> WidgetId;
  fn boxed_build(self: Box<Self>, ctx: &BuildCtx) -> WidgetId;
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
impl MultiChild for Box<dyn BoxedMultiParent> {}

impl StrictBuilder for Box<dyn BoxedSingleParent> {
  #[inline]
  fn strict_build(self, ctx: &BuildCtx) -> WidgetId { self.boxed_build(ctx) }
}

impl SingleParent for Box<dyn BoxedSingleParent> {
  #[inline]
  fn append_child(self, child: WidgetId, ctx: &BuildCtx) -> WidgetId {
    self.boxed_append_child(child, ctx)
  }
}

impl StrictBuilder for Box<dyn BoxedMultiParent> {
  #[inline]
  fn strict_build(self, ctx: &BuildCtx) -> WidgetId { self.boxed_build(ctx) }
}

impl MultiParent for Box<dyn BoxedMultiParent> {
  #[inline]
  fn append_children(self, children: Vec<WidgetId>, ctx: &BuildCtx) -> WidgetId {
    self.boxed_append_children(children, ctx)
  }
}

pub(crate) trait SingleParent {
  fn append_child(self, child: WidgetId, ctx: &BuildCtx) -> WidgetId;
}

pub(crate) trait MultiParent {
  fn append_children(self, children: Vec<WidgetId>, ctx: &BuildCtx) -> WidgetId;
}

impl<T: Into<Box<dyn Render>> + SingleChild> SingleParent for T {
  fn append_child(self, child: WidgetId, ctx: &BuildCtx) -> WidgetId {
    let p = ctx.alloc_widget(self.into());
    ctx.append_child(p, child);
    p
  }
}

impl<T: Into<Box<dyn Render>> + MultiChild> MultiParent for T {
  #[inline]
  fn append_children(self, children: Vec<WidgetId>, ctx: &BuildCtx) -> WidgetId {
    let p = ctx.alloc_widget(self.into());
    for c in children {
      ctx.append_child(p, c);
    }
    p
  }
}

impl<W: SingleParent + WidgetBuilder + 'static> BoxedSingleParent for W {
  #[inline]
  fn boxed_append_child(self: Box<Self>, child: WidgetId, ctx: &BuildCtx) -> WidgetId {
    (*self).append_child(child, ctx)
  }

  #[inline]
  fn boxed_build(self: Box<Self>, ctx: &BuildCtx) -> WidgetId { (*self).build(ctx) }
}

impl<W: MultiParent + StrictBuilder + 'static> BoxedMultiParent for W {
  #[inline]
  fn boxed_append_children(self: Box<Self>, children: Vec<WidgetId>, ctx: &BuildCtx) -> WidgetId {
    (*self).append_children(children, ctx)
  }

  #[inline]
  fn boxed_build(self: Box<Self>, ctx: &BuildCtx) -> WidgetId { (*self).build(ctx) }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::widget::StrictBuilder;
  use crate::{reset_test_env, test_helper::*};
  use ribir_dev_helper::*;

  #[test]
  fn compose_template_child() {
    reset_test_env!();
    #[derive(Declare2)]
    struct Page;
    #[derive(Declare2, SingleChild)]
    struct Header;
    #[derive(Declare2, SingleChild)]
    struct Content;
    #[derive(Declare2, SingleChild)]
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

    fn_widget! {
      @Page {
        @Header { @Void {} }
        @Content { @Void {} }
        @Footer { @Void {} }
      }
    };
  }

  #[test]
  fn compose_option_child() {
    reset_test_env!();

    #[derive(Declare2)]
    struct Parent;
    #[derive(Declare2, SingleChild)]
    struct Child;

    impl ComposeChild for Parent {
      type Child = Option<SinglePair<Child, Widget>>;

      fn compose_child(_: State<Self>, _: Self::Child) -> Widget {
        unreachable!("Only for syntax support check");
      }
    }

    fn_widget! {
      @Parent {
        @Child { @Void {} }
      }
    };
  }

  #[test]
  fn compose_option_dyn_parent() {
    reset_test_env!();

    fn_widget! {
      let p = Some(MockBox { size: Size::zero() });
      @$p { @{ Void } }
    };
  }

  #[test]
  fn tuple_as_vec() {
    reset_test_env!();

    #[derive(Declare2)]
    struct A;
    #[derive(Declare2)]
    struct B;

    impl ComposeChild for A {
      type Child = Vec<B>;

      fn compose_child(_: State<Self>, _: Self::Child) -> Widget {
        unreachable!("Only for syntax support check");
      }
    }
    let a = A;
    fn_widget! {
      @$a {
        @ { B}
        @ { B }
      }
    };
  }

  #[test]
  fn expr_with_child() {
    reset_test_env!();

    let size = Stateful::new(Size::zero());
    let c_size = size.clone_reader();
    // with single child
    let _e = fn_widget! {
      let p = pipe!{
        if $c_size.area() > 0. {
          MockBox { size: *$c_size }
        } else {
          MockBox { size: Size::new(1., 1.) }
        }
      };
      @$p { @MockBox { size: pipe!(*$c_size) } }
    };

    // with multi child
    let _e = fn_widget! {
      @MockMulti {
        @MockBox { size: Size::zero() }
        @MockBox { size: Size::zero() }
        @MockBox { size: Size::zero() }
      }
    };

    let c_size = size.clone_reader();
    // option with single child
    let _e = fn_widget! {
      let p = pipe!(($c_size.area() > 0.).then(|| @MockBox { size: Size::zero() }));
      @$p { @MockBox { size: Size::zero() } }
    };

    // option with `Widget`
    let _e = fn_widget! {
      let p = pipe!(($size.area() > 0.).then(|| @MockBox { size: Size::zero() }));
      @$p { @ { Void }}
    };
  }

  #[test]
  fn compose_expr_option_widget() {
    reset_test_env!();

    let _ = fn_widget! {
      @MockBox {
        size: ZERO_SIZE,
        @{ Some(@MockBox { size: Size::zero() })}
      }
    };
  }

  #[test]
  fn pair_to_pair() {
    reset_test_env!();

    #[derive(Declare2)]
    struct P;

    impl ComposeChild for P {
      type Child = WidgetOf<State<MockBox>>;
      fn compose_child(_: State<Self>, _: Self::Child) -> Widget { unreachable!() }
    }

    let _ = fn_widget! {
      @P { @MockBox { @Void {} } }
    };
  }

  #[test]
  fn fix_multi_fill_for_pair() {
    reset_test_env!();

    struct X;
    impl ComposeChild for X {
      type Child = WidgetOf<MockBox>;
      fn compose_child(_: State<Self>, _: Self::Child) -> Widget { Void.into() }
    }

    let _ = FnWidget::new(|ctx| {
      let child = MockBox { size: ZERO_SIZE }.with_child(Void, ctx);
      X.with_child(child, ctx).strict_build(ctx)
    });
  }

  const FIX_OPTION_TEMPLATE_EXPECT_SIZE: Size = Size::new(100., 200.);
  fn fix_option_template() -> impl Into<Widget> {
    struct Field(String);

    #[derive(Template, Default)]
    pub struct ConfigTml {
      _field: Option<Field>,
    }
    #[derive(Declare2)]
    struct Host {}

    impl ComposeChild for Host {
      type Child = Option<ConfigTml>;
      fn compose_child(_: State<Self>, _: Self::Child) -> Widget {
        fn_widget! { @MockBox { size: FIX_OPTION_TEMPLATE_EXPECT_SIZE } }.into()
      }
    }

    fn_widget! { @Host { @{ Field("test".into()) } }}
  }
  widget_layout_test!(
    fix_option_template,
    { path = [0], size == FIX_OPTION_TEMPLATE_EXPECT_SIZE, }
  );
}
