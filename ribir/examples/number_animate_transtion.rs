use ribir::prelude::{*, font_db::FontDB, shaper::TextShaper};
use std::{sync::{Arc, RwLock}, time::Duration, ops::Add, collections::HashSet};
use lyon_path::{math::Point};
use lyon_algorithms::geom::QuadraticBezierSegment;

#[derive(Debug, Clone, Copy, PartialEq)]
struct CubicPoint {
  end_point: Point,
  ctrl_points: Option<(Point, Point)>,
}

fn get_points_from_path(path: &lyon_path::Path) -> Vec<Vec<CubicPoint>> {
  let mut collection: Vec<Vec<CubicPoint>> = vec![];
  for event in path.iter() {
    match event {
      lyon_path::Event::Begin { at } => {
        let mut current = vec![];
        current.push(CubicPoint { end_point: at, ctrl_points: None });
        collection.push(current);
      },
      lyon_path::Event::Line { from, to } => {
        if let Some(current) = collection.last_mut() {
          current.push(CubicPoint { end_point: to, ctrl_points: Some((to, to)) });
        } else {
          panic!("Path must be start with Event::Begin!");
        }
      },
      lyon_path::Event::Quadratic { from, ctrl, to } => {
        if let Some(current) = collection.last_mut() {
          let seg = QuadraticBezierSegment { from, ctrl, to }.to_cubic();
          current.push(CubicPoint { end_point: seg.to, ctrl_points: Some((seg.ctrl1, seg.ctrl2)) });
        } else {
          panic!("Path must be start with Event::Begin!");
        }
      },
      lyon_path::Event::Cubic { from, ctrl1, ctrl2, to } => {
        if let Some(current) = collection.last_mut() {
          current.push(CubicPoint { end_point: to, ctrl_points: Some((ctrl1, ctrl2)) });
        } else {
          panic!("Path must be start with Event::Begin!");
        }
      },
      lyon_path::Event::End { last, first, close } => {},
    }
  }

  collection
}

#[derive(PartialEq)]
enum QuadrantName {
  first,
  second,
  third,
  fourth,
}

fn get_quadrant_name_by_center_point(
  point: Point,
  center_point: Point,
) -> QuadrantName {
  let vector = point - center_point;
  let is_horizontal_positive = vector.x > 0.;
  let is_vertical_positive = vector.y > 0.;
  if is_horizontal_positive && is_vertical_positive {
    return QuadrantName::first;
  }

  if !is_horizontal_positive && is_vertical_positive {
    return QuadrantName::second;
  }
  
  if !is_horizontal_positive && !is_vertical_positive {
    return QuadrantName::third;
  }
  
  if is_horizontal_positive && !is_vertical_positive {
    return QuadrantName::fourth;
  }

  panic!("Can't find quadrant");
}

fn find_nearest_point_pair(
  source_points: &Vec<CubicPoint>,
  target_points: &Vec<CubicPoint>,
  center_point: Point,
) -> Vec<(usize, usize)> {
  let source_len = source_points.len();
  let target_len = target_points.len();
  let mut pair = vec![];

  if source_len >= target_len {
    let [
      first_target,
      second_target,
      third_target,
      fourth_target,
    ] = get_quadrant_by_center_point(target_points, center_point);

    let [
      first_source,
      second_source,
      third_source,
      fourth_source,
    ] = get_quadrant_by_center_point(source_points, center_point);

    for (idx, cp) in source_points.iter().enumerate() {
      let CubicPoint { end_point, .. } = cp;
      let quadrant_name = get_quadrant_name_by_center_point(*end_point, center_point);

      let search_order = match quadrant_name {
        QuadrantName::first => [&first_target, &second_target, &third_target, &fourth_target],
        QuadrantName::second => [&second_target, &third_target, &fourth_target, &first_target],
        QuadrantName::third => [&third_target, &fourth_target, &first_target, &second_target],
        QuadrantName::fourth => [&fourth_target, &first_target, &second_target, &third_target],
      };

      let mut min_distance = f32::INFINITY;
      let mut find_target_idx = usize::MAX;
      if search_order[0].len() > 0 {
        for (target, target_idx) in search_order[0] {
          let distance = end_point.distance_to(target.end_point);
          if distance < min_distance {
            min_distance = distance;
            find_target_idx = *target_idx;
          }
        }
      } else if search_order[1].len() > 0 {
        for (target, target_idx) in search_order[1] {
          let distance = end_point.distance_to(target.end_point);
          if distance < min_distance {
            min_distance = distance;
            find_target_idx = *target_idx;
          }
        }
      } else if search_order[2].len() > 0 {
        for (target, target_idx) in search_order[2] {
          let distance = end_point.distance_to(target.end_point);
          if distance < min_distance {
            min_distance = distance;
            find_target_idx = *target_idx;
          }
        }
      } else if search_order[3].len() > 0 {
        for (target, target_idx) in search_order[3] {
          let distance = end_point.distance_to(target.end_point);
          if distance < min_distance {
            min_distance = distance;
            find_target_idx = *target_idx;
          }
        }
      } else {
        panic!("")
      }

      pair.push((idx, find_target_idx));
    }
  } else {
    // reversed
    let [
      first_source,
      second_source,
      third_source,
      fourth_source,
    ] = get_quadrant_by_center_point(source_points, center_point);

    todo!()
  }

  println!("");

  return pair;
}

