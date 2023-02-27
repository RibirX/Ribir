use euclid::default::Box2D;
use lyon_algorithms::{
  aabb::bounding_box,
  geom::{CubicBezierSegment, QuadraticBezierSegment},
  measure::{PathMeasurements, SampleType},
};
use lyon_path::{
  math::{Point, Vector},
  path::Builder,
  Event,
};
use ribir::prelude::{font_db::FontDB, shaper::TextShaper, *};
use std::{
  f32::consts::PI,
  mem::MaybeUninit,
  ops::RangeInclusive,
  sync::{Arc, RwLock},
  time::Duration,
};

fn main() {
  let mut font_db = FontDB::default();
  font_db.load_system_fonts();
  let font_db = Arc::new(RwLock::new(font_db));
  let shaper = TextShaper::new(font_db.clone());
  let reorder = TextReorder::default();
  let typography_store = TypographyStore::new(reorder.clone(), font_db, shaper.clone());
  let text_style = TextStyle {
    font_size: FontSize::Pixel(240.0.into()),
    ..Default::default()
  };
  let init_path = get_text_paths(&typography_store, "2", &text_style)
    .into_iter()
    .map(|path| PathPaintKit {
      path,
      brush: Brush::Color(Color::BLACK),
    })
    .collect::<Vec<_>>();
  let finally_path = get_text_paths(&typography_store, "1", &text_style)
    .into_iter()
    .map(|path| PathPaintKit {
      path,
      brush: Brush::Color(Color::BLACK),
    })
    .collect::<Vec<_>>();

  let w = widget! {
    Column {
      Container {
        size: Size::new(300., 300.),
        PathsPaintKit {
          id: path_kit,
          paths: finally_path,
        }
      }
      Button {
        on_tap: move |_| {
          animate.run();
        },
        ButtonText::new("RUN")
      }
    }

    Animate {
      id: animate,
      transition: Transition {
        delay: Some(Duration::from_millis(200)),
        duration: Duration::from_millis(10000),
        easing: easing::LINEAR,
        repeat: None,
      },
      prop: prop!(path_kit.paths, char_path_lerp_fn()),
      from: init_path,
    }
  };

  app::run(w);
}

