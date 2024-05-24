use ribir_text::{Em, FontSize, Pixel, PIXELS_PER_EM};

use crate::prelude::{
  Angle, Box2D, Brush, Color, DevicePoint, DeviceRect, DeviceSize, DeviceVector, Point, Radius,
  Rect, Size, Transform, Vector,
};

/// Linearly interpolate between two value.
///
/// * `factor`: the percent of the distance between two value to advance.s
pub trait Lerp {
  fn lerp(&self, to: &Self, factor: f32) -> Self;
}

macro_rules! impl_lerp_for_integer {
  ($($ty: ident), *) => {
    $(
      impl Lerp for $ty {
        #[inline]
        fn lerp(&self, to: &Self, factor: f32) -> Self{
        (*self as f32 * (1. - factor)   +  *to as f32 * factor) as $ty
        }
      }
    )*
  }
}

impl_lerp_for_integer! { i8, i16, i32, i64, isize, u8, u16, u32, u64, usize }

impl Lerp for f32 {
  fn lerp(&self, to: &Self, factor: f32) -> Self { factor.mul_add(to - self, *self) }
}

impl Lerp for f64 {
  fn lerp(&self, to: &Self, factor: f32) -> Self { (factor as f64).mul_add(to - self, *self) }
}

impl Lerp for bool {
  fn lerp(&self, to: &Self, factor: f32) -> Self { if factor == 0. { *self } else { *to } }
}

impl<V: Lerp + Default> Lerp for Option<V> {
  fn lerp(&self, to: &Self, factor: f32) -> Self {
    match (self, to) {
      (Some(from), Some(to)) => Some(from.lerp(to, factor)),
      (None, None) => Some(<_>::default()),
      (None, Some(to)) => Some(V::default().lerp(to, factor)),
      (Some(from), None) => Some(from.lerp(&V::default(), factor)),
    }
  }
}

macro_rules! impl_lerp_for_tuple {
  ($ty: ident, $idx: tt $(,$other_ty: ident, $other_idx: tt)*) => {
    impl_lerp_for_tuple!({$ty, $idx} $($other_ty, $other_idx),*);
  };
  (
    {$($ty: ident, $idx: tt),+}
    $next_ty: ident, $next_idx: tt
    $(,$other_ty: ident, $other_idx: tt)*
  ) => {
      impl_lerp_for_tuple!({$($ty, $idx),+});
      impl_lerp_for_tuple!(
        {$($ty, $idx,)+ $next_ty, $next_idx }
        $($other_ty, $other_idx),*
      );
  };
  ({$($ty: ident, $index: tt),*}) => {
    impl <$($ty: Lerp,)*> Lerp for ($($ty),*,) {
      fn lerp(&self, to: &Self, factor: f32) -> Self {
        ($( self.$index.lerp(&to.$index, factor),)*)
      }
    }
  }
}

impl_lerp_for_tuple! {T0, 0, T1, 1, T2, 2, T3, 3, T4, 4, T5, 5, T6, 6, T7, 7,T8, 8, T9, 9,
  T10, 10, T11, 11, T12, 12, T13, 13, T14, 14, T15, 15, T16, 16, T17, 17,T18, 18, T19, 19,
  T20, 20, T21, 21, T22, 22, T23, 23, T24, 24, T25, 25, T26, 26, T27, 27,T28, 28, T29, 29,
  T30, 30, T31, 31
}

macro_rules! impl_lerp_for_copy_geom {
  ($($ty: ident), *) => {
    $(
      impl Lerp for $ty {
        #[inline]
        fn lerp(&self, to: &Self, factor: f32) -> Self{
          $ty::lerp(*self, *to, factor)
        }
      }
    )*
  }
}

macro_rules! impl_lerp_for_geom {
  ($($ty: ident), *) => {
    $(
      impl Lerp for $ty {
        #[inline]
        fn lerp(&self, to: &Self, factor: f32) -> Self{
          $ty::lerp(self, *to, factor)
        }
      }
    )*
  }
}

macro_rules! impl_lerp_for_device_geom {
  ($($ty: ident), *) => {
    $(
      impl Lerp for $ty {
        #[inline]
        fn lerp(&self, to: &Self, factor: f32) -> Self{
          self.to_f32().lerp(to.to_f32(), factor).to_i32()
        }
      }
    )*
  }
}

