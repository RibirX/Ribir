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
/// Trait to tell Ribir a widget can have one child.
pub trait SingleChild {}

/// Trait to tell Ribir a widget can have multi child.
pub trait MultiChild {}

/// Trait mark widget can have one child and also have compose logic for widget
/// and its child.
pub trait ComposeChild: Sized {
  type Child;
  fn compose_child(this: State<Self>, child: Self::Child) -> Widget;
}

/// Trait specify what child a widget can have, and the target type after widget
/// compose its child.
pub trait WithChild<M, C> {
  type Target;
  fn with_child(self, child: C) -> Self::Target;
}

/// A node of widget with not compose its child.
pub struct WidgetPair<W, C> {
  pub widget: W,
  pub child: C,
}

/// A alias of `WidgetPair<W, Widget>`, means `Widget` is the child of the
/// generic type.
pub type WidgetOf<W> = WidgetPair<W, Widget>;

// implementation of IntoWidget
impl<W, C, M> IntoWidget<NotSelf<M>> for WidgetPair<W, C>
where
  W: Render + 'static,
  M: ImplMarker,
  C: IntoWidget<M>,
{
  fn into_widget(self) -> Widget {
    let WidgetPair { widget, child } = self;
    Widget::Render {
      render: Box::new(widget),
      children: Some(vec![child.into_widget()]),
    }
  }
}

impl<W, C, M> IntoWidget<NotSelf<Option<M>>> for WidgetPair<W, DynWidget<Option<C>>>
where
  M: ImplMarker,
  WidgetPair<W, Option<C>>: IntoWidget<M>,
{
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    WidgetPair { widget, child: child.into_inner() }.into_widget()
  }
}

impl<W, C, M1, M2> IntoWidget<NotSelf<&(M1, M2)>> for WidgetPair<W, Option<C>>
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

impl<W> IntoWidget<NotSelf<&dyn MultiChild>> for WidgetPair<W, Vec<Widget>>
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

impl<W, C> IntoWidget<NotSelf<[(); 1]>> for WidgetPair<W, C>
where
  W: ComposeChild<Child = C>,
{
  #[inline]
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    ComposeChild::compose_child(State::Stateless(widget), child)
  }
}

impl<W, M, T> IntoWidget<NotSelf<[M; 2]>> for WidgetPair<W, T>
where
  M: ImplMarker,
  T: TemplateBuilder,
  WidgetPair<W, T::Target>: IntoWidget<M>,
{
  #[inline]
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    WidgetPair { widget, child: child.build_tml() }.into_widget()
  }
}

impl<W, M, T> IntoWidget<NotSelf<[M; 3]>> for WidgetPair<W, T>
where
  M: ImplMarker,
  T: TemplateBuilder,
  WidgetPair<W, Option<T::Target>>: IntoWidget<M>,
{
  #[inline]
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    WidgetPair {
      widget,
      child: Some(child.build_tml()),
    }
    .into_widget()
  }
}

impl<D, M1, M2, C> IntoWidget<NotSelf<(M1, M2)>> for WidgetPair<Stateful<DynWidget<D>>, C>
where
  D: DynsIntoWidget<SingleDyn<M1>>,
  WidgetPair<DynRender<D, SingleDyn<M1>>, C>: IntoWidget<M2>,
  M2: ImplMarker,
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

impl<D, C, M> IntoWidget<NotSelf<(M,)>> for WidgetPair<DynWidget<D>, C>
where
  WidgetPair<D, C>: IntoWidget<NotSelf<M>>,
{
  #[inline]
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    WidgetPair { widget: widget.into_inner(), child }.into_widget()
  }
}

trait FillVec<M, Item> {
  fn fill_vec(&mut self, item: Item);
}

impl<M, Item, Item2> FillVec<&M, Item2> for Vec<Item>
where
  Item2: CommonChildConvert<M, Item>,
{
  #[inline]
  fn fill_vec(&mut self, item: Item2) { self.push(item.common_convert()) }
}

impl<M, D, Item> FillVec<(M,), DynWidget<D>> for Vec<Item>
where
  D: IntoIterator,
  Self: FillVec<M, D>,
{
  #[inline]
  fn fill_vec(&mut self, item: DynWidget<D>) { self.fill_vec(item.into_inner()) }
}

impl<M, Iter, Item> FillVec<[M; 0], Iter> for Vec<Item>
where
  Iter: IntoIterator,
  Iter::Item: CommonChildConvert<M, Item>,
{
  #[inline]
  fn fill_vec(&mut self, item: Iter) {
    self.extend(item.into_iter().map(CommonChildConvert::common_convert));
  }
}