/// char convert to path lerp
fn char_path_lerp_fn()
-> impl Fn(&Vec<PathPaintKit>, &Vec<PathPaintKit>, f32) -> Vec<PathPaintKit> + Clone {
  move |from, to, rate| {
    let mut result = vec![];
    let init_path = &from[0].path.path;
    let finally_path = &to[0].path.path;

    let center_point = get_paths_center_point(vec![init_path, finally_path]);

    let init_path_points = get_points_from_path(init_path);
    let finally_path_points = get_points_from_path(finally_path);

    // let path_pair = find_nearest_path_pair(&init_path_points,
    // &finally_path_points);
    let path_pair = vec![
      (Some(0), Some(0)),
    ];

    for (op1, op2) in path_pair {
      if op1.is_some() && op2.is_some() {
        let s_idx = op1.unwrap();
        let t_idx = op2.unwrap();
        let (s_path_points, t_path_points) = fit_nearest_path_points(
          &init_path_points[s_idx],
          &finally_path_points[t_idx],
          center_point,
        );
        let last_idx = s_path_points.0.len() - 1;
        let mut result_path = lyon_path::Path::builder();
        for (idx, scp) in s_path_points.0.iter().enumerate() {
          let tcp = &t_path_points.0[idx];
          if idx == 0 {
            result_path.begin(scp.end_point + (tcp.end_point - scp.end_point) * rate);
          } else {
            let to = scp.end_point + (tcp.end_point - scp.end_point) * rate;
            let ctrl1 = scp.ctrl1 + (tcp.ctrl1 - scp.ctrl1) * rate;
            let ctrl2 = scp.ctrl2 + (tcp.ctrl2 - scp.ctrl2) * rate;

            result_path.cubic_bezier_to(ctrl1, ctrl2, to);

            if idx == last_idx {
              result_path.close();
            }
          }
        }

        let path = result_path.build();

        result.push(PathPaintKit {
          path: Path {
            path,
            // style: PathStyle::Fill,
            style: PathStyle::Stroke(StrokeOptions::default()),
          },
          brush: Brush::Color(Color::BLACK),
        });
      } else if op1.is_some() {
        let s_idx = op1.unwrap();
        let s_path_points = &init_path_points[s_idx];
        let t_point = CubicPoint {
          end_point: center_point,
          ctrl1: center_point,
          ctrl2: center_point,
        };
        let last_idx = s_path_points.0.len() - 1;
        let mut result_path = lyon_path::Path::builder();
        for (idx, scp) in s_path_points.0.iter().enumerate() {
          let tcp = &t_point;
          if idx == 0 {
            result_path.begin(scp.end_point + (tcp.end_point - scp.end_point) * rate);
          } else {
            let to = scp.end_point + (tcp.end_point - scp.end_point) * rate;
            let ctrl1 = scp.ctrl1 + (tcp.ctrl1 - scp.ctrl1) * rate;
            let ctrl2 = scp.ctrl2 + (tcp.ctrl2 - scp.ctrl2) * rate;

            result_path.cubic_bezier_to(ctrl1, ctrl2, to);

            if idx == last_idx {
              result_path.close();
            }
          }
        }

        let path = result_path.build();

        result.push(PathPaintKit {
          path: Path {
            path,
            // style: PathStyle::Fill,
            style: PathStyle::Stroke(StrokeOptions::default()),
          },
          brush: Brush::Color(Color::BLACK),
        });
      } else if op2.is_some() {
        let t_idx = op2.unwrap();
        let t_path_points = &finally_path_points[t_idx];
        let last_idx = t_path_points.0.len() - 1;
        let mut result_path = lyon_path::Path::builder();
        let s_point = CubicPoint {
          end_point: center_point,
          ctrl1: center_point,
          ctrl2: center_point,
        };

        for (idx, tcp) in t_path_points.0.iter().enumerate() {
          let scp = &s_point;
          if idx == 0 {
            result_path.begin(scp.end_point + (tcp.end_point - scp.end_point) * rate);
          } else {
            let to = scp.end_point + (tcp.end_point - scp.end_point) * rate;
            let ctrl1 = scp.ctrl1 + (tcp.ctrl1 - scp.ctrl1) * rate;
            let ctrl2 = scp.ctrl2 + (tcp.ctrl2 - scp.ctrl2) * rate;

            result_path.cubic_bezier_to(ctrl1, ctrl2, to);

            if idx == last_idx {
              result_path.close();
            }
          }
        }

        let path = result_path.build();

        result.push(PathPaintKit {
          path: Path {
            path,
            // style: PathStyle::Fill,
            style: PathStyle::Stroke(StrokeOptions::default()),
          },
          brush: Brush::Color(Color::BLACK),
        });
      } else {
        unreachable!("It is impossible that all paths cannot match");
      }
    }
    result
  }
}

#[derive(Debug, Clone)]
struct PathPoints(Vec<CubicPoint>);

#[derive(Debug, Clone, Copy)]
struct CubicPoint {
  // cubic segment end point
  end_point: Point,
  // cubic segment two control point c1, c2
  ctrl1: Point,
  ctrl2: Point,
}

