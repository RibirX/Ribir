use crate::{pipe::*, prelude::*, widget::Widget};
mod compose_child_impl;
mod multi_child_impl;
mod single_child_impl;
pub use compose_child_impl::*;
pub use multi_child_impl::*;
pub use single_child_impl::*;

/// Trait marking widgets that enforce single-child composition semantics.
///
/// Use `#[derive(SingleChild)]` for implementations this trait.
pub trait SingleChild: Sized {
  fn with_child<'c, K>(self, child: impl RInto<OptionWidget<'c>, K>) -> SinglePair<'c, Self> {
    SinglePair { parent: self, child: child.r_into().0 }
  }
}

/// The trait is for a widget that can have more than one children.
///
/// Use `#[derive(MultiChild)]` for implementing this trait.
pub trait MultiChild: Sized {
  fn with_child<'c, K: ?Sized>(self, children: impl IntoWidgetIter<'c, K>) -> MultiPair<'c, Self> {
    let children = children.into_widget_iter().collect();
    MultiPair { parent: self, children }
  }
}

/// Defines how a widget composes its children, specifying accepted child types
/// and composition logic.
///
/// This trait enables two fundamental child composition strategies:
///
/// 1. **Direct Conversion**: Accepts any type that can be converted into the
///    [`ComposeChild::Child`] type via the [`RInto`] trait. This provides
///    flexibility in child type acceptance.
/// 2. **Template-Based**: Uses a dedicated [`Template`] type to define
///    structured child requirements, enabling complex child configurations with
///    elements and type-safe validation.
///
/// # Implementing Composition
///
/// ## Basic Direct Conversion
///
///
/// ```rust
/// use ribir::prelude::*;
///
/// #[declare]
/// struct RedBackground;
///
/// impl<'c> ComposeChild<'c> for RedBackground {
///   type Child = Widget<'c>;
///
///   fn compose_child(_: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
///     let mut w = FatObj::new(child);
///     w.with_background(Color::RED); // Apply styling to composed child
///     w.into_widget()
///   }
/// }
///
/// // Usage examples:
/// let _red_container = red_background! { @Container { size: Size::new(100., 100.) } };
/// let _red_text = red_background! { @Text { text: "Red Text!" } };
/// ```
///
/// ## Template-Based Composition
///
/// For complex child structures, define a [`Template`] type to specify required
/// child elements witch different types.
///
/// ```rust
/// use ribir::prelude::*;
///
/// #[declare]
/// struct Dashboard;
///
/// // Header, Content, and Footer just for example here.
/// struct Header;
/// struct Content;
/// struct Footer;
///
/// #[derive(Template)]
/// struct DashboardChildren {
///   header: Header,
///   content: Content,
///   footer: Option<Footer>,
/// }
///
/// impl<'c> ComposeChild<'c> for Dashboard {
///   type Child = DashboardChildren;
///
///   fn compose_child(_: impl StateWriter<Value = Self>, children: Self::Child) -> Widget<'c> {
///     // Implementation would arrange header/content/footer in a layout
///     unimplemented!()
///   }
/// }
///
/// // Valid compositions:
/// let _basic_dashboard = dashboard! {
///   @ { Header }
///   @ { Content }
///   @ { Footer }
/// };
/// ```
///
/// Templates can also be enums for alternative child configurations (see
/// [`Template`] documentation).
pub trait ComposeChild<'c>: Sized {
  /// The type of child(ren) this widget accepts.
  type Child: 'c;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c>;

  /// Creates a builder for template-based child composition.
  ///
  /// Only available when [`Self::Child`] implements [`Template`].
  /// This provides type-safe construction of complex child structures.
  fn child_builder() -> <Self::Child as Template>::Builder
  where
    Self::Child: Template,
  {
    <Self::Child as Template>::builder()
  }
}

pub type OptionWidget<'c> = OptionBuilder<Widget<'c>>;