impl<W, C, M> WithChild<[M; 0], C> for W
where
  W: ComposeChild,
  C: CommonChildConvert<M, W::Child>,
{
  type Target = WidgetPair<W, W::Child>;

  #[inline]
  fn with_child(self, child: C) -> Self::Target {
    WidgetPair {
      widget: self,
      child: child.common_convert(),
    }
  }
}

impl<W, C, Item, M> WithChild<[M; 1], Item> for W
where
  W: ComposeChild<Child = Vec<C>>,
  Vec<C>: FillVec<M, Item>,
{
  type Target = WidgetPair<W, W::Child>;

  #[inline]
  fn with_child(self, child: Item) -> Self::Target {
    let mut vec: Vec<C> = vec![];
    vec.fill_vec(child);
    WidgetPair { widget: self, child: vec }
  }
}

impl<W, C, Item, M> WithChild<[M; 1], Item> for WidgetPair<W, Vec<C>>
where
  Vec<C>: FillVec<M, Item>,
{
  type Target = Self;

  #[inline]
  fn with_child(mut self, child: Item) -> Self::Target {
    self.child.fill_vec(child);
    self
  }
}

impl<W, M1, M2, C> WithChild<[(M1, M2); 2], C> for W
where
  W: ComposeChild + WithChild<M1, Widget>,
  C: IntoWidget<OptionDyn<M2>>,
{
  type Target = W::Target;

  fn with_child(self, child: C) -> Self::Target { self.with_child(child.into_widget()) }
}

impl<W, C, M, Builder> WithChild<[M; 3], C> for W
where
  W: ComposeChild,
  W::Child: Template<Builder = Builder>,
  Builder: FillTml<M, C>,
{
  type Target = WidgetPair<W, Builder::New>;

  #[inline]
  fn with_child(self, c: C) -> Self::Target {
    let child = W::Child::builder().fill(c);
    WidgetPair { widget: self, child }
  }
}

impl<W, C, T, M> WithChild<[M; 4], C> for W
where
  W: ComposeChild<Child = Option<T>>,
  T: Template,
  T::Builder: FillTml<M, C>,
{
  type Target = WidgetPair<W, <T::Builder as FillTml<M, C>>::New>;

  #[inline]
  fn with_child(self, c: C) -> Self::Target {
    let child = T::builder().fill(c);
    WidgetPair { widget: self, child }
  }
}

impl<W, T, C, M> WithChild<[M; 5], C> for WidgetPair<W, T>
where
  T: FillTml<M, C>,
{
  type Target = WidgetPair<W, T::New>;

  #[inline]
  fn with_child(self, c: C) -> Self::Target {
    let Self { widget, child } = self;
    WidgetPair { widget, child: child.fill(c) }
  }
}

impl<W, C, M> WithChild<[M; 6], C> for W
where
  W: TemplateBuilder + FillTml<M, C>,
  W::New: TemplateBuilder,
{
  type Target = W::New;

  #[inline]
  fn with_child(self, c: C) -> Self::Target { self.fill(c) }
}

impl<W, C, M> WithChild<[M; 7], C> for WidgetPair<W, Vec<Widget>>
where
  C: IntoWidget<OptionDyn<M>>,
{
  type Target = Self;

  #[inline]
  fn with_child(mut self, c: C) -> Self::Target {
    self.child.push(c.into_widget());
    self
  }
}

impl<W, D, C, M1, M2> WithChild<[(M1, M2); 8], Stateful<DynWidget<D>>> for W
where
  W: ComposeChild<Child = Vec<C>> + 'static,
  D: CommonChildConvert<M2, W::Child> + 'static,
  C: NotWidget<M1>,
{
  type Target = Widget;

  #[inline]
  fn with_child(self, child: Stateful<DynWidget<D>>) -> Self::Target {
    let this = Stateful::new(self);
    widget! {
      states { child }
      DynWidget {
        dyns: {
          let child = child.silent().dyns.take().unwrap().common_convert();
          ComposeChild::compose_child(State::Stateful(this.clone()), child)
        }
      }
    }
  }
}

impl<W, D, M> WithChild<[M; 9], Stateful<DynWidget<D>>> for W
where
  W: ComposeChild<Child = D> + 'static,
  D: NotWidget<M> + 'static,
{
  type Target = Widget;

  #[inline]
  fn with_child(self, child: Stateful<DynWidget<D>>) -> Self::Target {
    let this = Stateful::new(self);
    widget! {
      states { child }
      DynWidget {
        dyns: {
          let child = child.silent().dyns.take().unwrap();
          ComposeChild::compose_child(State::Stateful(this.clone()), child)
        }
      }
    }
  }
}

