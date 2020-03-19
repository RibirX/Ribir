//! widget is a cheap config to detect how user interface should be display
use crate::render_object::RenderObject;
use ::herald::prelude::*;
use subject::LocalSubject;

/// detect if two trait widget objects are same type.
// pub fn same_type(a: &dyn Widget, b: &dyn Widget) -> bool {
//   let raw_a: TraitObject = unsafe { std::mem::transmute(a) };
//   let raw_b: TraitObject = unsafe { std::mem::transmute(b) };
//   raw_a.vtable == raw_b.vtable
// }

// fn equal(&self, other: &dyn Widget) -> bool
// where
//   Self: PartialEq<Self> + Sized,
// {
//   if same_type(self, other) {
//     let other = unsafe {
//       let raw_obj: TraitObject = std::mem::transmute(other);
//       &*(raw_obj.data as *const Self)
//     };
//     self.equal(other)
//   } else {
//     false
//   }
// }
pub trait RebuildEmitter<'a> {
  fn emitter(
    &mut self,
    notifier: LocalSubject<'a, (), ()>,
  ) -> Option<LocalCloneBoxOp<'a, (), ()>>;
}

/// A widget represented by other widget compose.
pub trait CombinationWidget<'a>: RebuildEmitter<'a> {
  #[cfg(debug_assertions)]
  fn to_str(&self) -> String;
  /// Describes the part of the user interface represented by this widget.
  fn build(&self) -> Widget;
}

/// RenderWidget is a widget has its render object to display self.
pub trait RenderWidget<'a>: RebuildEmitter<'a> {
  #[cfg(debug_assertions)]
  fn to_str(&self) -> String;

  fn create_render_object(&self) -> Box<dyn RenderObject>;
}

/// a widget has a child.
pub trait SingleChildWidget<'a>: RebuildEmitter<'a> {
  fn split(self: Box<Self>) -> (Box<dyn for<'r> RenderWidget<'r>>, Widget);
}

/// a widget has multi child
pub trait MultiChildWidget<'a>: RebuildEmitter<'a> {
  fn split(self: Box<Self>)
  -> (Box<dyn for<'r> RenderWidget<'r>>, Vec<Widget>);
}

pub enum Widget {
  Combination(Box<dyn for<'a> CombinationWidget<'a>>),
  Render(Box<dyn for<'a> RenderWidget<'a>>),
  SingleChild(Box<dyn for<'a> SingleChildWidget<'a>>),
  MultiChild(Box<dyn for<'a> MultiChildWidget<'a>>),
}

impl<'a> RebuildEmitter<'a> for Widget {
  fn emitter(
    &mut self,
    notifier: LocalSubject<'a, (), ()>,
  ) -> Option<LocalCloneBoxOp<'a, (), ()>> {
    match self {
      Widget::Combination(w) => w.emitter(notifier),
      Widget::Render(w) => w.emitter(notifier),
      Widget::SingleChild(w) => w.emitter(notifier),
      Widget::MultiChild(w) => w.emitter(notifier),
    }
  }
}

impl<'a, T> RebuildEmitter<'a> for T {
  #[inline]
  default fn emitter(
    &mut self,
    _notifier: LocalSubject<'a, (), ()>,
  ) -> Option<LocalCloneBoxOp<'a, (), ()>> {
    None
  }
}

impl<'a, T: Herald<'a> + 'a> RebuildEmitter<'a> for T {
  #[inline]
  default fn emitter(
    &mut self,
    notifier: LocalSubject<'a, (), ()>,
  ) -> Option<LocalCloneBoxOp<'a, (), ()>> {
    Some(self.batched_change_stream(notifier).map(|_v| ()).box_it())
  }
}
