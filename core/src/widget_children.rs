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

use crate::prelude::*;
/// Trait to tell Ribir a widget can have one child.
pub trait SingleChild {}

/// Trait to tell Ribir a widget can have multi child.
pub trait MultiChild {}

/// Trait mark widget can have one child and also have compose logic for widget
/// and its child.
pub trait ComposeChild: Sized {
  type Child;
  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget;
}

/// Trait specify what child a widget can have, and the target type after widget
/// compose its child.
pub trait WithChild<M, C> {
  type Target;
  type Builder;

  fn with_child(self, child: C) -> Self::Target;
  fn child_builder(&self) -> Self::Builder;
}

impl<W, C> WithChild<W, C> for W
where
  W: ComposeChild<Child = C>,
  C: Template,
{
  type Target = Widget;
  type Builder = C::Builder;

  #[inline]
  fn with_child(self, child: W::Child) -> Self::Target {
    ComposeChild::compose_child(StateWidget::Stateless(self), child)
  }
  #[inline]
  fn child_builder(&self) -> Self::Builder { <_>::default() }
}

impl<W, C> WithChild<&dyn Render, C> for W
where
  W: ComposeChild<Child = C>,
  C: Render,
{
  type Target = Widget;
  type Builder = AllowConvertTml<C>;

  #[inline]
  fn with_child(self, child: W::Child) -> Self::Target {
    ComposeChild::compose_child(StateWidget::Stateless(self), child)
  }
  #[inline]
  fn child_builder(&self) -> Self::Builder { AllowConvertTml(None) }
}

impl<W, C> WithChild<&dyn SingleChild, C> for W
where
  W: SingleChild,
{
  type Target = WidgetPair<Self, C>;
  type Builder = SingleTml<C>;

  #[inline]
  fn with_child(self, child: C) -> Self::Target { WidgetPair { widget: self, child } }
  #[inline]
  fn child_builder(&self) -> Self::Builder { SingleTml(None) }
}

impl<W: MultiChild> WithChild<&dyn MultiChild, Vec<Widget>> for W {
  type Target = WidgetPair<Self, Vec<Widget>>;
  type Builder = MultiTml;

  #[inline]
  fn with_child(self, child: Vec<Widget>) -> Self::Target { WidgetPair { widget: self, child } }
  #[inline]
  fn child_builder(&self) -> Self::Builder { MultiTml(vec![]) }
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
      children: Some(vec![child.into_widget()]),
    }
  }
}

impl<W, C, M> IntoWidget<Concrete<Option<M>>> for WidgetPair<W, DynWidget<Option<C>>>
where
  M: ImplMarker,
  WidgetPair<W, Option<C>>: IntoWidget<M>,
{
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    WidgetPair { widget, child: child.into_inner() }.into_widget()
  }
}

impl<W, C, M1, M2> IntoWidget<Concrete<Option<(M1, M2)>>> for WidgetPair<W, Option<C>>
where
  M1: ImplMarker,
  M2: ImplMarker,
  W: IntoWidget<M1>,
  WidgetPair<W, C>: IntoWidget<M2>,
{
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    if let Some(child) = child {
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
      children: Some(child),
    }
  }
}

impl<D, M1, M2, C> IntoWidget<Generic<(M1, M2)>> for WidgetPair<Stateful<DynWidget<D>>, C>
where
  D: DynsIntoWidget<SingleDyn<M1>>,
  M2: ImplMarker,
  WidgetPair<DynRender<D, SingleDyn<M1>>, C>: IntoWidget<M2>,
{
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    WidgetPair {
      widget: DynRender::new(widget),
      child,
    }
    .into_widget()
  }
}

impl<D, C, M> IntoWidget<Concrete<M>> for WidgetPair<DynWidget<D>, C>
where
  WidgetPair<D, C>: IntoWidget<Generic<M>>,
{
  #[inline]
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    WidgetPair { widget: widget.into_inner(), child }.into_widget()
  }
}

// Template use to construct child of a widget.
pub trait Template: Sized {
  type Builder: TemplateBuilder;
}

pub trait TemplateBuilder: Default + Sized {
  type Target;
  fn build(self) -> Self::Target;
}

pub trait FillTemplate<M: ImplMarker, W> {
  type New;
  fn fill(self, c: W) -> Self::New;
}

