use crate::{prelude::*, widget::Widget};
mod compose_child_impl;
mod multi_child_impl;
mod single_child_impl;
pub use compose_child_impl::*;
pub use multi_child_impl::*;
pub use single_child_impl::*;
pub mod child_convert;
pub use child_convert::{ChildFrom, FromAnother, IntoChild};

/// Trait to mark a widget can have one widget as child.
pub trait SingleChild {}
/// Trait to tell Ribir a object that has multi children.
pub trait MultiChild {}
/// A boxed render widget that support accept one child.
#[derive(SingleChild)]
pub struct BoxedSingleChild(Widget);

/// A boxed render widget that support accept multi children.
#[derive(MultiChild)]
pub struct BoxedMultiChild(Widget);

/// This trait specifies the type of child a widget can have, and the target
/// type represents the result of the widget composing its child.
///
/// The N and M markers are used to avoid implementation conflicts. If Rust
/// supports generic specialization, we could avoid using them.
///
/// The M marker is used for child conversion.
/// The N marker is used to distinguish the parent type:
/// - 0 for SingleChild
/// - 1 for MultiChild
/// - 2..9 for ComposeChild
pub trait WithChild<C, const N: usize, const M: usize> {
  type Target;
  fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target;
}

// todo: remove it.
/// Trait to mark an object that it should compose with its child as a
/// `SinglePair` and the parent and child keep their type.
pub trait PairChild {}

/// Trait mark widget can have one child and also have compose logic for widget
/// and its child.
pub trait ComposeChild: Sized {
  type Child;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> impl WidgetBuilder;
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
  fn build(self, _: &BuildCtx) -> Widget { self.0 }
}

impl IntoWidgetStrict<RENDER> for BoxedMultiChild {
  #[inline]
  fn into_widget_strict(self, ctx: &BuildCtx) -> Widget { self.build(ctx) }
}

impl IntoWidgetStrict<RENDER> for BoxedSingleChild {
  #[inline]
  fn into_widget_strict(self, ctx: &BuildCtx) -> Widget { self.build(ctx) }
}

impl RenderBuilder for BoxedMultiChild {
  #[inline]
  fn build(self, _: &BuildCtx) -> Widget { self.0 }
}

impl BoxedSingleChild {
  #[inline]
  pub fn new(widget: impl SingleIntoParent, ctx: &BuildCtx) -> Self {
    Self(widget.into_parent(ctx))
  }
}

impl BoxedMultiChild {
  #[inline]
  pub fn new(widget: impl MultiIntoParent, ctx: &BuildCtx) -> Self { Self(widget.into_parent(ctx)) }
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

impl<W, C> Pair<FatObj<W>, C> {
  /// Replace the host of the FatObj in parent with the child, this is useful
  /// when the host of the `FatObj` is useless.
  pub fn child_replace_host(self) -> FatObj<C> {
    let Self { parent, child } = self;
    parent.map(|_| child)
  }
}

pub trait PairWithChild<C> {
  type Target;
  fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target;
}

impl<W: PairChild, C> PairWithChild<C> for W {
  type Target = Pair<W, C>;

  #[inline]
  #[track_caller]
  fn with_child(self, child: C, _: &BuildCtx) -> Self::Target { Pair { parent: self, child } }
}

impl<W, C1: PairChild, C2> PairWithChild<C2> for Pair<W, C1> {
  type Target = Pair<W, Pair<C1, C2>>;

  #[track_caller]
  fn with_child(self, c: C2, ctx: &BuildCtx) -> Self::Target {
    let Pair { parent: widget, child } = self;
    Pair { parent: widget, child: child.with_child(c, ctx) }
  }
}

#[cfg(test)]
mod tests {
  use ribir_dev_helper::*;