pub trait IntoWidgetIter<'w, K: ?Sized> {
  fn into_widget_iter(self) -> impl Iterator<Item = Widget<'w>>;
}
/// Defines type-safe widget composition templates for [`ComposeChild`].
///
/// Enables structured child validation through two primary patterns:
/// - **Struct Templates**: Define fixed layouts with required/optional fields
/// - **Enum Templates**: Support alternative child configurations via variants
///
/// # Core Concepts
/// - **Compile-Time Validation**: Ensures valid widget hierarchies at compile
///   time
/// - **Flexible Composition**: Combines direct widgets with structured data
///   fields
/// - **Automatic Conversion**: Leverages Rust's type system for seamless child
///   adoption
///
/// # Implementation Details
///
/// ## Struct Templates
/// Deriving `Template` on a struct:
/// 1. Generates type-checked builder for field assignment
/// 2. Enforces child/parent compatibility through trait bounds
/// 3. Provides default values for non-widget fields
/// 4. Automatically implements child conversion traits
///
/// ```rust
/// use ribir::prelude::*;
///
/// #[derive(Template)]
/// struct FormField<'w> {
///   label: CowArc<str>,
///   input: Widget<'w>,
///   help_text: Option<Widget<'w>>,
/// }
/// ```
///
/// ## Enum Templates
/// Deriving `Template` on an enum:
/// 1. Implements `RFrom` for all variant types
/// 2. Enables direct use of variant types as children
/// 3. Supports flexible configuration patterns
///
/// ```rust
/// use ribir::prelude::*;
///
/// #[derive(Template)]
/// enum ContentBlock {
///   Text(CowArc<str>),
///   Media(Widget<'static>),
///   Mixed { text: CowArc<str>, image: Widget<'static> },
/// }
/// ```
///
/// # Usage Examples
///
/// ## Basic Struct Template
///
/// ```rust
/// use ribir::prelude::*;
///
/// #[derive(Template)]
/// struct ButtonContent<'w> {
///   icon: Option<Widget<'w>>,
///   label: CowArc<str>,
/// }
///
/// #[declare]
/// struct MyButton;
///
/// impl ComposeChild<'static> for MyButton {
///   type Child = ButtonContent<'static>;
///
///   fn compose_child(_: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'static> {
///     // Layout implementation combining icon and label
///     unimplemented!()
///   }
/// }
///
/// // String literal automatically converts to ButtonContent
/// let btn = my_button! { @{ "Submit" } };
/// ```
///
/// ## Advanced Struct Template
/// ```rust
/// use ribir::prelude::*;
///
/// #[declare]
/// struct ArticleCard;
///
/// impl ComposeChild<'static> for ArticleCard {
///   type Child = Summary;
///
///   fn compose_child(_: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'static> {
///     unimplemented!()
///   }
/// }
///
/// #[derive(Template)]
/// struct Summary {
///   title: CowArc<str>,
///   #[template(field = 3usize)]
///   max_lines: usize,
/// }
///
/// let card = article_card! {
///   @Summary {
///     max_lines: 2usize,
///     @ { "Title" }
///   }
/// };
/// ```
///
/// When the `Summary` struct contains additional fields, explicit declaration
/// is required. The string content remains declared as a child element.
///
/// ## Enum Template Variants
/// ```rust
/// use ribir::prelude::*;
///
/// #[derive(Template)]
/// enum ButtonContent<'w> {
///   Icon(PairOf<'w, Icon>),
///   Label(CowArc<str>),
///   Combined { icon: PairOf<'w, Icon>, label: CowArc<str> },
/// }
/// ```
///
/// This enum template enables `ComposeChild` implementations accepting
/// `ButtonContent` to directly receive any of the following as children:
/// - `@Icon { ... }`
/// - `@ { "text" }`
/// - `@ButtonContent::XX` variants
pub trait Template {
  /// Builder type for constructing validated template instances
  type Builder: TemplateBuilder;

  /// Creates a configured builder for template construction
  fn builder() -> Self::Builder
  where
    Self: Sized;
}

/// The builder of a template.
pub trait TemplateBuilder: Default {
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
pub struct PairOf<'c, W: ComposeChild<'c>>(
  pub(super) FatObj<Pair<Stateful<W>, <W as ComposeChild<'c>>::Child>>,
);

impl<'w> OptionWidget<'w> {
  pub fn unwrap_or_void(self) -> Widget<'w> { self.0.unwrap_or_else(|| Void.into_widget()) }
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
  pub fn parent(&self) -> &Stateful<W> { &self.0.parent }

  pub fn unzip(self) -> (FatObj<Stateful<W>>, W::Child) {
    let (pair, fat) = self.0.into_parts();
    let (parent, child) = pair.unzip();
    (fat.with_child(parent), child)
  }

  pub fn into_fat_widget(self) -> FatObj<Widget<'c>>
  where
    W: 'static,
  {
    self.0.map(IntoWidget::into_widget)
  }
}

// ----- Parent Implementations --------

/// A parent widget wrapper that assists child composition for [`SingleChild`]
/// or [`MultiChild`].
///
/// This type enables proper child management while hiding implementation
/// details about how parent-child widget relationships are maintained. The
/// framework automatically provides [`From`] conversions for valid parent
/// widgets, so you shouldn't need to implement this manually.
pub(crate) trait Parent {
  fn with_children<'w>(self, children: Vec<Widget<'w>>) -> Widget<'w>
  where
    Self: 'w;
}