pub struct SingleTml<W>(Option<W>);
pub struct MultiTml(Vec<Widget>);
pub struct AllowConvertTml<W>(Option<W>);

pub struct DynTml<T>(T);

macro_rules! assert_assign_once {
  ($e: expr) => {
    assert!(
      $e.is_none(),
      "Give two child to a widget which allow only one."
    );
  };
}

// Not implement `TemplateBuilder` for  `SingleTml<W>` adn `MultiTml` because we
// only implement it for template of `ComposeChild`, so we distinguish if a
// template is use for `ComposeChild`
impl<W> SingleTml<W> {
  #[inline]
  pub fn build(self) -> W { self.0.expect("Template not be filled.") }
}

impl MultiTml {
  #[inline]
  pub fn build(self) -> Vec<Widget> { self.0 }
}

impl<W> FillTemplate<Concrete<W>, W> for SingleTml<W> {
  type New = Self;
  fn fill(mut self, c: W) -> Self {
    assert_assign_once!(self.0);
    self.0 = Some(c);
    self
  }
}

impl FillTemplate<Generic<Widget>, Widget> for MultiTml {
  type New = Self;
  fn fill(mut self, c: Widget) -> Self {
    self.0.push(c);
    self
  }
}

impl<Iter> FillTemplate<Generic<&dyn Iterator<Item = Widget>>, Iter> for MultiTml
where
  Iter: Iterator<Item = Widget>,
{
  type New = Self;
  #[inline]
  fn fill(mut self, c: Iter) -> Self {
    self.0.extend(c);
    self
  }
}

impl<W> Default for AllowConvertTml<W> {
  #[inline]
  fn default() -> Self { Self(Default::default()) }
}

impl<W> TemplateBuilder for AllowConvertTml<W> {
  type Target = W;
  #[inline]
  fn build(self) -> Self::Target { self.0.expect("Template not be filled.") }
}

impl<W> FillTemplate<Generic<W>, W> for AllowConvertTml<W> {
  type New = Self;
  fn fill(mut self, c: W) -> Self {
    assert_assign_once!(self.0);
    self.0 = Some(c);
    self
  }
}

impl Template for Widget {
  type Builder = AllowConvertTml<Widget>;
}

impl<W: Compose> Template for W {
  type Builder = AllowConvertTml<W>;
}

impl<W> Template for Option<W> {
  type Builder = AllowConvertTml<Option<W>>;
}

impl<W> FillTemplate<Generic<W>, W> for AllowConvertTml<Option<W>> {
  type New = Self;
  #[inline]
  fn fill(self, c: W) -> Self::New { self.fill(Some(c)) }
}

impl<W, C> Template for WidgetPair<W, C> {
  type Builder = AllowConvertTml<Self>;
}

impl<W> Template for Vec<W> {
  type Builder = AllowConvertTml<Vec<W>>;
}

impl<W> FillTemplate<Generic<W>, W> for AllowConvertTml<Vec<W>> {
  type New = Self;
  #[inline]
  fn fill(mut self, c: W) -> Self {
    self.0.get_or_insert_with(Vec::default).push(c);
    self
  }
}

impl<W, I> FillTemplate<Generic<&dyn Iterator<Item = W>>, I> for AllowConvertTml<Vec<W>>
where
  I: Iterator<Item = W>,
{
  type New = Self;
  #[inline]
  fn fill(mut self, c: I) -> Self {
    self.0.as_mut().unwrap().extend(c);
    self
  }
}

impl<M, Iter> FillTemplate<Generic<&dyn Iterator<Item = Generic<M>>>, Iter>
  for AllowConvertTml<Vec<Widget>>
where
  Iter: Iterator,
  Iter::Item: IntoWidget<Generic<M>>,
{
  type New = Self;
  #[inline]
  fn fill(mut self, c: Iter) -> Self {
    self
      .0
      .as_mut()
      .unwrap()
      .extend(c.map(IntoWidget::into_widget));
    self
  }
}

impl<M, Iter> FillTemplate<Generic<&dyn Iterator<Item = Generic<M>>>, Iter> for MultiTml
where
  Iter: Iterator,
  Iter::Item: IntoWidget<Generic<M>>,
{
  type New = Self;
  #[inline]
  fn fill(mut self, c: Iter) -> Self {
    self.0.extend(c.map(IntoWidget::into_widget));
    self
  }
}