impl<W, D, M> WithChild<[M; 10], Stateful<DynWidget<D>>> for W
where
  W: ComposeChild<Child = Vec<Widget>> + 'static,
  D: IntoIterator + 'static,
  M: ImplMarker + 'static,
  D::Item: IntoWidget<M>,
{
  type Target = Widget;

  #[inline]
  fn with_child(self, child: Stateful<DynWidget<D>>) -> Self::Target {
    let this = Stateful::new(self);
    widget! {
      states { child }
      DynWidget {
        dyns:{
          let widgets = DynRender::spread(&mut child);

          // In this widget, we subscribed the `child` modifies, then spread it.
          // When we spread it, we modifies it, a circular occur. So we forget
          // the modify of take its children to break the circular.
          //
          // In other side, `child` is a stateful dynamic widget and use as
          // child here, and all its content all a black box, so others
          // should not depends on it.
          child.forget_modifies();
          ComposeChild::compose_child(State::Stateful(this.clone()), widgets)
        }
      }
    }
  }
}

impl<W, M, T> WithChild<[M; 11], T> for W
where
  W: WithChild<M, T::Target>,
  T: TemplateBuilder,
{
  type Target = W::Target;

  #[inline]
  fn with_child(self, child: T) -> Self::Target { self.with_child(child.build_tml()) }
}

impl<W, C, M> WithChild<[M; 12], C> for W
where
  W: ComposeChild<Child = State<C>>,
{
  type Target = Widget;

  #[inline]
  fn with_child(self, c: C) -> Self::Target { ComposeChild::compose_child(self.into(), c.into()) }
}

impl<W, C: 'static, M> WithChild<[M; 13], Stateful<DynWidget<C>>> for W
where
  W: ComposeChild<Child = State<C>>,
{
  type Target = Widget;

  #[inline]
  fn with_child(self, c: Stateful<DynWidget<C>>) -> Self::Target {
    ComposeChild::compose_child(self.into(), c.into())
  }
}

pub trait CommonChildConvert<M, T> {
  fn common_convert(self) -> T;
}

impl<W> CommonChildConvert<SelfImpl, W> for W {
  #[inline]
  fn common_convert(self) -> W { self }
}

impl<T, M> CommonChildConvert<&M, Widget> for T
where
  T: IntoWidget<NotSelf<M>>,
{
  fn common_convert(self) -> Widget { self.into_widget() }
}

impl<T, T2, M> CommonChildConvert<&M, Option<T2>> for T
where
  T: CommonChildConvert<M, T2>,
{
  fn common_convert(self) -> Option<T2> { Some(self.common_convert()) }
}

impl<D> CommonChildConvert<SelfImpl, D> for DynWidget<D> {
  #[inline]
  fn common_convert(self) -> D { self.into_inner().common_convert() }
}

impl<W, C, M> CommonChildConvert<[M; 0], WidgetOf<W>> for WidgetPair<W, C>
where
  C: IntoWidget<NotSelf<M>>,
{
  fn common_convert(self) -> WidgetOf<W> {
    let WidgetPair { widget, child } = self;
    WidgetPair { widget, child: child.into_widget() }
  }
}

impl<W, C, M> CommonChildConvert<[M; 1], WidgetOf<W>> for WidgetPair<W, C>
where
  C: IntoWidget<OptionDyn<M>>,
{
  fn common_convert(self) -> WidgetOf<W> {
    let WidgetPair { widget, child } = self;
    WidgetPair { widget, child: child.into_widget() }
  }
}
/// trait mark the expected `ComposeChild::Child` is not `Widget` or
/// `Option<Widget>`, so when it meet a dynamic widget, it need to convert the
/// result to a dynamic widget.
trait NotWidget<M> {}
impl<W: IntoWidget<NotSelf<M>>, M> NotWidget<(M,)> for W {}
impl<W: Template> NotWidget<&W> for W {}
impl<W: NotWidget<M>, M> NotWidget<M> for Option<W> {}
impl<W> NotWidget<()> for Vec<W> {}
impl<W, C> NotWidget<()> for WidgetPair<W, C> {}

// implement `WithChild` for `SingleChild`
impl<W, C> WithChild<&dyn SingleChild, C> for W
where
  W: SingleChild,
{
  type Target = WidgetPair<Self, C>;
  #[inline]
  fn with_child(self, child: C) -> Self::Target { WidgetPair { widget: self, child } }
}