/// One path has many end points and control points, this function will collect
/// these points as `CubicPoint`. A path may be have sub path, we will split it
/// to many path.
fn get_points_from_path(path: &lyon_path::Path) -> Vec<PathPoints> {
  let mut multi_paths = vec![];
  for evt in path.iter() {
    match evt {
      Event::Begin { at } => {
        let mut cur = PathPoints(vec![]);
        cur
          .0
          .push(CubicPoint { end_point: at, ctrl1: at, ctrl2: at });
        multi_paths.push(cur);
      }
      Event::Line { from: _, to } => {
        if let Some(cur) = multi_paths.last_mut() {
          cur
            .0
            .push(CubicPoint { end_point: to, ctrl1: to, ctrl2: to });
        } else {
          unreachable!("Path must be start with Event::Begin!");
        }
      }
      Event::Quadratic { from, ctrl, to } => {
        if let Some(cur) = multi_paths.last_mut() {
          let CubicBezierSegment { to, ctrl1, ctrl2, .. } =
            QuadraticBezierSegment { from, ctrl, to }.to_cubic();
          cur.0.push(CubicPoint { end_point: to, ctrl1, ctrl2 });
        } else {
          unreachable!("Path must be start with Event::Begin!");
        }
      }
      Event::Cubic { from: _, ctrl1, ctrl2, to } => {
        if let Some(cur) = multi_paths.last_mut() {
          cur.0.push(CubicPoint { end_point: to, ctrl1, ctrl2 });
        } else {
          unreachable!("Path must be start with Event::Begin!");
        }
      }
      Event::End { last: _, first: _, close: _ } => {}
    }
  }

  multi_paths
}

/// Get two path center point
fn get_paths_center_point(paths: Vec<&lyon_path::Path>) -> Point {
  paths
    .iter()
    .fold(Box2D::default(), |box_2d, path| {
      box_2d.union(&bounding_box(path.iter()))
    })
    .center()
}

#[derive(Clone, Copy, Debug)]
struct QuadrantPoint {
  pt: Point,
  vc: Vector,
  idx: usize,
  offset_percent: f32,
}

const QUADRANT_COUNT: usize = 4;

fn get_point_by_quadrant(
  path_points: &PathPoints,
  center_point: Point,
) -> [Vec<QuadrantPoint>; QUADRANT_COUNT] {
  let mut quadrants: [Vec<_>; QUADRANT_COUNT] = {
    let mut data: [MaybeUninit<Vec<QuadrantPoint>>; QUADRANT_COUNT] =
      unsafe { MaybeUninit::uninit().assume_init() };
    for elem in &mut data[..] {
      elem.write(vec![]);
    }
    unsafe { std::mem::transmute::<_, [Vec<QuadrantPoint>; QUADRANT_COUNT]>(data) }
  };

  let avg_distance = path_points
    .0
    .iter()
    .fold(0., |acc, cp| acc + cp.end_point.distance_to(center_point))
    / (path_points.0.len() as f32);

  let unit_pi = 2. * PI / (QUADRANT_COUNT as f32);

  for (idx, cp) in path_points.0.iter().enumerate() {
    let pt = cp.end_point;
    let vc = pt - center_point;
    let Angle { mut radians } = vc.angle_from_x_axis();
    radians += PI;
    let offset_percent = (pt.distance_to(center_point) - avg_distance).abs() / avg_distance;
    let i = (radians / unit_pi).floor() as usize;
    // let mut insert_idx = (&quadrants[u]).len();
    // for (i, elem) in (&quadrants[u]).iter().enumerate() {
    //   if offset_percent >= elem.offset_percent {
    //     insert_idx = i;
    //     break;
    //   }
    // }
    // quadrants[u].insert(insert_idx, QuadrantPoint { pt: pt, vc, idx,
    // offset_percent });
    quadrants[i].push(QuadrantPoint { pt, vc, idx, offset_percent });
  }

  quadrants
}

/// Get the character path through text and text_style
fn get_text_paths<T: Into<Substr>>(
  typography_store: &TypographyStore,
  text: T,
  style: &TextStyle,
) -> Vec<Path> {
  let visual_glyphs = typography_with_text_style(typography_store, text, style, None);
  let glyphs = visual_glyphs.pixel_glyphs().collect::<Vec<_>>();
  glyphs
    .into_iter()
    .map(|g| {
      let Glyph {
        glyph_id,
        face_id,
        x_offset,
        y_offset,
        ..
      } = g;
      let face = {
        let mut font_db = typography_store.shaper.font_db_mut();
        font_db
          .face_data_or_insert(face_id)
          .expect("Font face not exist!")
          .clone()
      };
      let font_size_ems = style.font_size.into_pixel().value();
      let t = euclid::Transform2D::default()
        .pre_translate((x_offset.value(), y_offset.value()).into())
        .pre_scale(font_size_ems, font_size_ems);
      Path {
        path: face.outline_glyph(glyph_id).unwrap().transformed(&t),
        style: PathStyle::Stroke(StrokeOptions::default()),
        // style: PathStyle::Fill,
      }
    })
    .collect::<Vec<_>>()
}

