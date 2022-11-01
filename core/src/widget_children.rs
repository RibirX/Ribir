use crate::prelude::*;
/// Trait to tell Ribir a widget can have one child.
pub trait SingleChild {}

/// Trait to tell Ribir a widget can have multi child.
pub trait MultiChild {}

/// Trait mark widget can have one child and also have compose logic for widget
/// and its child.
pub trait ComposeChild {
  type Child;
  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget
  where
    Self: Sized;
}

pub trait WithChild<M: ?Sized, C> {
  type Target;
  fn with_child(self, child: C) -> Self::Target;
}

pub struct WidgetWithChild<W, C> {
  pub widget: W,
  pub child: C,
}

pub struct ChildVec<W>(Vec<W>);

impl<W> ChildVec<W> {
  #[inline]
  pub fn into_inner(self) -> Vec<W> { self.0 }
}

impl<W> std::ops::Deref for ChildVec<W> {
  type Target = Vec<W>;
  #[inline]
  fn deref(&self) -> &Self::Target { &self.0 }
}

impl<W> std::ops::DerefMut for ChildVec<W> {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl<W> Default for ChildVec<W> {
  #[inline]
  fn default() -> Self { Self(Default::default()) }
}

// implementation of IntoWidget

impl<W> IntoWidget<(&dyn Render, Widget)> for WidgetWithChild<W, Widget>
where
  W: SingleChild + Render + 'static,
{
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    let node = WidgetNode::Render(Box::new(widget));
    let children = ChildVec(vec![child]);
    Widget { node: Some(node), children }
  }
}

impl<W, C> IntoWidget<(&dyn Render, dyn Render)> for WidgetWithChild<W, C>
where
  W: SingleChild + Render + 'static,
  C: Render + 'static,
{
  #[inline]
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    WidgetWithChild { widget, child: child.into_widget() }.into_widget()
  }
}

impl<W, C> IntoWidget<(&dyn Render, dyn Compose)> for WidgetWithChild<W, C>
where
  W: Render + SingleChild + 'static,
  C: Compose + 'static,
{
  #[inline]
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    WidgetWithChild { widget, child: child.into_widget() }.into_widget()
  }
}

impl<W, C, M1: ?Sized, M2: ?Sized> IntoWidget<(&M1, Option<&M2>)> for WidgetWithChild<W, Option<C>>
where
  W: IntoWidget<M1>,
  WidgetWithChild<W, C>: IntoWidget<M2>,
{
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    if let Some(child) = child {
      WidgetWithChild { widget, child }.into_widget()
    } else {
      widget.into_widget()
    }
  }
}

impl<W, E, R, M1: ?Sized, M2: ?Sized> IntoWidget<(&M1, ExprWidget<&M2>)>
  for WidgetWithChild<W, ExprWidget<E>>
where
  WidgetWithChild<W, Widget>: IntoWidget<M1>,
  E: FnMut(&mut BuildCtx) -> R + 'static,
  R: IntoChild<M2, Option<Widget>>,
{
  #[inline]
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    WidgetWithChild { widget, child: child.into_widget() }.into_widget()
  }
}

impl<W1, W2, C, M2: ?Sized> IntoWidget<(&dyn Render, &M2)>
  for WidgetWithChild<W1, WidgetWithChild<W2, C>>
where
  W1: Render + SingleChild + 'static,
  WidgetWithChild<W2, C>: IntoWidget<M2>,
{
  #[inline]
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    WidgetWithChild { widget, child: child.into_widget() }.into_widget()
  }
}

impl<W> IntoWidget<dyn Render> for WidgetWithChild<W, ChildVec<Widget>>
where
  W: MultiChild + Render + 'static,
{
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    let node = WidgetNode::Render(Box::new(widget));
    Widget { node: Some(node), children: child }
  }
}

impl<R, E, C, M1: ?Sized, M2: ?Sized, M3: ?Sized> WithChild<(&M1, &M2, &M3), C> for ExprWidget<E>
where
  E: FnMut(&mut BuildCtx) -> R + 'static,
  R: WithChild<M2, C> + IntoChild<M3, Option<Widget>>,
  C: IntoChild<M1, ChildVec<Widget>>,
{
  type Target = Widget;

  fn with_child(self, child: C) -> Self::Target {
    let mut widget = self.into_widget();
    assert!(widget.children.is_empty());
    widget.children = child.into_child();
    widget
  }
}

pub trait IntoChild<M: ?Sized, C> {
  fn into_child(self) -> C;
}

// `IntoChild` implementations
impl<W: IntoWidget<M>, M: ?Sized> IntoChild<dyn IntoWidget<M>, Widget> for W {
  #[inline]
  fn into_child(self) -> Widget { self.into_widget() }
}

impl<W> IntoChild<W, W> for W {
  #[inline]
  fn into_child(self) -> W { self }
}

impl<T, M: ?Sized, C> IntoChild<dyn IntoChild<M, C>, Option<C>> for T
where
  T: IntoChild<M, C>,
{
  #[inline]
  fn into_child(self) -> Option<C> { Some(self.into_child()) }
}

