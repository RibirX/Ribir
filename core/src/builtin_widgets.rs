//! Built-in widgets is a set of minimal widgets that describes the most common
//! UI elements. The most of them can be used to extend other object in the
//! declare syntax, so other objects can use the builtin fields and methods like
//! self fields and methods.

pub mod key;
use std::cell::Cell;

pub use key::{Key, KeyWidget};
pub mod image_widget;
pub use image_widget::*;
pub mod delay_drop;
pub use delay_drop::DelayDrop;
mod theme;
use ribir_algo::Sc;
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
pub mod global_anchor;
pub use global_anchor::*;
mod mix_builtin;
pub use mix_builtin::*;
pub mod container;
pub use container::*;

use crate::{
  prelude::*,
  widget::{Widget, WidgetBuilder},
};

macro_rules! impl_builtin_obj {
  ($($builtin_ty: ty),*) => {
    paste::paste! {
      #[doc="A builtin object contains all builtin widgets, and can be used to \
      extend other object in the declare syntax, so other objects can use the \
      builtin fields and methods like self fields and methods."]
      #[derive(Default)]
      pub struct BuiltinObj {
        host_id: LazyWidgetId,
        id: LazyWidgetId,
        $([< $builtin_ty: snake:lower >]: Option<State<$builtin_ty>>),*
      }

      impl BuiltinObj {
        pub fn is_empty(&self) -> bool {
          self.host_id.ref_count() == 1
          && self.id.ref_count() == 1
          && $(self.[< $builtin_ty: snake:lower >].is_none())&& *
        }

        $(
          pub fn [< $builtin_ty: snake:lower >](&mut self, ctx: &BuildCtx)
            -> &mut State<$builtin_ty>
          {
            self
              .[< $builtin_ty: snake:lower >]
              .get_or_insert_with(|| $builtin_ty::declare_builder().build_declare(ctx))
          }
        )*

        $(
          pub fn [< set_builtin_ $builtin_ty: snake:lower >](
            mut self, builtin: State<$builtin_ty>
          ) -> Self {
            self.[< $builtin_ty: snake:lower >] = Some(builtin);
            self
          }
        )*

        $(
          pub fn [< get_builtin_ $builtin_ty: snake:lower >](&mut self, ctx: &BuildCtx)
            -> &mut State<$builtin_ty>
          {
            self
              .[< $builtin_ty: snake:lower >]
              .get_or_insert_with(|| $builtin_ty::declare_builder().build_declare(ctx))
          }
        )*

        pub fn compose_with_host(self, mut host: Widget, ctx: &BuildCtx) -> Widget {
          self.host_id.set(host.id());
          $(
            if let Some(builtin) = self.[< $builtin_ty: snake:lower >] {
              host = builtin.with_child(host, ctx).widget_build(ctx);
            }
          )*
          self.id.set(host.id());
          host
        }

        pub fn lazy_host_id(&self) -> LazyWidgetId { self.host_id.clone() }

        pub fn lazy_id(&self) -> LazyWidgetId { self.id.clone() }
      }

      impl<T> FatObj<T> {
        $(
          pub fn [< get_builtin_ $builtin_ty: snake:lower >](&mut self, ctx: &BuildCtx)
            -> &mut State<$builtin_ty>
          {
            self.builtin.[<get_builtin_ $builtin_ty: snake:lower >](ctx)
          }
        )*
      }
    }
  };
}

impl_builtin_obj!(
  MixBuiltin,
  RequestFocus,
  HasFocus,
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
  RelativeAnchor,
  GlobalAnchor,
  Visibility,
  Opacity,
  DelayDrop
);

#[derive(Clone)]
/// LazyWidgetId is a widget id that will be valid after widget build.
pub struct LazyWidgetId(Sc<Cell<Option<WidgetId>>>);

/// A fat object that extend the `T` object with all builtin widgets ability. A
/// `FatObj` will create during the compose phase, and compose with the builtin
/// widgets it actually use, and drop after composed.
pub struct FatObj<T> {
  host: T,
  builtin: BuiltinObj,
}

