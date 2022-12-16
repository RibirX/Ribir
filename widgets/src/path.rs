use std::rc::Rc;

use lyon_algorithms::{hit_test::hit_test_path, math::point, measure::{PathMeasurements, SampleType}};
use lyon_path::FillRule;
use ribir_core::{impl_query_self_only, prelude::*};

const TOLERANCE: f32 = 0.1;

/// Widget just use as a paint kit for a path and not care about its size.
/// Use `[PathWidget]!` instead of.
#[derive(Declare)]
pub struct PathPaintKit {
  pub path: Path,
  #[declare(convert=into)]
  pub brush: Brush,
}

impl PathPaintKit {
  pub fn path_lerp_fn<'a>(prop: impl Property<Value = Path>, style: PathStyle) -> impl Fn(&'a Path, &'a Path, f32) -> Path + Clone {
    let path = prop.get();
    let measurements = Rc::new(PathMeasurements::from_path(&path.path, 1e-3));
    move |_, _, rate| {
      let mut sampler = measurements.create_sampler(&path.path, SampleType::Normalized);
      let mut path_builder = Path::builder();
      sampler.split_range(0.0..rate, &mut path_builder.0);
      Path {
        path: path_builder.0.build(),
        style,
      }
    }
  }
  
  pub fn sample_lerp_fn(prop: impl Property<Value = Path>) -> impl FnMut(&Path, &Path, f32) -> Point {
    let mut measurements: Option<PathMeasurements> = None;
    let path = prop.get();
    move |_, _, rate| {
      let mut sampler = measurements
        .get_or_insert_with(|| PathMeasurements::from_path(&path.path, 1e-3))
        .create_sampler(&path.path, SampleType::Normalized);
      let sample  = sampler.sample(rate);
      let pos = sample.position();
      Point::new(pos.x, pos.y)
    }
  }
}

impl Render for PathPaintKit {
  #[inline]
  fn perform_layout(&self, _: BoxClamp, _: &mut LayoutCtx) -> Size { Size::zero() }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { true }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) {
    ctx
      .painter()
      .set_brush(self.brush.clone())
      .paint_path(self.path.clone());
  }

  #[inline]
  fn can_overflow(&self) -> bool { true }

  fn hit_test(&self, _ctx: &HitTestCtx, pos: Point) -> HitTest {
    let pt = point(pos.x, pos.y);
    // todo: support fillrule
    let is_hit = hit_test_path(
      &pt,
      self.path.path.into_iter(),
      FillRule::EvenOdd,
      TOLERANCE,
    );
    HitTest { hit: is_hit, can_hit_child: is_hit }
  }
}

impl Query for PathPaintKit {
  impl_query_self_only!();
}

#[derive(Declare)]
/// A path widget which size careful and can process events only if user hit at
/// the path self, not its size cover area.
pub struct PathWidget {
  pub path: Path,
  #[declare(convert=into)]
  pub brush: Brush,
}

/// Path widget just use as a paint kit for a path and not care about its size.
/// Use `[HitTesPath]!` instead of.
impl Render for PathWidget {
  #[inline]
  fn perform_layout(&self, _: BoxClamp, _: &mut LayoutCtx) -> Size { self.path.box_rect().size }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) {
    ctx
      .painter()
      .set_brush(self.brush.clone())
      .paint_path(self.path.clone());
  }
}

impl Query for PathWidget {
  impl_query_self_only!();
}

#[derive(Declare)]
/// Widget use to help directly paint dozens of paths, and not care about its
/// size.
pub struct PathsPaintKit {
  pub paths: Vec<PathPaintKit>,
}

impl Render for PathsPaintKit {
  #[inline]
  fn perform_layout(&self, _: BoxClamp, _: &mut LayoutCtx) -> Size { Size::zero() }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { true }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) { self.paths.iter().for_each(|p| p.paint(ctx)); }

  fn hit_test(&self, _ctx: &HitTestCtx, pos: Point) -> HitTest {
    let pt = point(pos.x, pos.y);
    let is_hit = self.paths.iter().any(|path| {
      hit_test_path(
        &pt,
        path.path.path.into_iter(),
        FillRule::EvenOdd,
        TOLERANCE,
      )
    });

    HitTest { hit: is_hit, can_hit_child: false }
  }
}

impl Query for PathsPaintKit {
  impl_query_self_only!();
}
