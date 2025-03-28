use crate::{prelude::*, widget::Widget};
mod compose_child_impl;
mod multi_child_impl;
mod single_child_impl;
pub use compose_child_impl::*;
pub use multi_child_impl::*;
pub use single_child_impl::*;
pub mod into_child_compose;

/// The trait is for a widget that can have only one child.
///
/// Use `#[derive(SingleChild)]` for implementing this trait. It's best to use
/// the derive method first; manual implementation is not suggested unless you
/// fully understand how widget composition works in the framework.
pub trait SingleChild: IntoWidget<'static, RENDER> {
  /// Compose the child to a new widget.
  fn with_child<'c, const M: usize>(self, child: impl IntoChildSingle<'c, M>) -> Widget<'c>
  where
    Self: Sized;

  fn into_parent(self: Box<Self>) -> Widget<'static>;
}

/// The trait is for a widget that can have more than one children.
///
/// Use `#[derive(MultiChild)]` for implementing this trait. It's best to use
/// the derive method first; manual implementation is not suggested unless you
/// fully understand how widget composition works in the framework.
pub trait MultiChild: IntoWidget<'static, RENDER> {
  type Target<'c>
  where
    Self: Sized;

  fn with_child<'c, const N: usize, const M: usize>(
    self, child: impl IntoChildMulti<'c, N, M>,
  ) -> Self::Target<'c>
  where
    Self: Sized;

  fn into_parent(self: Box<Self>) -> Widget<'static>;
}

/// Trait for specifying the child type and defining how to compose the child.
///
/// ## Child Conversion
///
/// `ComposeChild` only accepts children that can be converted to
/// `ComposeChild::Child` by implementing `IntoChildCompose`. If the child is a
/// [`Template`], it allows for more flexibility.
///
/// ### Basic Conversion
///
/// The most basic child type is `Widget<'c>`, which automatically converts any
/// widget to it. This allows you to compose any widget.
///
///
/// ```rust
/// use ribir::prelude::*;
///
/// #[derive(Declare)]
/// struct X;
///
/// impl<'c> ComposeChild<'c> for X {
///   type Child = Widget<'c>;
///
///   fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
///     let mut w = FatObj::new(child);
///     w.background(Color::RED);
///     w.into_widget()
///   }
/// }
///
/// // You can compose `X` with any widget, and `X` will automatically apply a background color to it.
///
/// let _with_container = x! {
///   @Container {  size: Size::splat(100.) }
/// };
///
/// let _with_text = x! {
///   @Text { text: "Hi!" }
/// };
/// ```
///
/// If you want to compose a custom type, you can derive [`ChildOfCompose`] for
/// it to restrict composition to only that type. Additionally, you can
/// implement [`ComposeChildFrom`] to enable the composition of more types.
/// ```rust
/// use ribir::prelude::*;
///
/// #[derive(Declare)]
/// struct X;
///
/// #[derive(ChildOfCompose)]
/// struct A;
///
/// impl ComposeChild<'static> for X {
///   type Child = A;
///
///   fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'static> {
///     unimplemented!()
///   }
/// }
///
/// // Only A is supported as a child of X.
/// let _only_a = x! {
///   @ { A }
/// };
///
/// struct B;
///
/// impl ComposeChildFrom<B, 1> for A {
///   fn compose_child_from(_: B) -> Self { A }
/// }
///
/// // After implementing `ComposeChildFrom<B>` for `A`, now `B` can also be a child of `X`.
/// let _with_a = x! { @ { A } };
/// let _with_b = x! { @ { B } };
/// ```
///
/// ### Template Child
///
/// Templates outline the shape of children for `ComposeChild` and offer more
/// flexible child conversion.
/// ```rust
/// use ribir::prelude::*;
///
/// #[derive(Declare)]
/// struct X;
///
/// #[derive(ChildOfCompose)]
/// struct B;
///
/// #[derive(Template)]
/// struct XChild {
///   a: Widget<'static>,
///   b: Option<B>,
/// }
///
/// impl<'c> ComposeChild<'c> for X {
///   type Child = XChild;
///
///   fn compose_child(_: impl StateWriter<Value = Self>, _: Self::Child) -> Widget<'c> {
///     unimplemented!()
///   }
/// }
///
/// // The template child allows `X` to have two children: a widget and a `B`, where `B` is optional.
///
/// let _with_only_widget = x! { @Container { size: Size::splat(100.) } };
/// let _with_widget_and_b = x! {
///   @Container { size: Size::splat(100.) }
///   @ { B }
/// };
/// ```
///
/// Templates can also be enums, see [`Template`] for more details.
pub trait ComposeChild<'c>: Sized {
  type Child: 'c;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c>;

  /// Returns a builder for the child template.
  fn child_template() -> <Self::Child as Template>::Builder
  where
    Self::Child: Template,
  {
    <Self::Child as Template>::builder()
  }
}

