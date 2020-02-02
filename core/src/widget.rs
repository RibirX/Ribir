use crate::render_object::RenderObject;
use std::raw::TraitObject;

pub trait Widget {
  /// Describes the part of the user interface represented by this widget.
  fn build(&self) -> Option<Box<dyn Widget>>;
  fn render(&self) -> Box<dyn RenderObject>;
  fn equal(&self, other: &dyn Widget) -> bool
  where
    Self: PartialEq<Self> + Sized,
  {
    if same_type(self, other) {
      let other = unsafe {
        let raw_obj: TraitObject = std::mem::transmute(other);
        &*(raw_obj.data as *const Self)
      };
      self.equal(other)
    } else {
      false
    }
  }
}

/// detect if two trait widget objects are same type.
pub fn same_type(a: &dyn Widget, b: &dyn Widget) -> bool {
  let raw_a: TraitObject = unsafe { std::mem::transmute(a) };
  let raw_b: TraitObject = unsafe { std::mem::transmute(b) };
  raw_a.vtable == raw_b.vtable
}