pub(crate) trait BoxedParent {
  fn boxed_with_children<'w>(self: Box<Self>, children: Vec<Widget<'w>>) -> Widget<'w>
  where
    Self: 'w;
}

pub(crate) trait XParent {
  fn x_with_children<'w>(self, children: Vec<Widget<'w>>) -> Widget<'w>
  where
    Self: 'w;
}

impl<P> Parent for P
where
  P: IntoWidget<'static, OtherWidget<dyn Render>>,
{
  fn with_children<'w>(self, children: Vec<Widget<'w>>) -> Widget<'w>
  where
    Self: 'w,
  {
    let p = self.into_widget();
    if !children.is_empty() { Widget::new(p, children) } else { p }
  }
}

impl<P: XParent> Parent for FatObj<P> {
  fn with_children<'w>(self, children: Vec<Widget<'w>>) -> Widget<'w>
  where
    Self: 'w,
  {
    self
      .map(|p| p.x_with_children(children))
      .compose()
  }
}

impl<P: XParent + 'static> Parent for Pipe<P> {
  fn with_children<'w>(self, children: Vec<Widget<'w>>) -> Widget<'w>
  where
    Self: 'w,
  {
    self.with_children(children)
  }
}

impl<F: FnOnce() -> P, P: XParent> Parent for FnWidget<P, F> {
  fn with_children<'w>(self, children: Vec<Widget<'w>>) -> Widget<'w>
  where
    Self: 'w,
  {
    FnWidget::new(move || self.call().x_with_children(children)).into_widget()
  }
}

impl<P: Parent> BoxedParent for P {
  fn boxed_with_children<'w>(self: Box<Self>, children: Vec<Widget<'w>>) -> Widget<'w>
  where
    Self: 'w,
  {
    (*self).with_children(children)
  }
}

impl<P: Parent> XParent for P {
  fn x_with_children<'w>(self, children: Vec<Widget<'w>>) -> Widget<'w>
  where
    Self: 'w,
  {
    self.with_children(children)
  }
}

impl<'p> XParent for XSingleChild<'p> {
  #[inline]
  fn x_with_children<'w>(self, children: Vec<Widget<'w>>) -> Widget<'w>
  where
    Self: 'w,
  {
    (self.0).boxed_with_children(children)
  }
}

impl<'p> XParent for XMultiChild<'p> {
  #[inline]
  fn x_with_children<'w>(self, children: Vec<Widget<'w>>) -> Widget<'w>
  where
    Self: 'w,
  {
    (self.0).boxed_with_children(children)
  }
}

impl<'c, W> RFrom<PairOf<'c, W>, OtherWidget<dyn Compose>> for Widget<'c>
where
  W: ComposeChild<'c> + 'static,
{
  fn r_from(value: PairOf<'c, W>) -> Self {
    value
      .0
      .map(|p| {
        let (parent, child) = p.unzip();
        ComposeChild::compose_child(parent, child)
      })
      .into_widget()
  }
}

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
    #[declare]
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

    #[declare]
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
      @(p) { @ { Void } }
    };
  }

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
  #[test]
  fn tuple_as_vec() {
    reset_test_env!();

    #[declare]
    struct A;
    #[declare]
    struct B;

    impl ComposeChild<'static> for A {
      type Child = Vec<B>;

      fn compose_child(_: impl StateWriter<Value = Self>, _: Self::Child) -> Widget<'static> {
        Void.into_widget()
      }
    }
    let a = A;
    let _ = fn_widget! {
      @(a) {
        @ { B }
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
        @MockBox { size: if $read(c_size).area() > 0. { *$read(c_size) } else { Size::new(1., 1.)} }
      };
      @(p) { @MockBox { size: pipe!(*$read(c_size)) } }
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
      let p = pipe!(($read(c_size).area() > 0.).then(|| {
        fn_widget! { @MockBox { size: Size::zero() }}
      }));
      @(p) { @MockBox { size: Size::zero() } }
    };

    // option with `Widget`
    let _e = fn_widget! {
      let p = pipe!(($read(size).area() > 0.).then(|| {
        fn_widget! { @MockBox { size: Size::zero() }}
      }));
      @(p) { @ { Void }}
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

  struct Field;

  #[derive(Template, Default)]
  pub struct ConfigTml {
    _field: Option<Field>,
  }
  #[declare]
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
  #[allow(dead_code)]
  fn template_field() {
    #[derive(Template)]
    struct TemplateField {
      #[template(field = 0)]
      x: i32,
      #[template(field)]
      y: TextValue,
      child: Widget<'static>,
    }

    #[declare]
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
          y: "hi",
          // x is optional, is has a default value of 0
          // y: "hi",
          @Void {}
        }
      }
    };
  }
}