impl<C> FillTemplate<Generic<Option<C>>, Option<C>> for AllowConvertTml<Vec<C>> {
  type New = Self;
  #[inline]
  fn fill(self, c: Option<C>) -> Self { if let Some(c) = c { self.fill(c) } else { self } }
}

impl<M, C> FillTemplate<Concrete<Option<M>>, Option<C>> for MultiTml
where
  Self: FillTemplate<Generic<M>, C, New = Self>,
{
  type New = Self;
  #[inline]
  fn fill(self, c: Option<C>) -> Self { if let Some(c) = c { self.fill(c) } else { self } }
}

impl<M, W, T, N> FillTemplate<Concrete<Generic<M>>, W> for T
where
  T: FillTemplate<Generic<Widget>, Widget, New = N>,
  W: IntoWidget<Generic<M>>,
{
  type New = N;
  #[inline]
  fn fill(self, c: W) -> Self::New { self.fill(c.into_widget()) }
}

impl<M, W, C, T, N> FillTemplate<Concrete<WidgetPair<W, M>>, WidgetPair<W, C>> for T
where
  T: FillTemplate<Generic<WidgetOf<W>>, WidgetOf<W>, New = N>,
  C: IntoWidget<Generic<M>>,
{
  type New = N;
  #[inline]
  fn fill(self, c: WidgetPair<W, C>) -> Self::New {
    let WidgetPair { widget, child } = c;
    self.fill(WidgetPair { widget, child: child.into_widget() })
  }
}

impl<M, D, T, N> FillTemplate<Concrete<&M>, DynWidget<D>> for T
where
  T: FillTemplate<Generic<M>, D, New = N>,
{
  type New = N;
  #[inline]
  fn fill(self, c: DynWidget<D>) -> Self::New { self.fill(c.into_inner()) }
}

impl<M, D> FillTemplate<Concrete<&dyn Iterator<Item = M>>, Stateful<DynWidget<D>>> for MultiTml
where
  M: ImplMarker + 'static,
  D: Iterator + 'static,
  D::Item: IntoWidget<M>,
{
  type New = Self;
  #[inline]
  fn fill(self, c: Stateful<DynWidget<D>>) -> Self { self.fill(DynRender::new(c).into_widget()) }
}

impl<M, D, T> FillTemplate<Concrete<&dyn Iterator<Item = M>>, Stateful<DynWidget<D>>> for T
where
  // `TemplateBuilder` only work for `ComposeChild`
  T: TemplateBuilder,
  M: ImplMarker + 'static,
  D: Iterator + 'static,
  D::Item: IntoWidget<M>,
{
  type New = Self;
  #[inline]
  fn fill(self, c: Stateful<DynWidget<D>>) -> Self { todo!() }
}
#[cfg(test)]
mod tests {
  use crate::test::*;

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
      states { size: size.clone() }
      DynWidget {
        dyns: if size.area() > 0. { MockMulti {} } else { MockMulti {} },
        MockBox { size: Size::zero() }
        MockBox { size: Size::zero() }
        MockBox { size: Size::zero() }
      }
    };

    // option with single child
    let _e = widget! {
      states { size: size.clone() }
      DynWidget {
        dyns: (size.area() > 0.).then(|| MockBox { size: Size::zero() }) ,
        MockBox { size: Size::zero() }
      }
    };

    // option with `Widget`
    let _e = widget! {
      states { size: size.clone() }
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
      P { MockBox { Void {} } }
    };
  }

  #[test]
  fn fix_multi_fill_for_pair() {
    let tml = AllowConvertTml::<WidgetPair<_, _>>(None);
    let child = MockBox { size: ZERO_SIZE }.with_child(Void.into_widget());
    tml.fill(child);
  }

  #[test]
  fn dyns_compose_child() {
    #[derive(Declare)]
    struct X;

    impl ComposeChild for X {
      type Child = MockBox;
      fn compose_child(_: StateWidget<Self>, child: Self::Child) -> Widget { child.into_widget() }
    }

    let dyns = DynWidget { dyns: Some(X) }.into_stateful();
    let size = Size::new(100., 200.);

    let w = ComposeChild::compose_child(StateWidget::Stateless(dyns), MockBox { size });
    expect_layout_result(
      w,
      None,
      &[LayoutTestItem {
        path: &[0],
        expect: ExpectRect::from_size(size),
      }],
    );
  }
}
