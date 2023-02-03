use ribir::prelude::{*, font_db::FontDB, shaper::TextShaper};
use std::{sync::{Arc, RwLock}, time::Duration, ops::Add};
use lyon_path::{math::Point};
use lyon_algorithms::geom::QuadraticBezierSegment;

#[derive(Debug)]
struct CubicPoint {
  end_point: Point,
  ctrl_points: Option<(Point, Point)>,
}

fn get_points_from_path(path: &lyon_path::Path) -> Vec<CubicPoint> {
  let mut points_collection: Vec<CubicPoint> = vec![];
  for evt in path.iter() {
    match evt {
      lyon_path::Event::Begin { at } => {
        points_collection.push(CubicPoint { end_point: at, ctrl_points: None });
      },
      lyon_path::Event::Line { from, to } => {
        points_collection.push(CubicPoint { end_point: to, ctrl_points: Some((to, to)) })
      },
      lyon_path::Event::Quadratic { from, ctrl, to } => {
        let seg = QuadraticBezierSegment { from, ctrl, to }.to_cubic();
        points_collection.push(CubicPoint { end_point: seg.to, ctrl_points: Some((seg.ctrl1, seg.ctrl2)) });
      },
      lyon_path::Event::Cubic { from, ctrl1, ctrl2, to } => {
        points_collection.push(CubicPoint { end_point: to, ctrl_points: Some((ctrl1, ctrl2)) });
      },
      lyon_path::Event::End { last, first, close } => {
        // points_collection.push(CubicPoint { end_point: last, ctrl_points: (last, last) });
      },
    }
  }

  points_collection
}

fn find_nearest_point_pair(
  from_points: &Vec<CubicPoint>,
  to_points: &Vec<CubicPoint>,
) -> Vec<(usize, usize)> {
  assert!(from_points.len() > 0, "from points size is zero!");
  assert!(to_points.len() > 0, "to points size is zero");

  let mut pair = vec![];

  // Whether to reverse
  let is_reversed = to_points.len() >= from_points.len();
  let (source, target) = if is_reversed {
    (to_points, from_points)
  } else {
    (from_points, to_points)
  };

  let mut min_distance = f32::INFINITY;
  // find find_point first point pair to target point index.
  let source_first_point = source.get(0).unwrap();
  let mut source_first_point_pair_target_idx = 0;

  for (found_idx, found_cp) in target.iter().enumerate() {
    let CubicPoint { end_point: to_end_point, .. } = found_cp;
    let distance = to_end_point.distance_to(source_first_point.end_point.clone());
    if distance < min_distance {
      min_distance = distance;
      source_first_point_pair_target_idx = found_idx;
    }
  }

  let mut target_sort_index = vec![];
  for idx in source_first_point_pair_target_idx..target.len() {
    target_sort_index.push((target_sort_index.len(), idx));
  }
  for idx in 0..source_first_point_pair_target_idx {
    target_sort_index.push((target_sort_index.len(), idx));
  }

  let mut current_found_target_idx = 0;
  let mut prev_found_target_idx = current_found_target_idx;

  for (find_idx, find_cp) in source.iter().enumerate() {
    let mut min_distance = f32::INFINITY;
    // skip first point
    if find_idx == 0 {
      if is_reversed {
        pair.push((source_first_point_pair_target_idx, find_idx));
      } else {
        pair.push((find_idx, source_first_point_pair_target_idx));
      }
      continue;
    }

    let CubicPoint { end_point: find_end_point, .. } = find_cp;
    let mut found_sort_idx = current_found_target_idx;
    loop {
      if found_sort_idx >= target_sort_index.len() {
        break;
      }

      let (_, found_idx) = target_sort_index.get(found_sort_idx).unwrap();
      let found_cp = target.get(*found_idx);
      if let Some(found_cp) = found_cp {
        let CubicPoint { end_point: found_end_point, .. } = found_cp;
        let distance = found_end_point.distance_to(find_end_point.clone());
        if distance <= min_distance {
          min_distance = distance;
          current_found_target_idx = found_sort_idx;
        }
      }

      found_sort_idx += 1;
    }

    let (_, source_found_idx) = target_sort_index.get(current_found_target_idx).unwrap();

    for idx in (prev_found_target_idx + 1)..current_found_target_idx {
      let (_, idx) = target_sort_index.get(idx).unwrap();
      if is_reversed {
        pair.push((*idx, find_idx));
      } else {
        pair.push((find_idx, *idx));
      }
    }

    prev_found_target_idx = current_found_target_idx;

    if is_reversed {
      pair.push((*source_found_idx, find_idx));
    } else {
      pair.push((find_idx, *source_found_idx));
    }
  }
  
  pair
}

fn get_text_path<T: Into<Substr>>(typography_store: &TypographyStore, text: T, style: &TextStyle) -> Vec<Path> {
  let visual_glyphs = typography_with_text_style(typography_store, text, style, None);
    let glyphs = visual_glyphs.pixel_glyphs().collect::<Vec<_>>();
    glyphs.into_iter().map(|g| {
      let Glyph { glyph_id, face_id, x_offset, y_offset, .. } = g;
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
    }).collect::<Vec<_>>()
}

