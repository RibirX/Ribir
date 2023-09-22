use crate::{
  prelude::*,
  widget::{Widget, WidgetBuilder},
};
mod compose_child_impl;
mod multi_child_impl;
mod single_child_impl;
pub use compose_child_impl::*;
pub use multi_child_impl::*;
pub use single_child_impl::*;
pub mod child_convert;
pub use child_convert::{ChildFrom, FromAnother};

/// Trait to mark a widget can have one child.
pub trait SingleChild {}

/// A boxed render widget that support accept one child.
pub struct BoxedSingleChild(Widget);

/// Trait to tell Ribir a object that has multi children.
pub trait MultiChild {}

/// A boxed render widget that support accept multi children.
pub struct BoxedMultiChild(Widget);

/// Trait to mark an object that it should compose with its child as a
/// `SinglePair` and the parent and child keep their type.
pub trait PairChild {}

/// Trait mark widget can have one child and also have compose logic for widget
/// and its child.
pub trait ComposeChild: Sized {
  type Child;
  fn compose_child(this: State<Self>, child: Self::Child) -> impl WidgetBuilder;
}

/// A pair of object and its child without compose, this keep the type
/// information of parent and child. `PairChild` and `ComposeChild` can create a
/// `Pair` with its child.
pub struct Pair<W, C> {
  parent: W,
  child: C,
}

/// A alias of `Pair<W, Widget>`, means `Widget` is the child of the generic
/// type.
pub type WidgetOf<W> = Pair<W, Widget>;

impl RenderBuilder for BoxedSingleChild {
  #[inline]
  fn widget_build(self, _: &BuildCtx) -> Widget { self.0 }
}

impl SingleParent for BoxedSingleChild {
  #[inline]
  fn compose_child(self, child: Widget, ctx: &BuildCtx) -> Widget {
    ctx.append_child(self.0.id(), child);
    self.0
  }
}

impl RenderBuilder for BoxedMultiChild {
  #[inline]
  fn widget_build(self, _: &BuildCtx) -> Widget { self.0 }
}

impl MultiParent for BoxedMultiChild {
  fn compose_children(self, children: impl Iterator<Item = Widget>, ctx: &BuildCtx) -> Widget {
    for c in children {
      ctx.append_child(self.0.id(), c)
    }
    self.0
  }
}

/// The trait to help `SingleChild` to compose child so the `SingleChild` no
/// need to expose the compose logic. If user want have its own compose logic,
/// use `ComposeChild` instead.
pub(crate) trait SingleParent {
  fn compose_child(self, child: Widget, ctx: &BuildCtx) -> Widget;
}

/// The trait to help `MultiParent` to compose child so the `MultiParent` no
/// need to expose the compose logic. If user want have its own compose logic,
/// use `ComposeChild` instead.
pub(crate) trait MultiParent {
  fn compose_children(self, children: impl Iterator<Item = Widget>, ctx: &BuildCtx) -> Widget;
}

impl<T: Render + SingleChild> SingleParent for T {
  fn compose_child(self, child: Widget, ctx: &BuildCtx) -> Widget {
    let p = self.widget_build(ctx);
    ctx.append_child(p.id(), child);

    p
  }
}

impl<T: SingleParent> SingleParent for Option<T> {
  fn compose_child(self, child: Widget, ctx: &BuildCtx) -> Widget {
    if let Some(this) = self {
      this.compose_child(child, ctx)
    } else {
      child
    }
  }
}

impl<T: Render + MultiChild> MultiParent for T {
  fn compose_children(self, children: impl Iterator<Item = Widget>, ctx: &BuildCtx) -> Widget {
    let p = self.widget_build(ctx);
    for c in children {
      ctx.append_child(p.id(), c);
    }
    p
  }
}

impl BoxedSingleChild {
  #[inline]
  pub fn new(widget: impl RenderBuilder + SingleChild, ctx: &BuildCtx) -> Self {
    Self(widget.widget_build(ctx))
  }

  /// Create a `BoxedSingleChild` from a `Widget` and not check the type , the
  /// caller should make sure the `w` is build from a `SingleChild`.
  #[inline]
  pub(crate) fn from_id(w: Widget) -> Self { Self(w) }
}

impl BoxedMultiChild {
  #[inline]
  pub fn new(widget: impl RenderBuilder + MultiChild, ctx: &BuildCtx) -> Self {
    Self(widget.widget_build(ctx))
  }
}