// implement `WithChild` for `MultiChild`
impl<W, C, M> WithChild<Vec<M>, C> for W
where
  W: MultiChild,
  Vec<Widget>: FillMultiChild<M, C>,
{
  type Target = WidgetPair<Self, Vec<Widget>>;
  #[inline]
  fn with_child(self, c: C) -> Self::Target {
    let mut child = vec![];
    child.fill_multi(c);
    WidgetPair { widget: self, child }
  }
}

trait FillMultiChild<M, Item> {
  fn fill_multi(&mut self, item: Item);
}

// multi dynamic widget can directly as child of `MultiChild`
impl<D, M> FillMultiChild<&dyn Iterator<Item = M>, Stateful<DynWidget<D>>> for Vec<Widget>
where
  M: ImplMarker + 'static,
  D: IntoIterator + 'static,
  D::Item: IntoWidget<M>,
{
  fn fill_multi(&mut self, item: Stateful<DynWidget<D>>) {
    self.push(DynRender::new(item).into_widget())
  }
}

impl<M, Item> FillMultiChild<M, Item> for Vec<Widget>
where
  Vec<Widget>: FillVec<M, Item>,
{
  #[inline]
  fn fill_multi(&mut self, item: Item) { self.fill_vec(item) }
}

impl<D: SingleChild> SingleChild for Option<D> {}

// Template use to construct child of a widget.
pub trait Template: Sized {
  type Builder: TemplateBuilder;
  fn builder() -> Self::Builder;
}

pub trait TemplateBuilder: Sized {
  type Target;
  fn build_tml(self) -> Self::Target;
}

pub trait FillTml<M, W> {
  type New;
  fn fill(self, c: W) -> Self::New;
}

impl<W, M1, M2, C, T> FillTml<[(M1, M2); 0], WidgetPair<W, C>> for T
where
  C: IntoWidget<NotSelf<M1>>,
  T: FillTml<M2, WidgetOf<W>>,
{
  type New = T::New;
  #[inline]
  fn fill(self, c: WidgetPair<W, C>) -> Self::New {
    let WidgetPair { widget, child } = c;
    self.fill(WidgetPair { widget, child: child.into_widget() })
  }
}

impl<W, M1, M2, C, T> FillTml<[(M1, M2); 1], WidgetPair<W, C>> for T
where
  C: IntoWidget<OptionDyn<M1>>,
  T: FillTml<M2, WidgetOf<W>>,
{
  type New = T::New;
  #[inline]
  fn fill(self, c: WidgetPair<W, C>) -> Self::New {
    let WidgetPair { widget, child } = c;
    self.fill(WidgetPair { widget, child: child.into_widget() })
  }
}

impl<M, D, T> FillTml<&M, DynWidget<D>> for T
where
  T: FillTml<M, D>,
{
  type New = T::New;
  #[inline]
  fn fill(self, c: DynWidget<D>) -> Self::New { self.fill(c.into_inner()) }
}

impl<C, T> FillTml<State<C>, C> for T
where
  T: FillTml<SelfImpl, State<C>>,
{
  type New = T::New;
  #[inline]
  fn fill(self, c: C) -> Self::New { self.fill(c.into()) }
}

impl<C: 'static, T> FillTml<State<C>, Stateful<DynWidget<C>>> for T
where
  T: FillTml<SelfImpl, State<C>>,
{
  type New = T::New;
  #[inline]
  fn fill(self, c: Stateful<DynWidget<C>>) -> Self::New { self.fill(c.into()) }
}

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
        dyns: (size.area() > 0.).then(|| MockBox { size: Size::zero() }) ,
        MockBox { size: Size::zero() }
      }
    };

    // option with `Widget`
    let _e = widget! {
      states { size: size }
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
      fn compose_child(_: State<Self>, child: Self::Child) -> Widget { child.into_widget() }
    }

    let dyns = Stateful::new(DynWidget { dyns: Some(X) });
    let size = Size::new(100., 200.);

    let w = ComposeChild::compose_child(State::Stateless(dyns), MockBox { size });
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
      type Child = MockBox;
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
      fn compose_child(_: State<Self>, _: Self::Child) -> Widget {
        widget! { MockBox { size: EXPECT_SIZE } }
      }
    }

    expect_layout_result(
      widget! {
        Host { Field("test".into()) }
      },
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

    let _ = Window::default_mock(A.with_child(child), None);
    assert_eq!(*cnt.borrow(), 0);
  }
}