fn text_path_lerp_fn() -> impl Fn(&Path, &Path, f32) -> Path + Clone {
  move |from, to, rate| {
    let from_path_points = get_points_from_path(&from.path);
    // println!("from_path_points {:?}", from_path_points);
    let to_path_points = get_points_from_path(&to.path);
    // println!("to_path_points {:?}", to_path_points);
    let pair_list = find_nearest_point_pair(&from_path_points, &to_path_points);
    // println!("pair_list {:?}", pair_list);
    let last_idx = pair_list.len() - 1;

    let mut result_path = lyon_path::Path::builder();
    
    for (idx, (from_idx, to_idx)) in pair_list.into_iter().enumerate() {
      let x_rate = 1.;
      let fast_rate = if rate * x_rate > 1.0 { 1.0 } else { rate * x_rate };

      if idx == 0 {
        let from = from_path_points.get(from_idx).unwrap().end_point;
        let to = to_path_points.get(to_idx).unwrap().end_point;
        result_path.begin(from.add((to - from) * rate));
      }

      let CubicPoint { ctrl_points: from_ctrls, end_point: from_end_point} = from_path_points.get(from_idx).unwrap().clone();
      let CubicPoint { ctrl_points: to_ctrls, end_point: to_end_point} = to_path_points.get(to_idx).unwrap().clone();

      let to = from_end_point.add((*to_end_point - *from_end_point) * rate);

      if to_ctrls.is_some() && from_ctrls.is_some() {
        let ctrl1 = from_ctrls.unwrap().0.add((to_ctrls.unwrap().0 - from_ctrls.unwrap().0) * fast_rate);
        let ctrl2 = from_ctrls.unwrap().1.add((to_ctrls.unwrap().1 - from_ctrls.unwrap().1) * fast_rate);
        result_path.cubic_bezier_to(ctrl1, ctrl2, to);
      } else if from_ctrls.is_some() {
        let ctrl1 = from_ctrls.unwrap().0.add((*to_end_point - from_ctrls.unwrap().0) * fast_rate);
        let ctrl2 = from_ctrls.unwrap().1.add((*to_end_point - from_ctrls.unwrap().1) * fast_rate);
        result_path.cubic_bezier_to(ctrl1, ctrl2, to);
      } else if to_ctrls.is_some() {
        let ctrl1 = (*from_end_point).add((to_ctrls.unwrap().0 - (*from_end_point)) * fast_rate);
        let ctrl2 = (*from_end_point).add((to_ctrls.unwrap().1 - (*from_end_point)) * fast_rate);
        result_path.cubic_bezier_to(ctrl1, ctrl2, to);
      } else {
        result_path.end(false);
        result_path.begin(to);
      }
      
      if idx == last_idx {
        result_path.end(true);
      }
    }

    let path = result_path.build();

    Path {
      path,
      // style: PathStyle::Fill,
      style: PathStyle::Stroke(StrokeOptions::default()),
    }
  }
}

fn main() {
  let mut font_db = FontDB::default();
  font_db.load_system_fonts();
  let font_db = Arc::new(RwLock::new(font_db));
  let shaper = TextShaper::new(font_db.clone());
  let reorder = TextReorder::default();
  let typography_store = TypographyStore::new(reorder.clone(), font_db, shaper.clone());

  let w = widget! {
    init ctx => {
      let text_style = TextStyle {
        font_size: FontSize::Pixel(192.0.into()),
        ..TypographyTheme::of(ctx).headline1.text.clone()
      };
      let init_path = get_text_path(&typography_store, "7", &text_style);
      let finally_path = get_text_path(&typography_store, "1", &text_style);
    }
    
    PathPaintKit {
      id: path_kit,
      path: finally_path.get(0).unwrap().clone(),
      brush: Color::BLACK,
      mounted: move |_| {
        animate.run();
      }
    }

    Animate {
      id: animate,
      transition: Transition {
        delay: Some(Duration::from_millis(1000)),
        duration: Duration::from_millis(5000),
        // duration: Duration::from_millis(u64::MAX),
        easing: easing::LINEAR,
        repeat: None,
      },
      prop: prop!(path_kit.path, text_path_lerp_fn()),
      from: init_path.get(0).unwrap().clone(),
    }
  };

  app::run(w);
}