impl<T, M: ?Sized> IntoChild<Option<&dyn IntoChild<M, Widget>>, Option<Widget>> for Option<T>
where
  T: IntoWidget<M>,
{
  #[inline]
  fn into_child(self) -> Option<Widget> { self.map(IntoWidget::into_widget) }
}

impl<E, R, M: ?Sized> IntoChild<dyn IntoChild<M, Widget>, Widget> for ExprWidget<E>
where
  E: FnMut(&mut BuildCtx) -> R + 'static,
  R: IntoChild<M, ChildVec<Widget>>,
{
  #[inline]
  fn into_child(self) -> Widget { self.inner_into_widget() }
}

impl<W, C, C2, M: ?Sized> IntoChild<&M, WidgetWithChild<W, C2>> for WidgetWithChild<W, C>
where
  C: IntoChild<M, C2>,
{
  #[inline]
  fn into_child(self) -> WidgetWithChild<W, C2> {
    let Self { widget, child } = self;
    WidgetWithChild { widget, child: child.into_child() }
  }
}

macro_rules! tuple_merge_into_vec {
  ($ty: ident, $mark: ident, $($other_ty: ident, $other_mark: ident,)+) => {
    tuple_merge_into_vec!({$ty, $mark, } $($other_ty, $other_mark,)+);
  };
  (
    {$($ty: ident, $mark: ident,)+}
    $next_ty: ident, $next_mark: ident,
    $($other_ty: ident, $other_mark: ident,)*
  ) => {
      tuple_merge_into_vec!({$($ty, $mark,)+});
      tuple_merge_into_vec!(
        {$($ty, $mark,)+ $next_ty, $next_mark, }
        $($other_ty, $other_mark,)*
      );
  };
  ({ $($ty: ident, $mark: ident,)+})  => {
    impl<W, $($ty, $mark: ?Sized),+> IntoChild<&($(&$mark,)+), ChildVec<W>>
      for ($($ty,)+)
    where
      $($ty: FillChildVec<$mark, W>),+
    {
      #[allow(non_snake_case)]
      fn into_child(self) -> ChildVec<W> {
        let ($($ty,)+) = self;
        let mut children = ChildVec(<_>::default());
        $($ty.fill(&mut children);)+
        children
      }
    }
  }
}

tuple_merge_into_vec!(
  T1, M1, T2, M2, T3, M3, T4, M4, T5, M5, T6, M6, T7, M7, T8, M8, T9, M9, T10, M10, T11, M11, T12,
  M12, T13, M13, T14, M14, T15, M15, T16, M16, T17, M17, T18, M18, T19, M19, T20, M20, T21, M21,
  T22, M22, T23, M23, T24, M24, T25, M25, T26, M26, T27, M27, T28, M28, T29, M29, T30, M30, T31,
  M31, T32, M32,
);

macro_rules! tuple_child_into {
  (
    $target: ident, $from: ident, $mark: ident,
    $($other_target: ident, $other_from: ident, $other_mark: ident,)+
  ) => {
    tuple_child_into!({$target, $from, $mark, } $($other_target, $other_from, $other_mark,)+);
  };
  (
    {$($target: ident, $from: ident, $mark: ident,)+}
    $next_target: ident, $next_from: ident, $next_mark: ident,
    $($other_target: ident, $other_from: ident, $other_mark: ident,)*
  ) => {
      tuple_child_into!({ $($target, $from, $mark,)+ });
      tuple_child_into!(
        {$($target, $from, $mark,)+ $next_target, $next_from, $next_mark, }
        $($other_target, $other_from, $other_mark,)*
      );
  };
  ({ $($target: ident, $from: ident, $mark: ident,)+ })  => {

    impl<$($target, $from, $mark: ?Sized),+>
      IntoChild<($(&dyn IntoChild<&$mark, $target>,)+), ($($target,)+)>
      for ($($from,)+)
    where
      $($from: IntoChild<$mark, $target>),+
    {
      #[allow(non_snake_case)]
      fn into_child(self) -> ($($target,)+) {
        let ($($from,)+) = self;
        ($($from.into_child(),)+)
      }
    }
  }
}

tuple_child_into!(
  T1, F1, M1, T2, F2, M2, T3, F3, M3, T4, F4, M4, T5, F5, M5, T6, F6, M6, T7, F7, M7, T8, F8, M8,
  T9, F9, M9, T10, F10, M10, T11, F11, M11, T12, F12, M12, T13, F13, M13, T14, F14, M14, T15, F15,
  M15, T16, F16, M16, T17, F17, M17, T18, F18, M18, T19, F19, M19, T20, F20, M20, T21, F21, M21,
  T22, F22, M22, T23, F23, M23, T24, F24, M24, T25, F25, M25, T26, F26, M26, T27, F27, M27, T28,
  F28, M28, T29, F29, M29, T30, F30, M30, T31, F31, M31, T32, F32, M32,
);