impl LazyWidgetId {
  pub fn id(&self) -> Option<WidgetId> { self.0.get() }

  pub fn assert_id(&self) -> WidgetId { self.0.get().unwrap() }

  fn set(&self, wid: WidgetId) { self.0.set(Some(wid)); }

  fn ref_count(&self) -> usize { self.0.ref_count() }
}

impl Default for LazyWidgetId {
  fn default() -> Self { Self(Sc::new(Cell::new(None))) }
}

impl<T> FatObj<T> {
  pub fn from_host(host: T) -> Self { Self { host, builtin: BuiltinObj::default() } }

  pub fn new(host: T, builtin: BuiltinObj) -> Self { Self { host, builtin } }

  #[inline]
  pub fn map<V>(self, f: impl FnOnce(T) -> V) -> FatObj<V> {
    let Self { host, builtin } = self;
    FatObj { host: f(host), builtin }
  }

  pub fn into_inner(self) -> T {
    assert!(
      self.builtin.is_empty(),
      "Unwrap a FatObj with contains builtin widgets is not allowed."
    );
    self.host
  }

  /// Return the LazyWidgetId of the host widget, through which you can access
  /// the WidgetId after building.
  pub fn lazy_host_id(&self) -> LazyWidgetId { self.builtin.lazy_host_id() }

  /// Return the LazyWidgetId point to WidgetId of the root of the sub widget
  /// tree after the FatObj has built.
  pub fn lazy_id(&self) -> LazyWidgetId { self.builtin.lazy_id() }
}

impl<T: SingleChild> SingleChild for FatObj<T> {}
impl<T: MultiChild> MultiChild for FatObj<T> {}

crate::widget::multi_build_replace_impl! {
  impl<T: {#} > {#} for FatObj<T> {
    fn widget_build(self, ctx: &BuildCtx) -> Widget {
      self.map(|host| host.widget_build(ctx)).widget_build(ctx)
    }
  }
}

impl WidgetBuilder for FatObj<Widget> {
  #[inline]
  fn widget_build(self, ctx: &BuildCtx) -> Widget {
    let Self { host, builtin } = self;
    builtin.compose_with_host(host, ctx)
  }
}

impl<T: ComposeWithChild<C, M>, C, M> ComposeWithChild<C, M> for FatObj<T> {
  type Target = FatObj<T::Target>;

  #[inline]
  fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target {
    self.map(|host| host.with_child(child, ctx))
  }
}

impl<T: PairWithChild<C>, C> PairWithChild<C> for FatObj<T> {
  type Target = Pair<FatObj<T>, C>;

  #[inline]
  fn with_child(self, child: C, _: &BuildCtx) -> Self::Target { Pair::new(self, child) }
}

impl<T: SingleParent + 'static> SingleParent for FatObj<T> {
  fn compose_child(self, child: Widget, ctx: &BuildCtx) -> Widget {
    self
      .map(|host| host.compose_child(child, ctx))
      .widget_build(ctx)
  }
}

impl<T: MultiParent + 'static> MultiParent for FatObj<T> {
  fn compose_children(self, children: impl Iterator<Item = Widget>, ctx: &BuildCtx) -> Widget {
    self
      .map(|host| host.compose_children(children, ctx))
      .widget_build(ctx)
  }
}

impl ComposeChild for BuiltinObj {
  type Child = Widget;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> impl WidgetBuilder {
    let Ok(this) = this.try_into_value() else {
      unreachable!("BuiltinObj should never be a state.")
    };
    fn_widget! { this.compose_with_host(child, ctx!()) }
  }
}

impl<T> std::ops::Deref for FatObj<T> {
  type Target = T;
  #[inline]
  fn deref(&self) -> &Self::Target { &self.host }
}

impl<T> std::ops::DerefMut for FatObj<T> {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.host }
}