fn match_pair_path_key_points(
  source: &PathPoints,
  target: &PathPoints,
  center_point: Point,
) -> Vec<(usize, usize)> {
  let source_quadrants = get_point_by_quadrant(source, center_point);
  let target_quadrants = get_point_by_quadrant(target, center_point);

  let mut source_first_idx = None;
  let mut target_first_idx = None;
  let mut pair_result = vec![];

  let mut infos = [0; QUADRANT_COUNT];

  // Iterate the collected points by quadrant
  for i in 0..QUADRANT_COUNT {
    if (&source_quadrants[i]).len() != 0 && (&target_quadrants[i]).len() != 0 {
      for (idx, sq) in (&source_quadrants[i]).iter().enumerate() {
        if let Some(tq) = (&target_quadrants[i]).get(idx) {
          if source_first_idx.is_none() && target_first_idx.is_none() {
            source_first_idx = Some(sq.idx);
            target_first_idx = Some(tq.idx);
            pair_result.push((sq.idx, tq.idx));
          } else {
            let mut insert_idx = pair_result.len();
            let sq_idx = if sq.idx < source_first_idx.unwrap() {
              sq.idx + source.0.len()
            } else {
              sq.idx
            };
            let tq_idx = if tq.idx < target_first_idx.unwrap() {
              tq.idx + target.0.len()
            } else {
              tq.idx
            };

            let mut has_cross = false;
            let mut remove_idxs = vec![];
            for (idx, (old_sq, old_tq)) in pair_result.iter().enumerate() {
              if (old_sq > &sq_idx && old_tq < &tq_idx) || (old_sq < &sq_idx && old_tq > &tq_idx) {
                if infos[i] > 1 {
                  has_cross = true;
                  break;
                } else {
                  remove_idxs.push(idx);
                }
              }

              if old_sq > &sq_idx {
                insert_idx = idx - 1;
              }
            }

            if remove_idxs.len() > 0 {
              remove_idxs.reverse();
              for idx in remove_idxs {
                if idx <= insert_idx {
                  insert_idx -= 1;
                }
                pair_result.remove(idx);
              }
            }

            if !has_cross {
              pair_result.insert(insert_idx, (sq_idx, tq_idx));
            }
          }
        }
      }
    }
  }

  pair_result
}

struct PointLenIdx {
  len: f32,
  idx: usize,
}

#[derive(Clone)]
struct PointPerIdx {
  per: f32,
  idx: usize,
}

fn fill_path_with_points_range(
  path_points: &PathPoints,
  range: &RangeInclusive<usize>,
  builder: &mut Builder,
) -> Vec<PointLenIdx> {
  let mut distance_list = vec![];
  let mut prev_pt = None;
  let path_points_len = path_points.0.len();

  for i in (*range).clone() {
    let i = i % path_points_len;
    let path_point = &path_points.0[i];
    if prev_pt.is_none() {
      prev_pt = Some(path_point);
      distance_list.push(PointLenIdx { idx: i, len: 0. });
      builder.begin(path_point.end_point);
    } else {
      let cur_pt = path_point;
      let &CubicPoint { end_point: to, ctrl1, ctrl2 } = cur_pt;
      builder.cubic_bezier_to(ctrl1, ctrl2, to);
      let seg = CubicBezierSegment {
        from: prev_pt.unwrap().end_point,
        ctrl1,
        ctrl2,
        to,
      };
      let len = if seg.from == seg.to {
        0.
      } else {
        seg.approximate_length(1e-3)
      };
      distance_list.push(PointLenIdx { idx: i, len });
      prev_pt = Some(path_point);
    }
  }
  builder.close();
  distance_list
}

