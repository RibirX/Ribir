use crate::prelude::*;
/// Trait to tell Ribir a widget can have one child.
pub trait SingleChild {}

/// Trait to tell Ribir a widget can have multi child.
pub trait MultiChild {}

/// Trait mark widget can have one child and also have compose logic for widget
/// and its child.
pub trait ComposeChild: Sized {
  type Child: AssociatedTemplate;
  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget;
}

/// Trait specify what child a widget can have, and the target type after widget
/// compose its child.
pub trait WithChild<M, T: Template> {
  type Target;
  fn with_child(self, child: T::Target) -> Self::Target;

  #[inline]
  fn child_template(&self) -> T { Template::empty() }
}

impl<W, T> WithChild<W, T> for W
where
  W: ComposeChild,
  W::Child: AssociatedTemplate<T = T>,
  T: Template<Target = W::Child>,
{
  type Target = Widget;
  #[inline]
  fn with_child(self, child: T::Target) -> Self::Target {
    ComposeChild::compose_child(StateWidget::Stateless(self), child)
  }
}

impl<W, C> WithChild<&dyn SingleChild, ConcreteTml<C>> for W
where
  W: SingleChild,
{
  type Target = WidgetPair<Self, C>;
  #[inline]
  fn with_child(self, child: C) -> Self::Target { WidgetPair { widget: self, child } }

  #[inline]
  fn child_template(&self) -> ConcreteTml<C> { Template::empty() }
}

impl<W: MultiChild> WithChild<&dyn MultiChild, Vec<Widget>> for W {
  type Target = WidgetPair<Self, Vec<Widget>>;
  #[inline]
  fn with_child(self, child: Vec<Widget>) -> Self::Target { WidgetPair { widget: self, child } }
}

impl<M, D, T> WithChild<DynWidget<M>, T> for DynWidget<D>
where
  D: WithChild<M, T>,
  T: Template,
{
  type Target = D::Target;

  fn with_child(self, child: T::Target) -> Self::Target { self.into_inner().with_child(child) }
}

impl<M1, M2, M3, D, T> WithChild<DynWidget<(M1, M2, M3)>, T> for Stateful<DynWidget<D>>
where
  D: WithChild<M1, T>,
  T: Template,
  Self: IntoDynRender<M2, D>,
  M3: ImplMarker,
  WidgetPair<DynRender<D>, T::Target>: IntoWidget<M3>,
{
  type Target = Widget;

  fn with_child(self, child: T::Target) -> Self::Target {
    WidgetPair {
      widget: self.into_dyn_render(),
      child,
    }
    .into_widget()
  }
}

impl<D: SingleChild> SingleChild for Option<D> {}

/// A node of widget with not compose its child.
pub struct WidgetPair<W, C> {
  pub widget: W,
  pub child: C,
}

/// A alias of `WidgetPair<W, Widget>`, means `Widget` is the child of the
/// generic type.
pub type WidgetOf<W> = WidgetPair<W, Widget>;

// implementation of IntoWidget
impl<W, C, M> IntoWidget<Generic<M>> for WidgetPair<W, C>
where
  M: ImplMarker,
  W: Render + 'static,
  C: IntoWidget<M>,
{
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    Widget::Render {
      render: Box::new(widget),
      children: vec![child.into_widget()],
    }
  }
}

impl<W, C, M1, M2> IntoWidget<Generic<(M1, M2)>> for WidgetPair<W, DynWidget<Option<C>>>
where
  M1: ImplMarker,
  M2: ImplMarker,
  W: IntoWidget<M1>,
  WidgetPair<W, C>: IntoWidget<M2>,
{
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    if let Some(child) = child.into_inner() {
      WidgetPair { widget, child }.into_widget()
    } else {
      widget.into_widget()
    }
  }
}

impl<W> IntoWidget<Generic<Self>> for WidgetPair<W, Vec<Widget>>
where
  W: Render + 'static,
{
  #[inline]
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    Widget::Render {
      render: Box::new(widget),
      children: child,
    }
  }
}

pub trait AssociatedTemplate: Sized {
  type T: Template<Target = Self>;
}
// Template use to construct child of a widget.
pub trait Template: Sized {
  type Target;
  fn empty() -> Self;
  fn build(self) -> Self::Target;
}

pub trait FillTemplate<M: ImplMarker, W>: Template {
  fn fill(self, c: W) -> Self;
}

pub struct ConcreteTml<W>(Option<W>);
pub struct WidgetTml(Option<Widget>);
pub struct WidgetPairTml<W, C>(Option<WidgetPair<W, C>>);

macro_rules! assert_impl_tml {
  () => {
    #[inline]
    fn empty() -> Self { Self(None) }

    #[inline]
    fn build(self) -> Self::Target { self.0.expect("concrete template not be filled.") }
  };
}

macro_rules! assert_assign_once {
  ($e: expr) => {
    assert!(
      $e.is_none(),
      "Give two child to a widget which allow only one."
    );
  };
}

// impl concrete template
impl<W> Template for ConcreteTml<W> {
  type Target = W;
  assert_impl_tml!();
}

impl<W> FillTemplate<Concrete<W>, W> for ConcreteTml<W> {
  fn fill(mut self, c: W) -> Self {
    assert_assign_once!(self.0);
    self.0 = Some(c);
    self
  }
}

// widget template
impl AssociatedTemplate for Widget {
  type T = WidgetTml;
}

impl Template for WidgetTml {
  type Target = Widget;
  assert_impl_tml!();
}

impl FillTemplate<Generic<Widget>, Widget> for WidgetTml {
  fn fill(mut self, w: Widget) -> Self {
    assert_assign_once!(self.0);
    self.0 = Some(w);
    self
  }
}

