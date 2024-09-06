use crate::{prelude::*, widget::Widget};
mod compose_child_impl;
mod multi_child_impl;
mod single_child_impl;
pub use compose_child_impl::*;
pub use multi_child_impl::*;
pub use single_child_impl::*;
pub mod child_convert;
pub use child_convert::IntoChild;

/// Trait to mark a widget can have one widget as child.
pub trait SingleChild {}
/// Trait to tell Ribir a object that has multi children.
pub trait MultiChild {}
/// A boxed render widget that support accept one child.
#[derive(SingleChild)]
pub struct BoxedSingleChild(Widget<'static>);

/// A boxed render widget that support accept multi children.
#[derive(MultiChild)]
pub struct BoxedMultiChild(Widget<'static>);

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
pub trait WithChild<'w, C, const N: usize, const M: usize> {
  type Target: 'w;
  fn with_child(self, child: C) -> Self::Target;
}

/// Trait for specifying the child type and implementing how to compose the
/// child.
pub trait ComposeChild<'c>: Sized {
  type Child: 'c;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c>;
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
pub type WidgetOf<'a, W> = Pair<W, Widget<'a>>;

impl IntoWidgetStrict<'static, RENDER> for BoxedMultiChild {
  #[inline]
  fn into_widget_strict(self) -> Widget<'static> { self.0 }
}

impl IntoWidgetStrict<'static, RENDER> for BoxedSingleChild {
  #[inline]
  fn into_widget_strict(self) -> Widget<'static> { self.0 }
}

impl BoxedSingleChild {
  #[inline]
  pub fn new(widget: impl SingleIntoParent) -> Self { Self(widget.into_parent()) }
}

impl BoxedMultiChild {
  #[inline]
  pub fn new(widget: impl MultiIntoParent) -> Self { Self(widget.into_parent()) }
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
    struct PageTml<'w> {
      _header: WidgetOf<'w, FatObj<Header>>,
      _content: WidgetOf<'w, FatObj<Content>>,
      _footer: WidgetOf<'w, FatObj<Footer>>,
    }

    impl<'c> ComposeChild<'c> for Page {
      type Child = PageTml<'c>;

      fn compose_child(_: impl StateWriter<Value = Self>, _: Self::Child) -> Widget<'c> {
        Void.into_widget()
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

    impl<'c> ComposeChild<'c> for Parent {
      type Child = Option<Pair<FatObj<Child>, Widget<'c>>>;

      fn compose_child(_: impl StateWriter<Value = Self>, _: Self::Child) -> Widget<'c> {
        Void.into_widget()
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

    impl ComposeChild<'static> for A {
      type Child = Vec<B>;

      fn compose_child(_: impl StateWriter<Value = Self>, _: Self::Child) -> Widget<'static> {
        Void.into_widget()
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

    impl<'c> ComposeChild<'c> for P {
      type Child = WidgetOf<'c, FatObj<X>>;
      fn compose_child(_: impl StateWriter<Value = Self>, _: Self::Child) -> Widget<'c> {
        Void.into_widget()
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
    impl<'c> ComposeChild<'c> for X {
      type Child = Widget<'c>;
      fn compose_child(_: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
        child
      }
    }

    let _ = |_: &BuildCtx| -> Widget {
      let child = MockBox { size: ZERO_SIZE }.with_child(Void);
      X.with_child(child).into_widget()
    };
  }

  const FIX_OPTION_TEMPLATE_EXPECT_SIZE: Size = Size::new(100., 200.);
  struct Field;

  #[derive(Template, Default)]
  pub struct ConfigTml {
    _field: Option<Field>,
  }
  #[derive(Declare)]
  struct Host {}

  impl ComposeChild<'static> for Host {
    type Child = Option<ConfigTml>;
    fn compose_child(_: impl StateWriter<Value = Self>, _: Self::Child) -> Widget<'static> {
      fn_widget! { @MockBox { size: FIX_OPTION_TEMPLATE_EXPECT_SIZE } }.into_widget()
    }
  }

  widget_layout_test!(
    fix_option_template,
    WidgetTester::new(fn_widget! { @Host { @{ Field } }}),
    LayoutCase::default().with_size(FIX_OPTION_TEMPLATE_EXPECT_SIZE)
  );
}