fn fit_nearest_path_points(
  source: &PathPoints,
  target: &PathPoints,
  center_point: Point,
) -> (PathPoints, PathPoints) {
  let mut pair = (PathPoints(vec![]), PathPoints(vec![]));

  let pair_idx = match_pair_path_key_points(source, target, center_point);
  let source_len = source.0.len();
  let target_len = target.0.len();

  let first_pair_idx_from_source = pair_idx[0].0;
  let first_pair_idx_from_target = pair_idx[0].1;
  let mut prev_pair_idx_from_source = first_pair_idx_from_source;
  let mut prev_pair_idx_from_target = first_pair_idx_from_target;
  let pair_last_idx = pair_idx.len() - 1;

  for i in 1..pair_idx.len() {
    let (source_idx, target_idx) = pair_idx[i];

    let source_range = generate_range(prev_pair_idx_from_source, source_idx, source_len);
    let target_range = generate_range(prev_pair_idx_from_target, target_idx, target_len);

    prev_pair_idx_from_source = source_idx;
    prev_pair_idx_from_target = target_idx;

    fill_lerp_pair(source, target, &mut pair, &source_range, &target_range);

    // last item must end to end.
    if i == pair_last_idx {
      let source_range = generate_range(source_idx, first_pair_idx_from_source, source_len);
      let target_range = generate_range(target_idx, first_pair_idx_from_target, target_len);

      fill_lerp_pair(source, target, &mut pair, &source_range, &target_range);
    }
  }

  pair
}

fn generate_range(start: usize, end: usize, len: usize) -> RangeInclusive<usize> {
  if start > end {
    return start..=(end + len);
  }

  start..=end
}