/// The trait converts a type into a child of the `SingleChild`.
pub trait IntoChildSingle<'c, const M: usize> {
  fn into_child_single(self) -> Option<Widget<'c>>;
}

/// The trait converts a type into a child of the `MultiChild`.
pub trait IntoChildMulti<'c, const N: usize, const M: usize> {
  fn into_child_multi(self) -> impl Iterator<Item = Widget<'c>>;
}

/// Trait for conversions type as a child of widget. The opposite of
/// `ComposeChildFrom`.
///
/// You should not directly implement this trait. Instead, implement
/// `ComposeChildFrom`.
///
/// It is similar to `Into` but with a const marker to automatically implement
/// all possible conversions without implementing conflicts.
pub trait IntoChildCompose<C, const M: usize> {
  fn into_child_compose(self) -> C;
}

/// Used to do value-to-value conversions while consuming the input value. It is
/// the reciprocal of `IntoChildCompose`.
///
/// One should always prefer implementing `ComposeChildFrom` over
/// `IntoChildCompose`, because implementing `ComposeChildFrom` will
/// automatically implement `IntoChildCompose`.
pub trait ComposeChildFrom<C, const M: usize> {
  fn compose_child_from(from: C) -> Self;
}

/// Marker trait for types that can be used as children in composition
/// hierarchies.
///
/// This trait is typically implemented using `#[derive(ChildOfCompose)]`, which
/// automatically generates the [`ComposeChildFrom`] implementation for `Self`.
pub trait ChildOfCompose {}

/// A type-safe template system for constructing valid widget composition
/// hierarchies.
///
/// Provides compile-time validation of widget structure through composition
/// rules and type-driven child relationships. Templates serve as blueprint
/// definitions that:
/// - Define valid child configurations through type constraints
/// - Enable automatic widget conversions via [`ComposeChildFrom`]
/// - Support default value initialization for non-widget fields
///
/// # Key Features
///
/// - **Type-Checked Composition**: Enforces valid widget hierarchies at compile
///   time
/// - **Flexible Child Specification**: Supports both required and optional
///   children
/// - **Dual-Mode Definition**: Works with both structs (fixed layout) and enums
///   (variant selection)
/// - **Automatic Conversions**: Leverages Rust's type system for seamless child
///   conversion
///
/// # Implementation Mechanics
///
/// When derived via `#[derive(Template)]`, the system:
/// 1. Generates a builder interface with type-checked composition methods
/// 2. Enforces parent/child compatibility through trait bounds
/// 3. Automatically applies default values to non-widget fields
/// 4. Implements necessary conversion traits for child widgets
///
/// # Template Patterns
///
/// ## Basic Widget Composition
/// ```rust
/// use ribir::prelude::*;
///
/// #[derive(Declare)]
/// struct MyButton;
///
/// // Defines valid child configurations for MyButton
/// #[derive(Template)]
/// struct ButtonChild<'w> {
///   icon: Option<PairOf<'w, Icon>>, // Optional icon child
///   label: Option<CowArc<str>>,     // Optional text label
/// }
///
/// impl<'c> ComposeChild<'c> for MyButton {
///   type Child = ButtonChild<'c>;
///
///   fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
///     unimplemented!() // Actual composition logic
///   }
/// }
///
/// // Usage with text child (automatically converted to ButtonChild)
/// let _btn = fn_widget! {
///   @MyButton { @{ "Hi!" } }  // String converts to label via ComposeChildFrom
/// };
/// ```
///
/// ## Struct-Based Layout Definition
/// ```rust
/// use ribir::prelude::*;
///
/// #[derive(Template)]
/// struct Toolbar<'w> {
///   title: CowArc<str>,       // Mandatory text element
///   menu: Option<Widget<'w>>, // Optional widget slot
///   #[template(field = Color::BLUE)]
///   theme: Color, // Non-widget field with default
/// }
/// ```
///
/// ## Enum-Based Variant Selection
/// ```rust
/// use ribir::prelude::*;
///
/// #[derive(Template)]
/// enum ButtonContent<'w> {
///   Icon(PairOf<'w, Icon>), // Icon-only variant
///   Label(CowArc<str>),     // Text-only variant
///   Both {
///     // Combined variant
///     icon: PairOf<'w, Icon>,
///     label: CowArc<str>,
///   },
/// }
/// ```
pub trait Template: Sized {
  /// Type responsible for validating and constructing template instances
  type Builder: TemplateBuilder;