impl<W, C> Pair<W, C> {
  #[inline]
  pub fn new(parent: W, child: C) -> Self { Self { parent, child } }

  #[inline]
  pub fn unzip(self) -> (W, C) {
    let Self { parent: widget, child } = self;
    (widget, child)
  }

  #[inline]
  pub fn child(self) -> C { self.child }

  #[inline]
  pub fn parent(self) -> W { self.parent }
}

pub trait PairWithChild<C> {
  type Target;
  fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target;
}

impl<W: PairChild, C> PairWithChild<C> for W {
  type Target = Pair<W, C>;

  #[inline]
  fn with_child(self, child: C, _: &BuildCtx) -> Self::Target { Pair { parent: self, child } }
}

impl<W, C1: PairChild, C2> PairWithChild<C2> for Pair<W, C1> {
  type Target = Pair<W, Pair<C1, C2>>;

  fn with_child(self, c: C2, ctx: &BuildCtx) -> Self::Target {
    let Pair { parent: widget, child } = self;
    Pair {
      parent: widget,
      child: child.with_child(c, ctx),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{reset_test_env, test_helper::*};
  use ribir_dev_helper::*;

  #[test]
  fn compose_template_child() {
    reset_test_env!();
    #[derive(Declare2)]
    struct Page;
    #[derive(Declare2, PairChild)]
    struct Header;
    #[derive(Declare2, PairChild)]
    struct Content;
    #[derive(Declare2, PairChild)]
    struct Footer;

    #[derive(Template)]
    struct PageTml {
      _header: WidgetOf<Header>,
      _content: WidgetOf<Content>,
      _footer: WidgetOf<Footer>,
    }

    impl ComposeChild for Page {
      type Child = PageTml;

      fn compose_child(_: State<Self>, _: Self::Child) -> impl WidgetBuilder { fn_widget!(Void) }
    }

    let _ = fn_widget! {
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
    #[derive(Declare2, PairChild)]
    struct Child;

    impl ComposeChild for Parent {
      type Child = Option<Pair<Child, Widget>>;

      fn compose_child(_: State<Self>, _: Self::Child) -> impl WidgetBuilder { fn_widget!(Void) }
    }

    let _ = fn_widget! {
      @Parent {
        @Child { @Void {} }
      }
    };
  }

  #[test]
  fn compose_option_dyn_parent() {
    reset_test_env!();

    let _ = fn_widget! {
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

      fn compose_child(_: State<Self>, _: Self::Child) -> impl WidgetBuilder { fn_widget!(Void) }
    }
    let a = A;
    let _ = fn_widget! {
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

    #[derive(Declare2, PairChild)]
    struct X;

    impl ComposeChild for P {
      type Child = WidgetOf<X>;
      fn compose_child(_: State<Self>, _: Self::Child) -> impl WidgetBuilder { fn_widget!(Void) }
    }

    let _ = fn_widget! {
      @P { @X { @Void {} } }
    };
  }

  #[test]
  fn fix_multi_fill_for_pair() {
    reset_test_env!();

    struct X;
    impl ComposeChild for X {
      type Child = Widget;
      fn compose_child(_: State<Self>, child: Self::Child) -> impl WidgetBuilder {
        fn_widget!(child)
      }
    }

    let _ = |ctx| -> Widget {
      let child = MockBox { size: ZERO_SIZE }.with_child(Void, ctx);
      X.with_child(child, ctx).widget_build(ctx)
    };
  }

  const FIX_OPTION_TEMPLATE_EXPECT_SIZE: Size = Size::new(100., 200.);
  fn fix_option_template() -> impl WidgetBuilder {
    struct Field(String);

    #[derive(Template, Default)]
    pub struct ConfigTml {
      _field: Option<Field>,
    }
    #[derive(Declare2)]
    struct Host {}

    impl ComposeChild for Host {
      type Child = Option<ConfigTml>;
      fn compose_child(_: State<Self>, _: Self::Child) -> impl WidgetBuilder {
        fn_widget! { @MockBox { size: FIX_OPTION_TEMPLATE_EXPECT_SIZE } }
      }
    }

    fn_widget! { @Host { @{ Field("test".into()) } }}
  }
  widget_layout_test!(
    fix_option_template,
    { path = [0], size == FIX_OPTION_TEMPLATE_EXPECT_SIZE, }
  );
}