fn fill_lerp_pair(
  source: &PathPoints,
  target: &PathPoints,
  pair: &mut (PathPoints, PathPoints),
  source_range: &RangeInclusive<usize>,
  target_range: &RangeInclusive<usize>,
) {
  let mut source_path_builder = lyon_path::Path::builder();
  let (mut per_source, source_distance_count) =
    get_path_percent_by_points_range(source, &source_range, &mut source_path_builder);
  let source_path = source_path_builder.build();
  let mut target_path_builder = lyon_path::Path::builder();
  let (mut per_target, target_distance_count) =
    get_path_percent_by_points_range(target, &target_range, &mut target_path_builder);
  let target_path = target_path_builder.build();

  let mut per_source_backup = per_source.clone();
  let mut per_target_backup = per_target.clone();

  // === source ===

  let source_measurements = PathMeasurements::from_path(&source_path, 1e-3);
  let mut source_sampler = source_measurements.create_sampler(&source_path, SampleType::Normalized);

  // fill first item, remove last item.
  // interpolation middle item.
  let first_per_source_item = per_source.first().unwrap().clone();
  let last_per_source_item = per_source.last().unwrap().clone();

  if per_source.len() > 0 && per_target.len() > 0 {
    per_source.remove(0);
    pair
      .0
      .0
      .push((&source.0[first_per_source_item.idx]).clone());
    per_target.remove(0);
  }

  while per_source.len() > 0 && per_target.len() > 0 {
    if per_source.first().is_some() && per_target.first().is_some() {
      let per_source_item = (&per_source[0]).clone();
      let per_target_item = (&per_target[0]).clone();
      if per_source_item.per <= per_target_item.per {
        if per_source.len() > 1 {
          pair.0.0.push((&source.0[per_source_item.idx]).clone());
        }
        per_source.remove(0);
      } else {
        per_target.remove(0);
        if source_distance_count > 0. {
          let pt = source_sampler.sample(per_target_item.per).position();
          pair
            .0
            .0
            .push(CubicPoint { end_point: pt, ctrl1: pt, ctrl2: pt });
        } else {
          pair.0.0.push((&source.0[last_per_source_item.idx]).clone());
        };
      }
    }
  }

  while per_source.len() > 0 {
    if per_source.first().is_some() {
      let per_source_item = (&per_source[0]).clone();
      per_source.remove(0);
      if per_source.len() > 1 {
        pair.0.0.push((&source.0[per_source_item.idx]).clone());
      }
    }
  }

  while per_target.len() > 0 {
    if per_target.first().is_some() {
      let per_target_item = (&per_target[0]).clone();
      if per_target_item.per == 1. && per_target.len() == 1 {
        break;
      }
      per_target.remove(0);
      if source_distance_count > 0. {
        let sample = source_sampler.sample(per_target_item.per);
        let pt = sample.position();
        pair
          .0
          .0
          .push(CubicPoint { end_point: pt, ctrl1: pt, ctrl2: pt });
      } else {
        pair.0.0.push((&source.0[last_per_source_item.idx]).clone());
      }
    }
  }

  // === target ===

  let target_measurements = PathMeasurements::from_path(&target_path, 1e-3);
  let mut target_sampler = target_measurements.create_sampler(&target_path, SampleType::Normalized);

  // fill first item, remove last item.
  // interpolation middle item.
  let first_per_target_item = per_target_backup.first().unwrap().clone();
  let last_per_target_item = per_target_backup.last().unwrap().clone();

  if per_source_backup.len() > 0 && per_target_backup.len() > 0 {
    per_target_backup.remove(0);
    pair
      .1
      .0
      .push((&target.0[first_per_target_item.idx]).clone());
    per_source_backup.remove(0);
  }

  while per_source_backup.len() > 0 && per_target_backup.len() > 0 {
    if per_source_backup.first().is_some() && per_target_backup.first().is_some() {
      let per_source_item = (&per_source_backup[0]).clone();
      let per_target_item = (&per_target_backup[0]).clone();
      if per_source_item.per < per_target_item.per {
        per_source_backup.remove(0);
        if target_distance_count > 0. {
          let sample = target_sampler.sample(per_source_item.per);
          let pt = sample.position();
          pair
            .1
            .0
            .push(CubicPoint { end_point: pt, ctrl1: pt, ctrl2: pt });
        } else {
          pair.1.0.push((&target.0[last_per_target_item.idx]).clone());
        }
      } else {
        if per_target_backup.len() > 1 {
          pair.1.0.push((&target.0[per_target_item.idx]).clone());
        }
        per_target_backup.remove(0);
      }
    }
  }

  while per_target_backup.len() > 0 {
    if per_target_backup.first().is_some() {
      let per_target_item = (&per_target_backup[0]).clone();
      if per_target_backup.len() > 1 {
        pair.1.0.push((&target.0[per_target_item.idx]).clone());
      }
      per_target_backup.remove(0);
    }
  }

  while per_source_backup.len() > 0 {
    if per_source_backup.first().is_some() {
      let per_source_item = (&per_source_backup[0]).clone();
      if per_source_item.per == 1. && per_source_backup.len() == 1 {
        break;
      }
      per_source_backup.remove(0);
      if target_distance_count > 0. {
        let sample = target_sampler.sample(per_source_item.per);
        let pt = sample.position();
        pair
          .1
          .0
          .push(CubicPoint { end_point: pt, ctrl1: pt, ctrl2: pt });
      } else {
        pair.1.0.push((&target.0[last_per_target_item.idx]).clone());
      }
    }
  }
}

fn get_path_percent_by_points_range(
  path_points: &PathPoints,
  range: &RangeInclusive<usize>,
  builder: &mut Builder,
) -> (Vec<PointPerIdx>, f32) {
  let distances = fill_path_with_points_range(path_points, range, builder);
  let distance_count = distances.iter().fold(0., |acc, x| acc + x.len);
  let mut path_per = vec![];
  let mut count = 0.;
  for dis in distances {
    count += dis.len;
    path_per.push(PointPerIdx {
      idx: dis.idx,
      per: if count == 0. {
        0.
      } else {
        count / distance_count
      },
    });
  }
  (path_per, distance_count)
}