  /// Creates a configured builder instance ready for composition
  fn builder() -> Self::Builder;
}

/// The builder of a template.
pub trait TemplateBuilder: Sized {
  type Target;
  fn build_tml(self) -> Self::Target;
}

/// A pair of object and its child without compose, this keep the type
/// information of parent and child. `PairChild` and `ComposeChild` can create a
/// `Pair` with its child.
pub struct Pair<W, C> {
  parent: W,
  child: C,
}

/// A pair used to store a `ComposeChild` widget and its child. This preserves
/// the type information of both the parent and child without composition.
pub struct PairOf<'c, W: ComposeChild<'c>>(FatObj<Pair<State<W>, <W as ComposeChild<'c>>::Child>>);

pub trait TemplateFieldFrom<T, const M: usize> {
  fn template_field_from(from: T) -> Self;
}

pub trait TemplateFieldInto<T, const M: usize> {
  fn template_field_into(self) -> T;
}

impl<T, U> TemplateFieldFrom<U, 0> for T
where
  T: From<U>,
{
  fn template_field_from(from: U) -> Self { from.into() }
}

impl<T, U> TemplateFieldFrom<U, 1> for DeclareInit<T>
where
  DeclareInit<T>: DeclareFrom<U, 1>,
  T: From<U>,
{
  fn template_field_from(from: U) -> Self { DeclareInit::declare_from(from) }
}

impl<T, U> TemplateFieldFrom<U, 2> for DeclareInit<T>
where
  DeclareInit<T>: DeclareFrom<U, 2>,
  T: From<U>,
{
  fn template_field_from(from: U) -> Self { DeclareInit::declare_from(from) }
}

impl<T, U, const M: usize> TemplateFieldInto<U, M> for T
where
  U: TemplateFieldFrom<T, M>,
{
  fn template_field_into(self) -> U { U::template_field_from(self) }
}

impl IntoWidget<'static, RENDER> for Box<dyn MultiChild> {
  #[inline]
  fn into_widget(self) -> Widget<'static> { self.into_parent() }
}

impl IntoWidget<'static, RENDER> for Box<dyn SingleChild> {
  #[inline]
  fn into_widget(self) -> Widget<'static> { self.into_parent() }
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

impl<'c, W: ComposeChild<'c>> PairOf<'c, W> {
  pub fn parent(&self) -> &State<W> { &self.0.parent }
}

impl<'c, W> IntoWidget<'c, COMPOSE> for PairOf<'c, W>
where
  W: ComposeChild<'c> + 'static,
{
  #[inline]
  fn into_widget(self) -> Widget<'c> { self.0.into_widget() }
}

impl<'c, W, C, const M: usize> ComposeChildFrom<Pair<W, C>, M> for PairOf<'c, W>
where
  W: ComposeChild<'c> + 'static,
  C: IntoChildCompose<<W as ComposeChild<'c>>::Child, M>,
{
  fn compose_child_from(from: Pair<W, C>) -> Self {
    let Pair { parent, child } = from;
    Self(FatObj::new(Pair { parent: State::value(parent), child: child.into_child_compose() }))
  }
}