trait FillChildVec<M: ?Sized, C> {
  fn fill(self, vec: &mut ChildVec<C>);
}

impl<M: ?Sized, W, C> FillChildVec<dyn IntoWidget<&M>, C> for W
where
  W: IntoChild<M, C>,
{
  fn fill(self, vec: &mut ChildVec<C>) { vec.0.push(self.into_child()) }
}

impl<M: ?Sized, I, C> FillChildVec<dyn Iterator<Item = &M>, C> for I
where
  I: IntoIterator,
  I::Item: FillChildVec<M, C>,
{
  #[inline]
  fn fill(self, vec: &mut ChildVec<C>) { self.into_iter().for_each(|w| w.fill(vec)); }
}

impl<C, M, T> IntoChild<ChildVec<&dyn IntoChild<M, C>>, ChildVec<C>> for T
where
  M: ?Sized,
  T: FillChildVec<M, C>,
{
  fn into_child(self) -> ChildVec<C> {
    let mut vec = ChildVec::default();
    self.fill(&mut vec);
    vec
  }
}

// implementations of `WithChild`

impl<M: ?Sized, W, C> WithChild<dyn ComposeChild<Child = &M>, C> for W
where
  W: ComposeChild,
  C: IntoChild<M, W::Child>,
{
  type Target = Widget;

  #[inline]
  fn with_child(self, child: C) -> Self::Target {
    ComposeChild::compose_child(StateWidget::Stateless(self), child.into_child())
  }
}

impl<W, C> WithChild<dyn SingleChild, C> for W
where
  W: SingleChild,
{
  type Target = WidgetWithChild<Self, C>;
  #[inline]
  fn with_child(self, child: C) -> Self::Target { WidgetWithChild { widget: self, child } }
}

impl<W, C, M: ?Sized> WithChild<(&dyn MultiChild, &M), C> for W
where
  W: MultiChild,
  C: IntoChild<M, ChildVec<Widget>>,
{
  type Target = WidgetWithChild<Self, ChildVec<Widget>>;
  #[inline]
  fn with_child(self, child: C) -> Self::Target {
    WidgetWithChild {
      widget: self,
      child: child.into_child(),
    }
  }
}

impl<F, R, C, M: ?Sized> WithChild<dyn Fn(&M) -> R, C> for F
where
  F: Fn(Widget) -> R,
  C: IntoWidget<M>,
{
  type Target = R;

  #[inline]
  fn with_child(self, child: C) -> Self::Target { self(child.into_widget()) }
}

impl<F, R, C, M: ?Sized> WithChild<dyn Fn(&M) -> R, C> for std::rc::Rc<F>
where
  F: Fn(Widget) -> R,
  C: IntoWidget<M>,
{
  type Target = R;

  #[inline]
  fn with_child(self, child: C) -> Self::Target { self(child.into_widget()) }
}

impl<T, C, M1: ?Sized, M2: ?Sized, M3: ?Sized> WithChild<(&M1, &M2, &M3), C> for Option<T>
where
  T: WithChild<M1, C>,
  T::Target: IntoChild<M2, Widget>,
  C: IntoChild<M3, Widget>,
{
  type Target = Widget;

  #[inline]
  fn with_child(self, child: C) -> Self::Target {
    if let Some(widget) = self {
      widget.with_child(child).into_child()
    } else {
      child.into_child()
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::test::{MockBox, MockMulti};

  use super::*;

  #[test]
  fn compose_tuple_child() {
    #[derive(Declare)]
    struct Page;
    #[derive(Declare, SingleChild)]
    struct Header;
    #[derive(Declare, SingleChild)]
    struct Content;
    #[derive(Declare, SingleChild)]
    struct Footer;

    impl ComposeChild for Page {
      type Child = (
        WidgetWithChild<Header, Widget>,
        WidgetWithChild<Content, Widget>,
        WidgetWithChild<Footer, Widget>,
      );

      fn compose_child(_: StateWidget<Self>, child: Self::Child) -> Widget {
        let (_, _, _) = child;
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
      type Child = Option<WidgetWithChild<Child, Widget>>;

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
      type Child = ChildVec<B>;

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
      ExprWidget {
        expr: if size.area() > 0. {
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
      ExprWidget {
        expr: if size.area() > 0. { MockMulti {} } else { MockMulti {} },
        MockBox { size: Size::zero() }
        MockBox { size: Size::zero() }
        MockBox { size: Size::zero() }
      }
    };

    // option with single child
    let _e = widget! {
      track { size: size.clone() }
      ExprWidget {
        expr: (size.area() > 0.).then(|| MockBox { size: Size::zero() }) ,
        MockBox { size: Size::zero() }
      }
    };

    // option with `Widget`
    let _e = widget! {
      track { size: size.clone() }
      ExprWidget {
        expr: (size.area() > 0.).then(|| MockBox { size: Size::zero() }) ,
        ExprWidget { expr: Void.into_widget() }
      }
    };
  }
}