// option template
impl<W> AssociatedTemplate for Option<W> {
  type T = Self;
}

impl<W> Template for Option<W> {
  type Target = Option<W>;
  #[inline]
  fn empty() -> Self { None }

  #[inline]
  fn build(self) -> Self::Target { self }
}

impl<W> FillTemplate<Generic<W>, W> for Option<W> {
  fn fill(mut self, c: W) -> Self {
    assert_assign_once!(self);
    self = Some(c);
    self
  }
}

// WidgetPair template
impl<W, C> AssociatedTemplate for WidgetPair<W, C> {
  type T = WidgetPairTml<W, C>;
}

impl<W, C> Template for WidgetPairTml<W, C> {
  type Target = WidgetPair<W, C>;
  assert_impl_tml!();
}

impl<W, C> FillTemplate<Generic<WidgetPair<W, C>>, WidgetPair<W, C>> for WidgetPairTml<W, C> {
  fn fill(mut self, c: WidgetPair<W, C>) -> Self {
    assert_assign_once!(self.0);
    self.0 = Some(c);
    self
  }
}

// vec template
impl<W> AssociatedTemplate for Vec<W> {
  type T = Self;
}

impl<W> Template for Vec<W> {
  type Target = Self;
  #[inline]
  fn empty() -> Self { vec![] }

  #[inline]
  fn build(self) -> Self::Target { self }
}

impl<W> FillTemplate<Generic<W>, W> for Vec<W> {
  #[inline]
  fn fill(mut self, c: W) -> Self {
    self.push(c);
    self
  }
}

impl<M, W, T> FillTemplate<Concrete<Generic<M>>, W> for T
where
  T: FillTemplate<Generic<Widget>, Widget>,
  W: IntoWidget<Generic<M>>,
{
  #[inline]
  fn fill(self, c: W) -> Self { self.fill(c.into_widget()) }
}

impl<M, T, D> FillTemplate<Concrete<M>, Stateful<DynWidget<D>>> for T
where
  T: FillTemplate<Generic<Widget>, Widget>,
  Stateful<DynWidget<D>>: IntoDynRender<M, D>,
  D: 'static,
{
  #[inline]
  fn fill(self, c: Stateful<DynWidget<D>>) -> Self { self.fill(c.into_dyn_render().into_widget()) }
}

impl<M, W, C, T> FillTemplate<Concrete<WidgetPair<W, M>>, WidgetPair<W, C>> for T
where
  T: FillTemplate<Generic<WidgetPair<W, Widget>>, WidgetPair<W, Widget>>,
  M: ImplMarker,
  C: IntoWidget<M>,
{
  #[inline]
  fn fill(self, c: WidgetPair<W, C>) -> Self {
    let WidgetPair { widget, child } = c;
    self.fill(WidgetPair { widget, child: child.into_widget() })
  }
}

impl<M, D, T> FillTemplate<Concrete<&M>, DynWidget<D>> for T
where
  T: FillTemplate<Generic<M>, D>,
{
  #[inline]
  fn fill(self, c: DynWidget<D>) -> Self { self.fill(c.into_inner()) }
}

impl<M, D> FillTemplate<Concrete<DynWidget<M>>, DynWidget<D>> for Vec<Widget>
where
  M: ImplMarker,
  D: IntoIterator,
  D::Item: IntoWidget<M>,
{
  #[inline]
  fn fill(mut self, c: DynWidget<D>) -> Self {
    self.extend(c.into_inner().into_iter().map(IntoWidget::into_widget));
    self
  }
}

#[cfg(test)]
mod tests {
  use crate::test::{MockBox, MockMulti};

  use super::*;

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

      fn compose_child(_: StateWidget<Self>, _: Self::Child) -> Widget {
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

      fn compose_child(_: StateWidget<Self>, _: Self::Child) -> Widget {
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
  fn tuple_as_vec() {
    #[derive(Declare)]
    struct A;
    #[derive(Declare)]
    struct B;

    impl ComposeChild for A {
      type Child = Vec<B>;

      fn compose_child(_: StateWidget<Self>, _: Self::Child) -> Widget {
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
    let size = Size::zero().into_stateful();
    // with single child
    let _e = widget! {
      track { size: size.clone() }
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
      track { size: size.clone() }
      DynWidget {
        dyns: if size.area() > 0. { MockMulti {} } else { MockMulti {} },
        MockBox { size: Size::zero() }
        MockBox { size: Size::zero() }
        MockBox { size: Size::zero() }
      }
    };

    // option with single child
    let _e = widget! {
      track { size: size.clone() }
      DynWidget {
        dyns: (size.area() > 0.).then(|| MockBox { size: Size::zero() }) ,
        MockBox { size: Size::zero() }
      }
    };

    // option with `Widget`
    let _e = widget! {
      track { size: size.clone() }
      DynWidget {
        dyns: (size.area() > 0.).then(|| MockBox { size: Size::zero() }) ,
        DynWidget { dyns: Void.into_widget() }
      }
    };
  }

  #[test]
  fn compose_const_dyn_option_widget() {
    let _ = widget! {
      MockBox {
        size: ZERO_SIZE,
        Option::Some(MockBox { size: Size::zero() })
      }
    };
  }

  #[test]
  fn pair_to_pair() {
    #[derive(Declare)]
    struct P;

    impl ComposeChild for P {
      type Child = WidgetOf<MockBox>;
      fn compose_child(_: StateWidget<Self>, _: Self::Child) -> Widget { unreachable!() }
    }

    let _ = widget! {
      P { MockBox {Void {} } }
    };
  }
}