#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn two_points_pair() {
    let from_points = vec![
      CubicPoint { end_point: Point::new(48.46875, 96.0), ctrl_points: None }, 
      CubicPoint { end_point: Point::new(2.953125, 96.0), ctrl_points: Some((Point::new(2.953125, 96.0), Point::new(2.953125, 96.0))) }, 
      CubicPoint { end_point: Point::new(7.0078125, 84.234375), ctrl_points: Some((Point::new(2.953125, 92.125), Point::new(4.3046875, 88.203125))) }, 
      CubicPoint { end_point: Point::new(22.171875, 69.328125), ctrl_points: Some((Point::new(9.7109375, 80.265625), Point::new(14.765625, 75.296875))) }, 
      CubicPoint { end_point: Point::new(34.3125, 57.914063), ctrl_points: Some((Point::new(26.703125, 65.703125), Point::new(30.75, 61.898438))) }, 
      CubicPoint { end_point: Point::new(39.65625, 45.890625), ctrl_points: Some((Point::new(37.875, 53.929688), Point::new(39.65625, 49.921875))) }, 
      CubicPoint { end_point: Point::new(35.976563, 37.289063), ctrl_points: Some((Point::new(39.65625, 42.421875), Point::new(38.429688, 39.554688))) }, 
      CubicPoint { end_point: Point::new(26.671875, 33.890625), ctrl_points: Some((Point::new(33.523438, 35.023438), Point::new(30.421875, 33.890625))) }, 
      CubicPoint { end_point: Point::new(17.15625, 37.382813), ctrl_points: Some((Point::new(22.890625, 33.890625), Point::new(19.71875, 35.054688))) },
      CubicPoint { end_point: Point::new(13.265625, 47.8125), ctrl_points: Some((Point::new(14.59375, 39.710938), Point::new(13.296875, 43.1875))) },
      CubicPoint { end_point: Point::new(4.59375, 46.828125), ctrl_points: Some((Point::new(4.59375, 46.828125), Point::new(4.59375, 46.828125))) }, 
      CubicPoint { end_point: Point::new(11.4609375, 32.0625), ctrl_points: Some((Point::new(5.125, 40.390625), Point::new(7.4140625, 35.46875))) }, 
      CubicPoint { end_point: Point::new(26.859375, 26.953125), ctrl_points: Some((Point::new(15.5078125, 28.65625), Point::new(20.640625, 26.953125))) }, 
      CubicPoint { end_point: Point::new(42.632813, 32.554688), ctrl_points: Some((Point::new(33.609375, 26.953125), Point::new(38.867188, 28.820313))) }, 
      CubicPoint { end_point: Point::new(48.28125, 46.078125), ctrl_points: Some((Point::new(46.398438, 36.289063), Point::new(48.28125, 40.796875))) }, 
      CubicPoint { end_point: Point::new(43.523438, 59.460938), ctrl_points: Some((Point::new(48.28125, 50.484375), Point::new(46.695313, 54.945313))) }, 
      CubicPoint { end_point: Point::new(24.5625, 77.578125), ctrl_points: Some((Point::new(40.351563, 63.976563), Point::new(34.03125, 70.015625))) }, 
      CubicPoint { end_point: Point::new(14.671875, 87.84375), ctrl_points: Some((Point::new(19.59375, 81.546875), Point::new(16.296875, 84.96875))) }, 
      CubicPoint { end_point: Point::new(48.46875, 87.84375), ctrl_points: Some((Point::new(48.46875, 87.84375), Point::new(48.46875, 87.84375))) }, 
      CubicPoint { end_point: Point::new(48.46875, 96.0), ctrl_points: Some((Point::new(48.46875, 96.0), Point::new(48.46875, 96.0))) }];

    let to_points = vec![
      CubicPoint { end_point: Point::new(36.0, 96.0), ctrl_points: None },
      CubicPoint { end_point: Point::new(27.5625, 96.0), ctrl_points: Some((Point::new(27.5625, 96.0), Point::new(27.5625, 96.0))) },
      CubicPoint { end_point: Point::new(27.5625, 42.234375), ctrl_points: Some((Point::new(27.5625, 42.234375), Point::new(27.5625, 42.234375))) },
      CubicPoint { end_point: Point::new(10.640625, 52.40625), ctrl_points: Some((Point::new(23.34375, 46.234375), Point::new(17.703125, 49.625))) }, 
      CubicPoint { end_point: Point::new(10.640625, 44.25), ctrl_points: Some((Point::new(10.640625, 44.25), Point::new(10.640625, 44.25))) }, 
      CubicPoint { end_point: Point::new(30.515625, 26.953125), ctrl_points: Some((Point::new(20.359375, 39.59375), Point::new(26.984375, 33.828125))) }, 
      CubicPoint { end_point: Point::new(36.0, 26.953125), ctrl_points: Some((Point::new(36.0, 26.953125), Point::new(36.0, 26.953125))) }, 
      CubicPoint { end_point: Point::new(36.0, 96.0), ctrl_points: Some((Point::new(36.0, 96.0), Point::new(36.0, 96.0))) }];
      
    let rst = find_nearest_point_pair(&from_points, &to_points);
    let pair_list = vec![
      (0, 0),
      (1, 1),
      (2, 1),
      (3, 2),
      (3, 3),
      (4, 3),
      (5, 4),
      (5, 5),
      (5, 6),
      (6, 6),
      (7, 6),
      (8, 6), 
      (9, 6), 
      (10, 6), 
      (11, 6), 
      (12, 6), 
      (13, 6),
      (14, 6),
      (15, 6),
      (16, 7),
      (17, 7),
      (18, 7),
      (19, 7)];
    assert_eq!(rst, pair_list);
  }
}