fn paths_pair(
  source_paths: &Vec<Vec<CubicPoint>>,
  target_paths: &Vec<Vec<CubicPoint>>,
  center_point: Point,
) -> Vec<(Option<usize>, Option<usize>)> {
  assert!(source_paths.len() > 0, "source paths size is zero!");
  assert!(target_paths.len() > 0, "target paths size is zero!");

  let mut pair = vec![];

  let mut source_quadrant_list = vec![];
  for source_path in source_paths {
    source_quadrant_list.push(get_quadrant_by_center_point(source_path, center_point));
  }

  let source_variance = get_points_variance_by_center_point(&source_quadrant_list, center_point);

  let mut target_quadrant_list = vec![];
  for target_path in target_paths {
    target_quadrant_list.push(get_quadrant_by_center_point(target_path, center_point));
  }

  let target_variance = get_points_variance_by_center_point(&target_quadrant_list, center_point);
  let mut delete_source_idx = HashSet::new();
  let mut delete_target_idx = HashSet::new();

  for (source_idx, source) in source_variance.iter().enumerate() {
    let mut min_variance = f32::INFINITY;
    let mut mark_target_idx = usize::MAX;

    for (target_idx, target) in target_variance.iter().enumerate() {
      if delete_target_idx.contains(&target_idx) {
        continue;
      }

      let diff_0 = source[0] - target[0];
      let diff_1 = source[1] - target[1];
      let diff_2 = source[2] - target[2];
      let diff_3 = source[3] - target[3];
      let ave = (diff_0 + diff_1 + diff_2 + diff_3) / 4.;

      let variance_0 = (diff_0 - ave).powf(2.);
      let variance_1 = (diff_1 - ave).powf(2.);
      let variance_2 = (diff_2 - ave).powf(2.);
      let variance_3 = (diff_3 - ave).powf(2.);
      let variance = (variance_0 + variance_1 + variance_2 + variance_3) / 4.;
      
      if variance < min_variance {
        min_variance = variance;
        mark_target_idx = target_idx;
      }
    }

    if mark_target_idx != usize::MAX {
      delete_source_idx.insert(source_idx);
      delete_target_idx.insert(mark_target_idx);
      pair.push((Some(source_idx), Some(mark_target_idx)));
    }
  }

  if source_variance.len() > delete_source_idx.len() {
    for (idx, _) in source_variance.iter().enumerate() {
      if !delete_source_idx.contains(&idx) {
        pair.push((Some(idx), None));
      }
    }
  }

  if target_variance.len() > delete_target_idx.len() {
    for (idx, _) in target_variance.iter().enumerate() {
      if !delete_target_idx.contains(&idx) {
        pair.push((None, Some(idx)));
      }
    }
  }

  pair
}

fn get_quadrant_by_center_point(
  path: &Vec<CubicPoint>,
  center_point: Point,
) -> [Vec<(CubicPoint, usize)>; 4] {
  let mut quadrant: [Vec<(CubicPoint, usize)>; 4] = [Vec::default(), Vec::default(), Vec::default(), Vec::default()];
  let mut first_quadrant_list = vec![];
  let mut second_quadrant_list = vec![];
  let mut third_quadrant_list = vec![];
  let mut fourth_quadrant_list = vec![];

  for (idx, cp) in path.iter().enumerate() {
    let CubicPoint { end_point, .. } = cp;
    let quadrant_name = get_quadrant_name_by_center_point(*end_point, center_point);
    
    match quadrant_name {
      QuadrantName::first => first_quadrant_list.push((*cp, idx)),
      QuadrantName::second => second_quadrant_list.push((*cp, idx)),
      QuadrantName::third => third_quadrant_list.push((*cp, idx)),
      QuadrantName::fourth => fourth_quadrant_list.push((*cp, idx)),
    }
  }

  quadrant[0] = first_quadrant_list;
  quadrant[1] = second_quadrant_list;
  quadrant[2] = third_quadrant_list;
  quadrant[3] = fourth_quadrant_list;

  quadrant
}

