//! Built-in widgets is a set of minimal widgets that describes the most common
//! UI elements. The most of them can be used to extend other object in the
//! declare syntax, so other objects can use the builtin fields and methods like
//! self fields and methods.

pub mod key;
pub use key::{Key, KeyWidget};
pub mod image_widget;
pub use image_widget::*;
pub mod delay_drop_widget;
pub use delay_drop_widget::DelayDropWidget;
mod theme;
pub use theme::*;
mod cursor;
pub use cursor::Cursor;
pub use winit::window::CursorIcon;
mod margin;
pub use margin::*;
mod padding;
pub use padding::*;
mod box_decoration;
pub use box_decoration::*;
mod scrollable;
pub use scrollable::*;
mod transform_widget;
pub use transform_widget::*;
mod visibility;
pub use visibility::*;
mod ignore_pointer;
pub use ignore_pointer::*;
mod void;
pub use void::Void;
mod unconstrained_box;
pub use unconstrained_box::*;
mod lifecycle;
pub use lifecycle::*;
mod opacity;
pub use opacity::*;
mod anchor;
pub use anchor::*;
mod layout_box;
pub use layout_box::*;
pub mod align;
pub use align::*;
pub mod fitted_box;
pub use fitted_box::*;
pub mod svg;
pub use svg::*;
pub mod has_focus;
pub use has_focus::*;
pub mod mouse_hover;
pub use mouse_hover::*;
pub mod clip;
pub use clip::*;
pub mod pointer_pressed;
pub use pointer_pressed::*;
pub mod focus_node;
pub use focus_node::*;
pub mod focus_scope;
pub use focus_scope::*;

use crate::{prelude::*, widget::WidgetBuilder};

macro_rules! impl_fat_obj {
  ($($builtin_ty: ty),*) => {
    paste::paste! {
      #[doc="A fat object that extend the `T` object with all builtin widgets \
       ability.\
       During the compose phase, the `FatObj` will be created when a object \
       use the builtin fields and methods. And they are only a help obj to \
       build the finally widget, so they will be dropped after composed."
      ]
      pub struct FatObj<T> {
        pub host: T,
        $([< $builtin_ty: snake:lower >]: Option<State<$builtin_ty>>),*
      }

      impl<T> FatObj<T> {
        pub fn new(host: T) -> Self {
          Self {
            host,
            $([< $builtin_ty: snake:lower >]: None),*
          }
        }

        $(
          pub fn [< with_ $builtin_ty: snake:lower >](
            mut self, builtin: State<$builtin_ty>
          ) -> Self {
            self.[< $builtin_ty: snake:lower >] = Some(builtin);
            self
          }
        )*

        $(
          pub fn [< $builtin_ty: snake:lower >](&mut self, ctx: &BuildCtx)
            -> &mut State<$builtin_ty>
          {
            self
              .[< $builtin_ty: snake:lower >]
              .get_or_insert_with(|| $builtin_ty::declare2_builder().build(ctx))
          }
        )*
      }

      impl<T: 'static> WidgetBuilder for FatObj<T>
      where
        T: Into<Widget>
      {
        fn build(self, ctx: &BuildCtx) -> WidgetId {
          let mut host: Widget = self.host.into();
          $(
            if let Some(builtin) = self.[< $builtin_ty: snake:lower >] {
              host = builtin.with_child(host, ctx).into();
            }
          )*
          host.build(ctx)
        }
      }

      impl<T: SingleWithChild<C>, C> SingleWithChild<C> for FatObj<T>{
        type Target = FatObj<T::Target>;
        fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target {
          let Self { host, $([< $builtin_ty: snake:lower >]),* } = self;
          let host = host.with_child(child, ctx);
          FatObj {
            host,
            $([< $builtin_ty: snake:lower >]),*
          }
        }
      }

      impl<T: MultiWithChild<C>, C> MultiWithChild<C> for FatObj<T>{
        type Target = FatObj<T::Target>;
        fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target {
          let Self { host, $([< $builtin_ty: snake:lower >]),* } = self;
          let host = host.with_child(child, ctx);
          FatObj {
            host,
            $([< $builtin_ty: snake:lower >]),*
          }
        }
      }
      impl<T, M, C> ComposeWithChild<C, M> for FatObj<T>
      where
        T: ComposeWithChild<C, M>
      {
        type Target = FatObj<T::Target>;
        fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target {
          let Self { host, $([< $builtin_ty: snake:lower >]),* } = self;
          let host = host.with_child(child, ctx);
          FatObj {
            host,
            $([< $builtin_ty: snake:lower >]),*
          }
        }
      }
    }
  };
}

impl_fat_obj!(
  PointerListener,
  FocusNode,
  RequestFocus,
  FocusListener,
  FocusBubbleListener,
  HasFocus,
  KeyboardListener,
  CharsListener,
  WheelListener,
  MouseHover,
  PointerPressed,
  FittedBox,
  BoxDecoration,
  Padding,
  LayoutBox,
  Cursor,
  Margin,
  ScrollableWidget,
  TransformWidget,
  HAlignWidget,
  VAlignWidget,
  LeftAnchor,
  RightAnchor,
  TopAnchor,
  BottomAnchor,
  Visibility,
  Opacity,
  LifecycleListener,
  DelayDropWidget
);

impl<T> std::ops::Deref for FatObj<T> {
  type Target = T;
  #[inline]
  fn deref(&self) -> &Self::Target { &self.host }
}

impl<T> std::ops::DerefMut for FatObj<T> {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.host }
}
