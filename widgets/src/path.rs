use lyon_algorithms::measure::{PathMeasurements, SampleType};
use ribir_core::{impl_query_self_only, prelude::*};
use std::rc::Rc;

/// Widget just use as a paint kit for a path and not care about its size.
/// Use `[PathWidget]!` instead of.
#[derive(Declare, Clone)]
pub struct PathPaintKit {
  pub path: PaintPath,
  #[declare(convert=into)]
  pub brush: Brush,
}

impl PathPaintKit {
  // pub fn path_lerp_fn(
  //   prop: impl Property<Value = Path>,
  // ) -> impl Fn(&Path, &Path, f32) -> Path + Clone {
  //   let path = prop.get();
  //   let measurements = Rc::new(PathMeasurements::from_path(&path.inner_path(),
  // 1e-3));   move |_, _, rate| {
  //     let mut sampler = measurements.create_sampler(path.inner_path(),
  // SampleType::Normalized);     let mut path_builder = Path::builder();
  //     sampler.split_range(0.0..rate, &mut path_builder.0);
  //     Path {
  //       path: path_builder.0.build(),
  //       style: path.style,
  //     }
  //   }
  // }

  // pub fn paths_lerp_fn(
  //   prop: impl Property<Value = Vec<PathPaintKit>> + Clone,
  // ) -> impl Fn(&Vec<PathPaintKit>, &Vec<PathPaintKit>, f32) ->
  // Vec<PathPaintKit> + Clone {   let paths = prop.get();
  //   let mut measurements_list = vec![];
  //   paths.iter().for_each(|path_paint_kit| {
  //     let measurements = PathMeasurements::from_path(&path_paint_kit.path.path,
  // 1e-3);     measurements_list.push(measurements);
  //   });
  //   let measurements_list = Rc::new(measurements_list);

  //   move |_, _, rate| {
  //     let mut total_len = 0.;
  //     let mut len_list = vec![];
  //     measurements_list.iter().enumerate().for_each(|(i, m)| {
  //       let sampler = m.create_sampler(&(paths[i].path.path),
  // SampleType::Normalized);       let len = sampler.length();
  //       len_list.push(len);
  //       total_len += len;
  //     });
  //     let real_len = total_len * rate;
  //     let mut rest_len = real_len;
  //     let mut rate = 0.;
  //     // find current rate at which path index
  //     let mut idx: usize = 0;
  //     for (i, len) in len_list.into_iter().enumerate() {
  //       if rest_len < len {
  //         rate = rest_len / len;
  //         idx = i;
  //         break;
  //       } else {
  //         rest_len -= len;
  //       }
  //     }
  //     let mut path_list = vec![];
  //     // before index path push path-list result
  //     for (i, path) in paths.iter().enumerate() {
  //       if i < idx {
  //         path_list.push(path.clone());
  //       } else {
  //         break;
  //       }
  //     }
  //     // generate rate rest path
  //     let path = paths[idx].clone();
  //     let measurements = PathMeasurements::from_path(&path.path.path, 1e-3);
  //     let mut sampler = measurements.create_sampler(&path.path.path,
  // SampleType::Normalized);     let mut path_builder = Path::builder();
  //     sampler.split_range(0.0..rate, &mut path_builder.0);
  //     path_list.push(PathPaintKit {
  //       path: Path {
  //         path: path_builder.0.build(),
  //         style: paths.get(idx).unwrap().path.style,
  //       },
  //       brush: path.brush,
  //     });
  //     path_list
  //   }
  // }

  // pub fn sample_lerp_fn(
  //   prop: impl Property<Value = Path>,
  // ) -> impl FnMut(&Path, &Path, f32) -> Point + Clone {
  //   let path = prop.get();
  //   let measurements = Rc::new(PathMeasurements::from_path(path.inner_path(),
  // 1e-3));   move |_, _, rate| {
  //     let mut sampler = measurements.create_sampler(path.inner_path(),
  // SampleType::Normalized);     let sample = sampler.sample(rate);
  //     let pos = sample.position();
  //     Point::new(pos.x, pos.y)
  //   }
  // }
}

impl Render for PathPaintKit {
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, _: &mut LayoutCtx) -> Size { clamp.max }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { true }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) {
    let size = ctx.box_rect().expect("must have layout").size;
    let path = PaintPath::rect(&Rect::from_size(size));
    ctx.painter().clip(path);
    ctx
      .painter()
      .set_brush(self.brush.clone())
      .paint_path(self.path.clone());
  }

  fn hit_test(&self, _ctx: &HitTestCtx, _: Point) -> HitTest {
    HitTest { hit: false, can_hit_child: false }
  }
}

impl Query for PathPaintKit {
  impl_query_self_only!();
}

#[derive(Declare)]
/// A path widget which size careful and can process events only if user hit at
/// the path self, not its size cover area.
pub struct PathWidget {
  pub path: PaintPath,
  #[declare(convert=into)]
  pub brush: Brush,
}

/// Path widget just use as a paint kit for a path and not care about its size.
/// Use `[HitTesPath]!` instead of.
impl Render for PathWidget {
  #[inline]
  fn perform_layout(&self, _: BoxClamp, _: &mut LayoutCtx) -> Size { self.path.bounds().size }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) {
    let size = ctx.box_rect().expect("must have layout").size;
    let path = PaintPath::rect(&Rect::from_size(size));
    ctx.painter().clip(path);
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
  fn perform_layout(&self, clamp: BoxClamp, _: &mut LayoutCtx) -> Size { clamp.max }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { true }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) { self.paths.iter().for_each(|p| p.paint(ctx)); }

  fn hit_test(&self, _ctx: &HitTestCtx, _: Point) -> HitTest {
    HitTest { hit: false, can_hit_child: false }
  }
}

impl Query for PathsPaintKit {
  impl_query_self_only!();
}