  use super::*;
  use crate::{reset_test_env, test_helper::*};

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn compose_template_child() {
    reset_test_env!();
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
      _header: WidgetOf<FatObj<Header>>,
      _content: WidgetOf<FatObj<Content>>,
      _footer: WidgetOf<FatObj<Footer>>,
    }

    impl ComposeChild for Page {
      type Child = PageTml;

      fn compose_child(_: impl StateWriter<Value = Self>, _: Self::Child) -> impl WidgetBuilder {
        fn_widget!(Void)
      }
    }

    let _ = fn_widget! {
      @Page {
        @Header { @Void {} }
        @Content { @Void {} }
        @Footer { @Void {} }
      }
    };
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn compose_option_child() {
    reset_test_env!();

    #[derive(Declare)]
    struct Parent;
    #[derive(Declare, SingleChild)]
    struct Child;

    impl ComposeChild for Parent {
      type Child = Option<Pair<FatObj<Child>, Widget>>;

      fn compose_child(_: impl StateWriter<Value = Self>, _: Self::Child) -> impl WidgetBuilder {
        fn_widget!(Void)
      }
    }

    let _ = fn_widget! {
      @Parent {
        @Child { @Void {} }
      }
    };
  }
  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn compose_option_dyn_parent() {
    reset_test_env!();

    let _ = fn_widget! {
      let p = Some(MockBox { size: Size::zero() });
      @$p { @{ Void } }
    };
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn tuple_as_vec() {
    reset_test_env!();

    #[derive(Declare)]
    struct A;
    #[derive(Declare)]
    struct B;

    impl ComposeChild for A {
      type Child = Vec<B>;

      fn compose_child(_: impl StateWriter<Value = Self>, _: Self::Child) -> impl WidgetBuilder {
        fn_widget!(Void)
      }
    }
    let a = A;
    let _ = fn_widget! {
      @$a {
        @ { B}
        @ { B }
      }
    };
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn expr_with_child() {
    reset_test_env!();

    let size = Stateful::new(Size::zero());
    let c_size = size.clone_watcher();
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

    let c_size = size.clone_watcher();
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

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
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

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn pair_to_pair() {
    reset_test_env!();

    #[derive(Declare)]
    struct P;

    #[derive(Declare, SingleChild)]
    struct X;

    impl ComposeChild for P {
      type Child = WidgetOf<FatObj<X>>;
      fn compose_child(_: impl StateWriter<Value = Self>, _: Self::Child) -> impl WidgetBuilder {
        fn_widget!(Void)
      }
    }

    let _ = fn_widget! {
      @P { @X { @Void {} } }
    };
  }
  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn fix_multi_fill_for_pair() {
    reset_test_env!();

    struct X;
    impl ComposeChild for X {
      type Child = Widget;
      fn compose_child(
        _: impl StateWriter<Value = Self>, child: Self::Child,
      ) -> impl WidgetBuilder {
        fn_widget!(child)
      }
    }

    let _ = |ctx| -> Widget {
      let child = MockBox { size: ZERO_SIZE }.with_child(Void, ctx);
      X.with_child(child, ctx).build(ctx)
    };
  }

  const FIX_OPTION_TEMPLATE_EXPECT_SIZE: Size = Size::new(100., 200.);
  fn fix_option_template() -> impl WidgetBuilder {
    struct Field;

    #[derive(Template, Default)]
    pub struct ConfigTml {
      _field: Option<Field>,
    }
    #[derive(Declare)]
    struct Host {}

    impl ComposeChild for Host {
      type Child = Option<ConfigTml>;
      fn compose_child(_: impl StateWriter<Value = Self>, _: Self::Child) -> impl WidgetBuilder {
        fn_widget! { @MockBox { size: FIX_OPTION_TEMPLATE_EXPECT_SIZE } }
      }
    }

    fn_widget! { @Host { @{ Field } }}
  }
  widget_layout_test!(
    fix_option_template,
    { path = [0], size == FIX_OPTION_TEMPLATE_EXPECT_SIZE, }
  );
}
