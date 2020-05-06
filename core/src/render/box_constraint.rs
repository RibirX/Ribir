use crate::prelude::*;
use crate::util::proxy_macro::*;

#[derive(Debug, Clone, Copy)]
pub struct BoxBound {
  min_height: f64,
  max_height: f64,
  min_width: f64,
  max_width: f64,
}

/// render object's layout box, the information about layout, including box size, layout constraints and box_bound.application
#[derive(Debug)]
pub struct BoxLayout {
  constraints: LayoutConstraints,
  size: Option<Size>,

  /// box bound is the bound of the layout can be place. it should be set before render object's process of layout.
  /// when the object it is in the layout such as row, flex ... it's size is decided by his parent.
  box_bound: Option<BoxBound>,
}

impl BoxLayout {
  pub fn new(lc: LayoutConstraints) -> BoxLayout{
      return BoxLayout{
          constraints: lc,
          size: None,
          box_bound: None,
      }
  }
  pub fn get_size(&self) -> Option<Size> { self.size.clone() }
  pub fn set_size(&mut self, size: Option<Size>) { self.size = size; }

  pub fn get_box_bound(&self) -> BoxBound {
    if self.box_bound.is_some() {
      return self.box_bound.unwrap().clone();
    } else {
      return BoxBound {
        min_height: 0.0,
        max_height: f64::INFINITY,
        min_width: 0.0,
        max_width: f64::INFINITY,
      };
    }
  }

  pub fn set_box_bound(&mut self, bound: Option<BoxBound>) {
    self.box_bound = bound;
  }

  pub fn get_constraints(&self) -> LayoutConstraints { self.constraints }
}


pub macro default_box_impl( {$($path:ident).*}) {
    #[inline]
    proxy_immut_impl!{get_size,  {$($path).*},  Option<Size>, get_size, ()}

    #[inline]
    proxy_immut_impl!{get_constraints,  {$($path).* }, LayoutConstraints, get_constraints, ()}

    #[inline]
    proxy_mut_impl!{set_box_bound,  {$($path).*}, (), set_box_bound, (bound: Option<BoxBound>)}
}
