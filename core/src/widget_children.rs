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

/// A node of widget with not compose its child.
pub struct WidgetPair<W, C> {
  pub widget: W,
  pub child: C,
}

/// A alias of `WidgetPair<W, Widget>`, means `Widget` is the child of the
/// generic type.
pub type WidgetOf<W> = WidgetPair<W, Widget>;

// implementation of IntoWidget
impl<W, C, M> IntoWidget<FromOther<M>> for WidgetPair<W, C>
where
  M: ChildMarker,
  W: SingleChild + Render + 'static,
  C: IntoChild<M, Option<Widget>>,
{
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    Widget {
      node: Some(WidgetNode::Render(Box::new(widget))),
      children: child.into_child().map_or_else(Vec::default, |c| vec![c]),
    }
  }
}

impl<W, C, M> IntoWidget<FromOther<Vec<M>>> for WidgetPair<W, C>
where
  M: ChildMarker,
  W: MultiChild + Render + 'static,
  C: IntoChild<M, Vec<Widget>>,
{
  #[inline]
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    Widget {
      node: Some(WidgetNode::Render(Box::new(widget))),
      children: child.into_child(),
    }
  }
}

pub trait IntoChild<M: ChildMarker, C> {
  fn into_child(self) -> C;
}

/// A middle type use to help to convert children to tuple.
pub struct TupleChild<W>(pub W);
pub trait ChildMarker {}
impl ChildMarker for FromSelf {}
impl<M> ChildMarker for FromOther<M> {}

// `IntoChild` implementations
// W -> W
impl<W> IntoChild<FromSelf, W> for W {
  #[inline]
  fn into_child(self) -> W { self }
}

// W -> Widget  W != Widget
impl<W: IntoWidget<FromOther<M>>, M> IntoChild<FromOther<M>, Widget> for W {
  #[inline]
  fn into_child(self) -> Widget { self.into_widget() }
}

// W -> Option<C>
impl<W, M: ChildMarker, C> IntoChild<FromOther<M>, Option<C>> for W
where
  W: IntoChild<M, C>,
{
  #[inline]
  fn into_child(self) -> Option<C> { Some(self.into_child()) }
}

// Option<W> -> Option<C>  W != C
impl<W, M, C> IntoChild<FromOther<Option<M>>, Option<C>> for Option<W>
where
  M: ChildMarker,
  W: IntoChild<FromOther<M>, C>,
{
  #[inline]
  fn into_child(self) -> Option<C> { self.map(IntoChild::into_child) }
}

// WidgetWithChild<W, C> -> WidgetWithChild<W, C2>
impl<W, C, C2, M: ChildMarker> IntoChild<FromOther<M>, WidgetPair<W, C2>> for WidgetPair<W, C>
where
  C: IntoChild<M, C2>,
{
  #[inline]
  fn into_child(self) -> WidgetPair<W, C2> {
    let Self { widget, child } = self;
    WidgetPair { widget, child: child.into_child() }
  }
}

// impl IntoChild<Vec<_>>

// trait help to implement Vec<Child>.

pub trait FillChildVec<M: ?Sized, C> {
  fn fill(self, vec: &mut Vec<C>);
}

impl<W, M, C> IntoChild<FromOther<M>, Vec<C>> for W
where
  W: FillChildVec<M, C>,
{
  fn into_child(self) -> Vec<C> {
    let mut vec = vec![];
    self.fill(&mut vec);
    vec
  }
}

// W -> Vec<C>
impl<M, W, C> FillChildVec<M, C> for W
where
  M: ChildMarker,
  W: IntoChild<M, C>,
{
  #[inline]
  fn fill(self, vec: &mut Vec<C>) { vec.push(self.into_child()) }
}

macro_rules! tuple_into_child {
  (
    $target: ident, $from: ident, $mark: ident,
    $($other_target: ident, $other_from: ident, $other_mark: ident,)+
  ) => {
    tuple_into_child!(
      {$target, $from, $mark, }
      $($other_target, $other_from, $other_mark,)+);
  };
  (
    {$($target: ident, $from: ident, $mark: ident,)+}
    $next_target: ident, $next_from: ident, $next_mark: ident,
    $($other_target: ident, $other_from: ident, $other_mark: ident,)*
  ) => {
      tuple_into_child!({ $($target, $from, $mark,)+ });
      tuple_into_child!(
        {$($target, $from, $mark,)+ $next_target, $next_from, $next_mark, }
        $($other_target, $other_from, $other_mark,)*
      );
  };
  ({ $($target: ident $comma:tt $from: ident, $mark: ident,)+ })  => {

    // tuple convert.
    // impl (W, W, ..) -> (W1, W2, ..)
    impl<$($target, $from, $mark),+>
      IntoChild<FromOther<($(&dyn IntoChild<&$mark, $target>,)+)>, ($($target,)+)>
      for TupleChild<($($from,)+)>
    where
      $(
        $mark: ChildMarker,
        $from: IntoChild<$mark, $target>
      ),+
    {
      #[allow(non_snake_case)]
      fn into_child(self) -> ($($target,)+) {
        let TupleChild(($($from,)+)) = self;
        ($($from.into_child(),)+)
      }
    }

    // impl (W, w, ..) -> Vec<C>
    impl<W, $($from, $mark),+> FillChildVec<($(&$mark,)+), W> for TupleChild<($($from,)+)>
    where
      $(
        $from: FillChildVec<$mark, W>
      ),+
    {
      #[allow(non_snake_case)]
      fn fill(self, vec: &mut Vec<W>) {
        let TupleChild(($($from,)+)) = self;
        $($from.fill(vec);)+
      }
    }
  }
}

tuple_into_child!(
  T1, F1, M1, T2, F2, M2, T3, F3, M3, T4, F4, M4, T5, F5, M5, T6, F6, M6, T7, F7, M7, T8, F8, M8,
  T9, F9, M9, T10, F10, M10, T11, F11, M11, T12, F12, M12, T13, F13, M13, T14, F14, M14, T15, F15,
  M15, T16, F16, M16, T17, F17, M17, T18, F18, M18, T19, F19, M19, T20, F20, M20, T21, F21, M21,
  T22, F22, M22, T23, F23, M23, T24, F24, M24, T25, F25, M25, T26, F26, M26, T27, F27, M27, T28,
  F28, M28, T29, F29, M29, T30, F30, M30, T31, F31, M31, T32, F32, M32,
);

// implementations of `WithChild`

impl<M, W, C> WithChild<dyn ComposeChild<Child = &M>, C> for W
where
  M: ChildMarker,
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
  type Target = WidgetPair<Self, C>;
  #[inline]
  fn with_child(self, child: C) -> Self::Target { WidgetPair { widget: self, child } }
}

impl<W, C, M> WithChild<(&dyn MultiChild, &M), C> for W
where
  M: ChildMarker,
  W: MultiChild,
  C: IntoChild<M, Vec<Widget>>,
{
  type Target = WidgetPair<Self, Vec<Widget>>;
  #[inline]
  fn with_child(self, child: C) -> Self::Target {
    WidgetPair {
      widget: self,
      child: child.into_child(),
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
        WidgetPair<Header, Widget>,
        WidgetPair<Content, Widget>,
        WidgetPair<Footer, Widget>,
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
    MockBox { size: Size::zero() }.with_child(DynWidget {
      dyns: Some(MockBox { size: Size::zero() }),
    });
  }

  #[test]
  fn tuple_into_child_self_hint() {
    let x: (i32, i32) = (0, 0);
    let _: (i32, i32) = x.into_child();
  }
}
