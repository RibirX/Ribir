//! Implements a dozen of enums to store different widget, and implement the
//! common trait if all enum variable implement it.

use crate::prelude::*;

macro_rules! impl_enum_widget {
  ($name: ident, $($var_ty: ident, $mark_ty: ident) ,+ ) => {
    pub enum $name<$($var_ty),+> {
      $($var_ty($var_ty)),+
    }
    impl<$($mark_ty: ImplMarker, $var_ty: IntoWidget<$mark_ty>),+>
      IntoWidget<NotSelf<($($mark_ty,)*)>> for $name <$($var_ty),+> {
      fn into_widget(self) -> Widget {
        match self {
          $($name::$var_ty(w) => w.into_widget()),+
        }
      }
    }

    impl<Child, $($mark_ty,$var_ty: SingleWithChild<$mark_ty, Child>),+>
      SingleWithChild<($($mark_ty,)*), Child> for $name <$($var_ty),+> {
        type Target = $name<$($var_ty::Target),+ >;

      #[inline]
      fn with_child(self, child: Child) -> Self::Target {
        match self {
          $($name::$var_ty(w) => $name::$var_ty(w.with_child(child))),+
        }
      }
    }

    impl<Child, $($mark_ty,$var_ty: MultiWithChild<$mark_ty, Child>),+>
      MultiWithChild<($($mark_ty,)*), Child> for $name <$($var_ty),+> {
        type Target = $name<$($var_ty::Target),+ >;

      #[inline]
      fn with_child(self, child: Child) -> Self::Target {
        match self {
          $($name::$var_ty(w) => $name::$var_ty(w.with_child(child))),+
        }
      }
    }

    impl<Child, $($mark_ty,$var_ty: ComposeWithChild<$mark_ty, Child>),+>
      ComposeWithChild<($($mark_ty,)*), Child> for $name <$($var_ty),+> {
        type Target = $name<$($var_ty::Target),+ >;

      #[inline]
      fn with_child(self, child: Child) -> Self::Target {
        match self {
          $($name::$var_ty(w) => $name::$var_ty(w.with_child(child))),+
        }
      }
    }
  };
}

impl_enum_widget!(WidgetE2, A, M1, B, M2);
impl_enum_widget!(WidgetE3, A, M1, B, M2, C, M3);
impl_enum_widget!(WidgetE4, A, M1, B, M2, C, M3, D, M4);
impl_enum_widget!(WidgetE5, A, M1, B, M2, C, M3, D, M4, E, M5);
impl_enum_widget!(WidgetE6, A, M1, B, M2, C, M3, D, M4, E, M5, F, M6);