impl_lerp_for_copy_geom! { Point, Size, Vector }
impl_lerp_for_geom! { Rect, Angle, Box2D }
impl_lerp_for_device_geom! { DeviceRect, DevicePoint, DeviceSize, DeviceVector }

impl Lerp for Radius {
  #[inline]
  fn lerp(&self, to: &Self, factor: f32) -> Self {
    Self::new(
      self.top_left.lerp(&to.top_left, factor),
      self.top_right.lerp(&to.top_right, factor),
      self.bottom_left.lerp(&to.bottom_left, factor),
      self.bottom_right.lerp(&to.bottom_right, factor),
    )
  }
}

impl Lerp for Color {
  fn lerp(&self, to: &Self, factor: f32) -> Self {
    Self::new(
      self.red.lerp(&to.red, factor),
      self.green.lerp(&to.green, factor),
      self.blue.lerp(&to.blue, factor),
      self.alpha.lerp(&to.alpha, factor),
    )
  }
}

impl Lerp for Brush {
  fn lerp(&self, to: &Self, factor: f32) -> Self {
    match (self, to) {
      (Brush::Color(from), Brush::Color(to)) => from.lerp(to, factor).into(),
      _ => {
        // todo: only support pure color brush now.
        to.clone()
      }
    }
  }
}

impl Lerp for Transform {
  fn lerp(&self, to: &Self, factor: f32) -> Self {
    let m11 = self.m11.lerp(&to.m11, factor);
    let m12 = self.m12.lerp(&to.m12, factor);
    let m21 = self.m21.lerp(&to.m21, factor);
    let m22 = self.m22.lerp(&to.m22, factor);
    let m31 = self.m31.lerp(&to.m31, factor);
    let m32 = self.m32.lerp(&to.m32, factor);

    Transform::new(m11, m12, m21, m22, m31, m32)
  }
}

impl Lerp for Pixel {
  #[inline]
  fn lerp(&self, to: &Self, factor: f32) -> Self {
    let v = (**self).lerp(to, factor);
    v.into()
  }
}

impl Lerp for Em {
  #[inline]
  fn lerp(&self, to: &Self, factor: f32) -> Self {
    let v = self.value().lerp(&to.value(), factor);
    Em::relative_to(v, FontSize::Pixel(PIXELS_PER_EM.into()))
  }
}

impl Lerp for FontSize {
  fn lerp(&self, to: &Self, factor: f32) -> Self {
    let from = self.into_pixel().value();
    let to = to.into_pixel().value();
    let v = from.lerp(&to, factor);
    FontSize::Pixel(v.into())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn lerp_f32() {
    let eq = |f1: f32, f2: f32| (f1 - f2).abs() < f32::EPSILON;
    assert!(eq(Lerp::lerp(&0., &10., 0.5), 5.));
    assert!(eq(Lerp::lerp(&5., &10., 0.), 5.));
    assert!(eq(Lerp::lerp(&5., &10., 2.), 15.));
    assert!(eq(Lerp::lerp(&10., &0., 0.2), 8.));
  }

  #[test]
  fn lerp_point() {
    let eq = |p1: Point, p2: Point| {
      (p1 - p2)
        .abs()
        .lower_than(Vector::new(f32::EPSILON, f32::EPSILON))
        .all()
    };
    assert!(eq(Lerp::lerp(&Point::new(0., 0.), &Point::new(0., 10.), 0.5), Point::new(0., 5.)));
    assert!(eq(Lerp::lerp(&Point::new(10., 0.), &Point::new(0., 0.), 0.2), Point::new(8., 0.)));
    assert!(eq(Lerp::lerp(&Point::new(20., 0.), &Point::new(0., 10.), 0.2), Point::new(16., 2.)));
    assert!(eq(Lerp::lerp(&Point::new(10., 0.), &Point::new(0., 10.), 2.), Point::new(-10., 20.)));
  }

  #[test]
  fn lerp_tuple() {
    let t1 = (0., 0.5, Point::new(10., 0.));
    let t2 = (1., 1., Point::new(10., 10.));

    assert!((0.5, 0.75, Point::new(10., 5.)) == Lerp::lerp(&t1, &t2, 0.5));
    assert!(t2 == Lerp::lerp(&t1, &t2, 1.));
    assert!(t1 == Lerp::lerp(&t1, &t2, 0.));
  }

  #[test]
  fn fix_avoid_calc_overflow() {
    assert_eq!(255u8.lerp(&0u8, 0.), 255);
  }
}