impl<'c, W, C, const M: usize> ComposeChildFrom<Pair<State<W>, C>, M> for PairOf<'c, W>
where
  W: ComposeChild<'c> + 'static,
  C: IntoChildCompose<<W as ComposeChild<'c>>::Child, M>,
{
  fn compose_child_from(from: Pair<State<W>, C>) -> Self {
    let Pair { parent, child } = from;
    Self(FatObj::new(Pair { parent, child: child.into_child_compose() }))
  }
}

impl<'c, W, C, const M: usize> ComposeChildFrom<FatObj<Pair<State<W>, C>>, M> for PairOf<'c, W>
where
  W: ComposeChild<'c> + 'static,
  C: IntoChildCompose<<W as ComposeChild<'c>>::Child, M>,
{
  fn compose_child_from(from: FatObj<Pair<State<W>, C>>) -> Self {
    let pair = from.map(|p| {
      let Pair { parent, child } = p;
      Pair { parent, child: child.into_child_compose() }
    });
    Self(pair)
  }
}

impl<T> ChildOfCompose for FatObj<T> {}

#[cfg(test)]
mod tests {
  use ribir_dev_helper::*;

  use super::*;
  use crate::{reset_test_env, test_helper::*};

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  #[allow(dead_code)]
  fn compose_template_child() {
    reset_test_env!();
    #[derive(Declare)]
    struct Page;

    #[derive(Template)]
    struct Header<'w>(Widget<'w>);

    #[derive(Template)]
    struct Content<'w>(Widget<'w>);

    #[derive(Template)]
    struct Footer<'w>(Widget<'w>);

    #[derive(Template)]
    struct PageTml<'w> {
      _header: Header<'w>,
      _content: Content<'w>,
      _footer: Footer<'w>,
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
    struct Child;

    impl<'c> ComposeChild<'c> for Parent {
      type Child = Option<Pair<Child, Widget<'c>>>;

      fn compose_child(_: impl StateWriter<Value = Self>, _: Self::Child) -> Widget<'c> {
        Void.into_widget()
      }
    }

    let _ = fn_widget! {
      @Parent {
        @ { Pair::new(Child, Void) }
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

    #[derive(Declare, ChildOfCompose)]
    struct A;
    #[derive(Declare, ChildOfCompose)]
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
        fn_widget! {
          @MockBox { size: if $c_size.area() > 0. { *$c_size } else { Size::new(1., 1.)} }
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
      let p = pipe!(($c_size.area() > 0.).then(|| {
        fn_widget! { @MockBox { size: Size::zero() }}
      }));
      @$p { @MockBox { size: Size::zero() } }
    };

    // option with `Widget`
    let _e = fn_widget! {
      let p = pipe!(($size.area() > 0.).then(|| {
        fn_widget! { @MockBox { size: Size::zero() }}
      }));
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

  #[derive(ChildOfCompose)]
  struct Field;

  #[derive(Template, Default)]
  pub struct ConfigTml {
    _field: Option<Field>,
  }
  #[derive(Declare)]
  struct Host {}

  impl ComposeChild<'static> for Host {
    type Child = ConfigTml;
    fn compose_child(_: impl StateWriter<Value = Self>, _: Self::Child) -> Widget<'static> {
      fn_widget! { @MockBox { size: FIX_OPTION_TEMPLATE_EXPECT_SIZE } }.into_widget()
    }
  }

  widget_layout_test!(
    template_option_field,
    WidgetTester::new(fn_widget! { @Host { @{ Field } }}),
    LayoutCase::default().with_size(FIX_OPTION_TEMPLATE_EXPECT_SIZE)
  );

  #[test]
  fn template_field() {
    #[derive(Template)]
    struct TemplateField {
      #[template(field = 0)]
      _x: i32,
      #[template(field)]
      _y: TextInit,
      _child: Widget<'static>,
    }

    #[derive(Declare)]
    struct X;

    impl ComposeChild<'static> for X {
      type Child = TemplateField;

      fn compose_child(_: impl StateWriter<Value = Self>, _child: Self::Child) -> Widget<'static> {
        unreachable!()
      }
    }

    let _ = fn_widget! {
      @X {
        @TemplateField {
          _y: "hi",
          // x is optional, is has a default value of 0
          // y: "hi",
          @Void {}
        }
      }
    };
  }
}