fn get_points_variance_by_center_point(
  paths: &Vec<[Vec<(CubicPoint, usize)>; 4]>,
  center_point: Point,
) -> Vec<[f32; 4]> {
  let mut variance = vec![];
  for path in paths.iter() {
    let mut quadrant_arr: [f32; 4] = [0.; 4];
    // iter for each quadrant
    for (quadrant_idx, cp_list) in path.iter().enumerate() {
      let mut count = 0.;
      let mut distances = vec![];
      for cp in cp_list.iter() {
        let CubicPoint { end_point, .. } = cp.0;
        let distance = end_point.distance_to(center_point);
        distances.push(distance);
        count += distance;
      }
      let average = if count == 0. { 0. } else { count / (cp_list.len() as f32) };
      let mut quadrant_count = 0.;
      for distance in distances {
        quadrant_count += (distance - average).powf(2.);
      }
      let quadrant = if quadrant_count == 0. { 0. } else { quadrant_count / (cp_list.len() as f32) };
      quadrant_arr[quadrant_idx] = quadrant;
    }
    variance.push(quadrant_arr)
  }

  variance
}

fn get_text_paths<T: Into<Substr>>(
  typography_store: &TypographyStore,
  text: T,
  style: &TextStyle,
) -> Vec<Path> {
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

fn text_path_lerp_fn() -> impl Fn(
  &Vec<PathPaintKit>,
  &Vec<PathPaintKit>,
  f32,
) -> Vec<PathPaintKit> + Clone {
  move |from, to, rate| {
    let from_path = &from.get(0).unwrap().path.path;
    let from_path_points = get_points_from_path(from_path);
    let to_path = &to.get(0).unwrap().path.path;
    let to_path_points = get_points_from_path(to_path);
    let center_point = Point::new(48., 96.);
    let paths_pair = paths_pair(&from_path_points, &to_path_points, center_point);
    let mut paths = vec![];
    for (from_idx, to_idx) in paths_pair {
      if from_idx.is_none() {

      }
      if to_idx.is_none() {

      }
      if from_idx.is_some() && to_idx.is_some() {
        let from_idx = from_idx.unwrap();
        let to_idx = to_idx.unwrap();
        let from_path = from_path_points.get(from_idx).unwrap();
        let to_path = to_path_points.get(to_idx).unwrap();
        let point_pair = find_nearest_point_pair(&from_path, &to_path, center_point);

        let mut result_path = lyon_path::Path::builder();
        let last_idx = point_pair.len() - 1;
        for (idx, (from_idx, to_idx)) in point_pair.iter().enumerate() {
          if idx == 0 {
            let from = from_path.get(*from_idx).unwrap().end_point;
            let to = to_path.get(*to_idx).unwrap().end_point;
            result_path.begin(from.add((to - from) * rate));
          }

          let CubicPoint { ctrl_points: from_ctrls, end_point: from_end_point} = from_path.get(*from_idx).unwrap().clone();
          let CubicPoint { ctrl_points: to_ctrls, end_point: to_end_point} = to_path.get(*to_idx).unwrap().clone();
          let to = from_end_point.add((to_end_point - from_end_point) * rate);

          if to_ctrls.is_some() && from_ctrls.is_some() {
            let ctrl1 = from_ctrls.unwrap().0.add((to_ctrls.unwrap().0 - from_ctrls.unwrap().0) * rate);
            let ctrl2 = from_ctrls.unwrap().1.add((to_ctrls.unwrap().1 - from_ctrls.unwrap().1) * rate);
            result_path.cubic_bezier_to(ctrl1, ctrl2, to);
          } else if from_ctrls.is_some() {
            let ctrl1 = from_ctrls.unwrap().0.add((to_end_point - from_ctrls.unwrap().0) * rate);
            let ctrl2 = from_ctrls.unwrap().1.add((to_end_point - from_ctrls.unwrap().1) * rate);
            result_path.cubic_bezier_to(ctrl1, ctrl2, to);
          } else if to_ctrls.is_some() {
            let ctrl1 = (from_end_point).add((to_ctrls.unwrap().0 - (from_end_point)) * rate);
            let ctrl2 = (from_end_point).add((to_ctrls.unwrap().1 - (from_end_point)) * rate);
            result_path.cubic_bezier_to(ctrl1, ctrl2, to);
          }

          if idx == last_idx {
            result_path.end(true);
          }
        }

        paths.push(PathPaintKit {
          path: Path {
            path: result_path.build(),
            style: PathStyle::Stroke(StrokeOptions::default()),
          },
          brush: Brush::Color(Color::BLACK),
        })
      }
    }

    paths
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
      let init_path = get_text_paths(&typography_store, "2", &text_style);
      let init_paths = vec![
        PathPaintKit {
          path: init_path.get(0).unwrap().clone(),
          brush: Brush::Color(Color::BLACK),
        }
      ];
      let finally_path = get_text_paths(&typography_store, "1", &text_style);
      let finally_paths = vec![
        PathPaintKit {
          path: finally_path.get(0).unwrap().clone(),
          brush: Brush::Color(Color::BLACK),
        }
      ];
    }

    PathsPaintKit {
      id: path_kit,
      paths: finally_paths,
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
      prop: prop!(path_kit.paths, text_path_lerp_fn()),
      from: init_paths,
    }
  };

  app::run(w);
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
    let expect_result = vec![
      vec![
        CubicPoint { end_point: Point::new(25., 25.), ctrl_points: None },
        CubicPoint { end_point: Point::new(30., 30.), ctrl_points: Some((Point::new(30., 30.), Point::new(30., 30.))) },
        CubicPoint { end_point: Point::new(45., 60.), ctrl_points: Some((Point::new(45., 60.), Point::new(45., 60.))) },
        CubicPoint { end_point: Point::new(25., 80.), ctrl_points: Some((Point::new(25., 80.), Point::new(25., 80.))) },
        CubicPoint { end_point: Point::new(25., 25.), ctrl_points: Some((Point::new(25., 25.), Point::new(25., 25.))) },
      ]
    ];

    assert_eq!(points, expect_result);
  }

  #[test]
  fn path_multi_move_to_cubic_point() {
    let mut path = lyon_path::Path::builder(); path.begin(Point::new(25., 25.));
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
      vec![
        CubicPoint { end_point: Point::new(25., 25.), ctrl_points: None },
        CubicPoint { end_point: Point::new(30., 30.), ctrl_points: Some((Point::new(30., 30.), Point::new(30., 30.))) },
        CubicPoint { end_point: Point::new(45., 60.), ctrl_points: Some((Point::new(45., 60.), Point::new(45., 60.))) },
        CubicPoint { end_point: Point::new(25., 80.), ctrl_points: Some((Point::new(25., 80.), Point::new(25., 80.))) },
        CubicPoint { end_point: Point::new(25., 25.), ctrl_points: Some((Point::new(25., 25.), Point::new(25., 25.))) },
      ],
      vec![
        CubicPoint { end_point: Point::new(45., 45.), ctrl_points: None },
        CubicPoint { end_point: Point::new(60., 60.), ctrl_points: Some((Point::new(60., 60.), Point::new(60., 60.))) },
        CubicPoint { end_point: Point::new(80., 20.), ctrl_points: Some((Point::new(80., 20.), Point::new(80., 20.))) },
        CubicPoint { end_point: Point::new(45., 45.), ctrl_points: Some((Point::new(45., 45.), Point::new(45., 45.))) },
      ]
    ];
  }

  #[test]
  fn points_quadrant_by_center_point() {
    let mut path = lyon_path::Path::builder();
    path.begin(Point::new(25., 25.)); // third
    path.line_to(Point::new(30., 30.)); // third
    path.line_to(Point::new(45., 60.)); // second
    path.line_to(Point::new(55., 80.)); // first
    path.line_to(Point::new(55., 20.)); // fourth
    path.line_to(Point::new(25., 25.)); // third
    path.end(true);

    let path = path.build();
    let center_point = Point::new(50., 50.);
    let binding = get_points_from_path(&path);
    let path_points = binding.get(0).unwrap();
    let quadrant_result = get_quadrant_by_center_point(path_points, center_point);
    let expect_result = [
      vec![
        (CubicPoint { end_point: Point::new(55., 80.), ctrl_points: Some((Point::new(55., 80.), Point::new(55., 80.))) }, 3),
      ],
      vec![
        (CubicPoint { end_point: Point::new(45., 60.), ctrl_points: Some((Point::new(45., 60.), Point::new(45., 60.))) }, 2),
      ],
      vec![
        (CubicPoint { end_point: Point::new(25., 25.), ctrl_points: None }, 0),
        (CubicPoint { end_point: Point::new(30., 30.), ctrl_points: Some((Point::new(30., 30.), Point::new(30., 30.))) }, 1),
        (CubicPoint { end_point: Point::new(25., 25.), ctrl_points: Some((Point::new(25., 25.), Point::new(25., 25.))) }, 5),
      ],
      vec![
        (CubicPoint { end_point: Point::new(55., 20.), ctrl_points: Some((Point::new(55., 20.), Point::new(55., 20.))) }, 4),
      ],
    ];
    assert_eq!(quadrant_result, expect_result);
  }

  #[test]
  fn points_variance_by_center_point() {
    let mut path = lyon_path::Path::builder();
    path.begin(Point::new(25., 25.)); // third
    path.line_to(Point::new(30., 30.)); // third
    path.line_to(Point::new(45., 60.)); // second
    path.line_to(Point::new(55., 80.)); // first
    path.line_to(Point::new(55., 20.)); // fourth
    path.line_to(Point::new(25., 25.)); // third
    path.end(true);

    let path = path.build();
    let center_point = Point::new(50., 50.);

  }
}