fn find_nearest_path_pair(
  source: &Vec<PathPoints>,
  target: &Vec<PathPoints>,
) -> Vec<(Option<usize>, Option<usize>)> {
  todo!()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn path_single_move_to_cubic_point() {
    let mut path = lyon_path::Path::builder();
    path.begin(Point::new(25., 25.));
    path.line_to(Point::new(30., 30.));
    path.line_to(Point::new(45., 60.));
    path.line_to(Point::new(25., 80.));
    path.line_to(Point::new(25., 25.));
    path.end(true);

    let path = path.build();
    let points = get_points_from_path(&path);
    let expect_result = vec![PathPoints(vec![
      CubicPoint {
        end_point: Point::new(25., 25.),
        ctrl1: Point::new(25., 25.),
        ctrl2: Point::new(25., 25.),
      },
      CubicPoint {
        end_point: Point::new(30., 30.),
        ctrl1: Point::new(30., 30.),
        ctrl2: Point::new(30., 30.),
      },
      CubicPoint {
        end_point: Point::new(45., 60.),
        ctrl1: Point::new(45., 60.),
        ctrl2: Point::new(45., 60.),
      },
      CubicPoint {
        end_point: Point::new(25., 80.),
        ctrl1: Point::new(25., 80.),
        ctrl2: Point::new(25., 80.),
      },
      CubicPoint {
        end_point: Point::new(25., 25.),
        ctrl1: Point::new(25., 25.),
        ctrl2: Point::new(25., 25.),
      },
    ])];

    assert_eq!(points, expect_result);
  }

  #[test]
  fn path_multi_move_to_cubic_point() {
    let mut path = lyon_path::Path::builder();
    path.begin(Point::new(25., 25.));
    path.line_to(Point::new(30., 30.));
    path.line_to(Point::new(45., 60.));
    path.line_to(Point::new(25., 80.));
    path.line_to(Point::new(25., 25.));
    path.end(false);
    path.begin(Point::new(45., 45.));
    path.line_to(Point::new(60., 60.));
    path.line_to(Point::new(80., 20.));
    path.line_to(Point::new(45., 45.));
    path.end(true);

    let path = path.build();
    let points = get_points_from_path(&path);
    let expect_result = vec![
      PathPoints(vec![
        CubicPoint {
          end_point: Point::new(25., 25.),
          ctrl1: Point::new(25., 25.),
          ctrl2: Point::new(25., 25.),
        },
        CubicPoint {
          end_point: Point::new(30., 30.),
          ctrl1: Point::new(30., 30.),
          ctrl2: Point::new(30., 30.),
        },
        CubicPoint {
          end_point: Point::new(45., 60.),
          ctrl1: Point::new(45., 60.),
          ctrl2: Point::new(45., 60.),
        },
        CubicPoint {
          end_point: Point::new(25., 80.),
          ctrl1: Point::new(25., 80.),
          ctrl2: Point::new(25., 80.),
        },
        CubicPoint {
          end_point: Point::new(25., 25.),
          ctrl1: Point::new(25., 25.),
          ctrl2: Point::new(25., 25.),
        },
      ]),
      PathPoints(vec![
        CubicPoint {
          end_point: Point::new(45., 45.),
          ctrl1: Point::new(45., 45.),
          ctrl2: Point::new(45., 45.),
        },
        CubicPoint {
          end_point: Point::new(60., 60.),
          ctrl1: Point::new(60., 60.),
          ctrl2: Point::new(60., 60.),
        },
        CubicPoint {
          end_point: Point::new(80., 20.),
          ctrl1: Point::new(80., 20.),
          ctrl2: Point::new(80., 20.),
        },
        CubicPoint {
          end_point: Point::new(45., 45.),
          ctrl1: Point::new(45., 45.),
          ctrl2: Point::new(45., 45.),
        },
      ]),
    ];

    assert_eq!(points, expect_result);
  }
}
