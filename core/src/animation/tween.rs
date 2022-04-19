use painter::{Brush, Radius};

use crate::prelude::{Color, EdgeInsets, Point, Size};
macro_rules! tween_check {
  ($begin: ident, $end: ident, $p: ident) => {
    if $p <= 0. {
      return $begin.clone();
    } else if $p >= 1. {
      return $end.clone();
    }
  };
}
pub trait Tween {
  fn tween(begin: &Self, end: &Self, p: f32) -> Self;
}

impl Tween for f32 {
  fn tween(begin: &Self, end: &Self, p: f32) -> Self {
    tween_check!(begin, end, p);
    begin + (end - begin) * p
  }
}

impl Tween for f64 {
  fn tween(begin: &Self, end: &Self, p: f32) -> f64 {
    tween_check!(begin, end, p);
    begin + (end - begin) * p as f64
  }
}

impl<V: Tween + Clone> Tween for Option<V> {
  fn tween(begin: &Self, end: &Self, p: f32) -> Self {
    tween_check!(begin, end, p);
    match (begin, end) {
      (Some(b), Some(e)) => Some(Tween::tween(b, e, p)),
      _ => end.as_ref().map(|v| v.clone()),
    }
  }
}

#[macro_export]
macro_rules! tween_field {
  ($target: ident, $s1: ident, $s2: ident, $p: ident, {$($field: ident),*}) => {
    $(
        $target.$field = Tween::tween(&$s1.$field, &$s2.$field, $p);
    )*
  };
}

#[macro_export]
macro_rules! impl_tween_struct_default {
    ($struct: ident, {$($field: ident),*}) => {
        impl Tween for $struct {
            fn tween(begin: &$struct, end: &$struct, p: f32) -> $struct {
            tween_check!(begin, end, p);

            let mut target = $struct::default();
            tween_field!(target, begin, end, p, {$($field),*});
            target
            }
        }
    }
}

impl_tween_struct_default!(Color, {red, green, blue, alpha});
impl_tween_struct_default!(Point, {x, y});
impl_tween_struct_default!(Size, {width, height});
impl_tween_struct_default!(EdgeInsets, {left, right, bottom, top});
impl_tween_struct_default!(Radius, {top_left, top_right, bottom_left, bottom_right});

impl Tween for Brush {
  fn tween(begin: &Self, end: &Self, p: f32) -> Self {
    tween_check!(begin, end, p);
    match (begin, end) {
      (Brush::Color(c1), Brush::Color(c2)) => Brush::from(Tween::tween(c1, c2, p)),
      _ => end.clone(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  extern crate test;
  use test::Bencher;
  #[test]
  fn test_tween_f32() {
    let eq = |f1: f32, f2: f32| (f1 - f2).abs() < 0.0001;
    assert!(eq(Tween::tween(&0., &10., 0.5), 5.));
    assert!(eq(Tween::tween(&5., &10., 0.), 5.));
    assert!(eq(Tween::tween(&5., &10., 2.), 10.));
    assert!(eq(Tween::tween(&10., &0., 0.2), 8.));
  }

  #[test]
  fn test_tween_point() {
    let eq = |p1: Point, p2: Point| (p1.x - p2.x).abs() < 0.0001 && (p1.y - p2.y).abs() < 0.0001;
    assert!(eq(
      Tween::tween(&Point::new(0., 0.), &Point::new(0., 10.), 0.5),
      Point::new(0., 5.)
    ));
    assert!(eq(
      Tween::tween(&Point::new(10., 0.), &Point::new(0., 0.), 0.2),
      Point::new(8., 0.)
    ));
    assert!(eq(
      Tween::tween(&Point::new(20., 0.), &Point::new(0., 10.), 0.2),
      Point::new(16., 2.)
    ));
    assert!(eq(
      Tween::tween(&Point::new(10., 0.), &Point::new(0., 10.), 2.),
      Point::new(0., 10.)
    ));
  }

  #[bench]
  fn bench_tween_color(b: &mut Bencher) {
    b.iter(|| {
      let sum: u32 = (0..100)
        .map(|i| Tween::tween(&Color::from_u32(i), &Color::from_u32(0xff_ff_ff), 0.3).as_u32())
        .sum();
      sum
    })
  }
}
