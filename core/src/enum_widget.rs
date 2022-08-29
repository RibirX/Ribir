//! Implements a dozen of enums to store different widget, and implement the
//! common trait if all enum variable implement it.

use crate::prelude::*;

macro_rules! impl_enum_widget {
  ($name: ident, $($var_ty: ident) ,+ ) => {
    pub enum $name<$($var_ty),+> {
      $($var_ty($var_ty)),+
    }

    impl< $($var_ty: Query),+> Query for $name <$($var_ty),+> {
      fn query_all(
        &self,
        type_id: TypeId,
        callback: &mut dyn FnMut(&dyn Any) -> bool,
        order: QueryOrder,
      ) {
        match self {
          $($name::$var_ty(w) => w.query_all(type_id, callback, order)),+
        }
      }

      fn query_all_mut(
        &mut self,
        type_id: TypeId,
        callback: &mut dyn FnMut(&mut dyn Any) -> bool,
        order: QueryOrder,
      ) {
        match self {
          $($name::$var_ty(w) => w.query_all_mut(type_id, callback, order)),+
        }
      }
    }

    impl< $($var_ty: Render),+> Render for $name <$($var_ty),+> {
      fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
        match self {
          $($name::$var_ty(w) => w.perform_layout(clamp, ctx)),+
        }
      }

      fn paint(&self, ctx: &mut PaintingCtx) {
        match self {
          $($name::$var_ty(w) => w.paint(ctx)),+
        }
      }

      fn only_sized_by_parent(&self) -> bool {
        match self {
          $($name::$var_ty(w) => w.only_sized_by_parent()),+
        }
       }
    }

    impl<$($var_ty: SingleChild),+> SingleChild for $name <$($var_ty),+> {}
    impl<$($var_ty: MultiChild),+> MultiChild for $name <$($var_ty),+> {}
    impl<$($var_ty: ComposeSingleChild),+> ComposeSingleChild for $name <$($var_ty),+> {
      fn compose_single_child(this: StateWidget<Self>, child: Widget, ctx: &mut BuildCtx)
         -> Widget {
        let w = match this {
         StateWidget::  Stateless(w) => w,
         StateWidget:: Stateful(_) =>  {
          unreachable!("Enum widgets only use to store widget, should never convert to stateful.");
         }
        };
        match w {
          $($name::$var_ty(w) => $var_ty::compose_single_child(w.into(), child, ctx)),+
        }
      }
    }
    
    impl<$($var_ty: ComposeMultiChild),+> ComposeMultiChild for $name <$($var_ty),+> {
      fn compose_multi_child(
        this: StateWidget<Self>,
        children: Vec<Widget>,
        ctx: &mut BuildCtx,
      ) -> Widget {
        let w = match this {
         StateWidget::  Stateless(w) => w,
         StateWidget:: Stateful(_) =>  {
          unreachable!("Enum widgets only use to store widget, should never convert to stateful.");
         }
        };

        match w {
          $($name::$var_ty(w) => $var_ty::compose_multi_child(w.into(), children, ctx)),+
        }
      }
    }
  };
}

impl_enum_widget!(WidgetE2, A, B);
impl_enum_widget!(WidgetE3, A, B, C);
impl_enum_widget!(WidgetE4, A, B, C, D);
impl_enum_widget!(WidgetE5, A, B, C, D, E);
impl_enum_widget!(WidgetE6, A, B, C, D, E, F);
