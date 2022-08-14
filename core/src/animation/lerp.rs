use crate::prelude::{
  Angle, Box2D, Brush, Color, DevicePoint, DeviceRect, DeviceSize, DeviceVector, Point, Radius,
  Rect, Size, Vector,
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
          self + ((to - self) as f32 * factor )as $ty
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
  ($({$param: ident, $index: tt},)*) => {
    impl <$($param: Lerp,)*> Lerp for ($($param),*,) {
      fn lerp(&self, to: &Self, factor: f32) -> Self {
        ($( self.$index.lerp(&to.$index, factor),)*)
      }
    }
  }
}

impl_lerp_for_tuple!({T0, 0},);
impl_lerp_for_tuple!({T0, 0}, {T1, 1},);
impl_lerp_for_tuple!({T0, 0}, {T1, 1}, {T2, 2},);
impl_lerp_for_tuple!({T0, 0}, {T1, 1}, {T2, 2}, {T3, 3},);
impl_lerp_for_tuple!({T0, 0}, {T1, 1}, {T2, 2}, {T3, 3}, {T4, 4},);
impl_lerp_for_tuple!({T0, 0}, {T1, 1}, {T2, 2}, {T3, 3}, {T4, 4}, {T5, 5},);
impl_lerp_for_tuple!({T0, 0}, {T1, 1}, {T2, 2}, {T3, 3}, {T4, 4}, {T5, 5}, {T6, 6},);
impl_lerp_for_tuple!({T0, 0}, {T1, 1}, {T2, 2}, {T3, 3}, {T4, 4}, {T5, 5}, {T6, 6}, {T7, 7},);
impl_lerp_for_tuple!({T0, 0}, {T1, 1}, {T2, 2}, {T3, 3}, {T4, 4}, {T5, 5}, {T6, 6}, {T7, 7}, {T8, 8},);
impl_lerp_for_tuple!({T0, 0}, {T1, 1}, {T2, 2}, {T3, 3}, {T4, 4}, {T5, 5}, {T6, 6}, {T7, 7}, {T8, 8}, {T9, 9},);
impl_lerp_for_tuple!({T0, 0}, {T1, 1}, {T2, 2}, {T3, 3}, {T4, 4}, {T5, 5}, {T6, 6}, {T7, 7}, {T8, 8}, {T9, 9}, {T10, 10},);
impl_lerp_for_tuple!({T0, 0}, {T1, 1}, {T2, 2}, {T3, 3}, {T4, 4}, {T5, 5}, {T6, 6}, {T7, 7}, {T8, 8}, {T9, 9}, {T10, 10}, {T11, 11},);

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
          self.to_f32().lerp(to.to_f32(), factor).to_u32()
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

#[cfg(test)]
mod tests {
  use super::*;

  extern crate test;
  use test::Bencher;
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
    assert!(eq(
      Lerp::lerp(&Point::new(0., 0.), &Point::new(0., 10.), 0.5),
      Point::new(0., 5.)
    ));
    assert!(eq(
      Lerp::lerp(&Point::new(10., 0.), &Point::new(0., 0.), 0.2),
      Point::new(8., 0.)
    ));
    assert!(eq(
      Lerp::lerp(&Point::new(20., 0.), &Point::new(0., 10.), 0.2),
      Point::new(16., 2.)
    ));
    assert!(eq(
      Lerp::lerp(&Point::new(10., 0.), &Point::new(0., 10.), 2.),
      Point::new(-10., 20.)
    ));
  }

  #[test]
  fn lerp_tuple() {
    let t1 = (0., 0.5, Point::new(10., 0.));
    let t2 = (1., 1., Point::new(10., 10.));

    assert!((0.5, 0.75, Point::new(10., 5.)) == Lerp::lerp(&t1, &t2, 0.5));
    assert!(t2 == Lerp::lerp(&t1, &t2, 1.));
    assert!(t1 == Lerp::lerp(&t1, &t2, 0.));
  }

  #[bench]
  fn bench_lerp_color(b: &mut Bencher) {
    b.iter(|| {
      let sum: u32 = (0..100)
        .map(|i| Lerp::lerp(&Color::from_u32(i), &Color::from_u32(0xff_ff_ff), 0.3).into_u32())
        .sum();
      sum
    })
  }
}