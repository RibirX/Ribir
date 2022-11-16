use super::{Direction, Expanded};
use ribir_core::{
  impl_query_self_only,
  prelude::{log::warn, *},
};

/// How the children should be placed along the main axis in a flex layout.
#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub enum JustifyContent {
  /// Place the children as close to the start of the main axis as possible.
  #[default]
  Start,
  ///Place the children as close to the middle of the main axis as possible.
  Center,
  /// Place the children as close to the end of the main axis as possible.
  End,
  /// The children are evenly distributed within the alignment container along
  /// the main axis. The spacing between each pair of adjacent items is the
  /// same. The first item is flush with the main-start edge, and the last
  /// item is flush with the main-end edge.
  SpaceBetween,
  /// The children are evenly distributed within the alignment container
  /// along the main axis. The spacing between each pair of adjacent items is
  /// the same. The empty space before the first and after the last item
  /// equals half of the space between each pair of adjacent items.
  SpaceAround,
  /// The children are evenly distributed within the alignment container along
  /// the main axis. The spacing between each pair of adjacent items, the
  /// main-start edge and the first item, and the main-end edge and the last
  /// item, are all exactly the same.
  SpaceEvenly,
}

#[derive(Default, MultiChild, Declare, Clone, PartialEq)]
pub struct Flex {
  /// Reverse the main axis.
  #[declare(default)]
  pub reverse: bool,
  /// Whether flex items are forced onto one line or can wrap onto multiple
  /// lines
  #[declare(default)]
  pub wrap: bool,
  /// Sets how flex items are placed in the flex container defining the main
  /// axis and the direction
  #[declare(default)]
  pub direction: Direction,
  /// How the children should be placed along the cross axis in a flex layout.
  #[declare(default)]
  pub align_items: Align,
  /// How the children should be placed along the main axis in a flex layout.
  #[declare(default)]
  pub justify_content: JustifyContent,
}

impl Render for Flex {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    if Align::Stretch == self.align_items && self.wrap {
      warn!("stretch align and wrap property is conflict");
    }
    let direction = self.direction;
    let max_size = FlexSize::from_size(clamp.max, direction);
    let mut min_size = FlexSize::from_size(clamp.min, direction);
    if Align::Stretch == self.align_items {
      min_size.cross = max_size.cross;
    }
    let mut layouter = FlexLayouter {
      max_size,
      min_size,
      direction,
      reverse: self.reverse,
      wrap: self.wrap,
      main_max: 0.,
      current_line: <_>::default(),
      lines_info: vec![],
      align_items: self.align_items,
      justify_content: self.justify_content,
    };
    layouter.layout(ctx)
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}
}

impl Query for Flex {
  impl_query_self_only!();
}

#[derive(Debug, Clone, Copy, Default)]
struct FlexSize {
  main: f32,
  cross: f32,
}

impl FlexSize {
  fn to_size(self, dir: Direction) -> Size {
    match dir {
      Direction::Horizontal => Size::new(self.main, self.cross),
      Direction::Vertical => Size::new(self.cross, self.main),
    }
  }

  fn from_size(size: Size, dir: Direction) -> Self {
    match dir {
      Direction::Horizontal => Self { main: size.width, cross: size.height },
      Direction::Vertical => Self { cross: size.width, main: size.height },
    }
  }

  fn to_point(self, dir: Direction) -> Point { self.to_size(dir).to_vector().to_point() }

  fn clamp(self, min: FlexSize, max: FlexSize) -> FlexSize {
    FlexSize {
      main: self.main.min(max.main).max(min.main),
      cross: self.cross.min(max.cross).max(min.cross),
    }
  }
}

impl std::ops::Sub for FlexSize {
  type Output = Self;
  fn sub(self, rhs: Self) -> Self::Output {
    FlexSize {
      main: self.main - rhs.main,
      cross: self.cross - rhs.cross,
    }
  }
}

struct FlexLayouter {
  max_size: FlexSize,
  min_size: FlexSize,
  reverse: bool,
  direction: Direction,
  /// the max of child touch in main axis
  main_max: f32,
  wrap: bool,
  current_line: MainLineInfo,
  lines_info: Vec<MainLineInfo>,
  align_items: Align,
  justify_content: JustifyContent,
}

impl FlexLayouter {
  fn layout(&mut self, ctx: &mut LayoutCtx) -> Size {
    macro_rules! inner_layout {
      ($method: ident) => {{
        let (ctx, iter) = ctx.$method();
        self.children_perform(ctx, iter);
        let (ctx, iter) = ctx.$method();
        self.layout_flex_children(ctx, iter);
        let size = self.box_size();
        let (ctx, iter) = ctx.$method();
        self.line_inner_align(ctx, iter, size);
        size.to_size(self.direction)
      }};
    }
    if self.reverse {
      inner_layout!(split_rev_children)
    } else {
      inner_layout!(split_children)
    }
  }

  fn children_perform<'a>(
    &mut self,
    ctx: &mut LayoutCtx,
    children: impl Iterator<Item = WidgetId>,
  ) {
    children.for_each(|child| {
      self.place_widget(child, ctx);
    });
    self.place_line();
  }

  fn layout_flex_children<'a>(
    &mut self,
    ctx: &mut LayoutCtx,
    mut children: impl Iterator<Item = WidgetId>,
  ) {
    let Self {
      lines_info,
      max_size,
      direction,
      main_max,
      ..
    } = self;
    let mut line_cross = 0.;
    lines_info.iter_mut().for_each(|line| {
      line.cross_pos = line_cross;
      if line.flex_sum == 0. {
        children.advance_by(line.child_count).unwrap();
      } else {
        let flex_unit = (max_size.main - line.main_width) / line.flex_sum;
        (0..line.child_count)
          .map(|_| children.next().unwrap())
          .for_each(|wid| {
            if let Some(flex) = Self::child_flex(ctx, wid) {
              Self::layout_flex_child(
                ctx,
                wid,
                flex,
                flex_unit,
                *max_size,
                FlexSize { main: 0., cross: 0. },
                *direction,
                line,
              );
            }
          });
      }
      line_cross += line.cross_line_height;
      *main_max = main_max.max(line.main_width);
    });
  }

  fn line_inner_align<'a>(
    &mut self,
    ctx: &mut LayoutCtx,
    mut children: impl Iterator<Item = WidgetId>,
    size: FlexSize,
  ) {
    let real_size = self.best_size();
    let Self {
      lines_info,
      justify_content: main_align,
      direction,
      align_items: cross_align,
      ..
    } = self;
    let container_cross_offset = cross_align.align_value(real_size.cross, size.cross);
    lines_info.iter_mut().for_each(|line| {
      let (offset, step) = match main_align {
        JustifyContent::Start => (0., 0.),
        JustifyContent::Center => ((size.main - line.main_width) / 2., 0.),
        JustifyContent::End => (size.main - line.main_width, 0.),
        JustifyContent::SpaceAround => {
          let step = (size.main - line.main_width) / line.child_count as f32;
          (step / 2., step)
        }
        JustifyContent::SpaceBetween => {
          let step = (size.main - line.main_width) / (line.child_count - 1) as f32;
          (0., step)
        }
        JustifyContent::SpaceEvenly => {
          let step = (size.main - line.main_width) / (line.child_count + 1) as f32;
          (step, step)
        }
      };

      let cross_pos = line.cross_pos;
      (0..line.child_count)
        .map(|_| children.next().unwrap())
        .fold(offset, |main_offset: f32, child| {
          let rect = ctx
            .widget_box_rect(child)
            .expect("relayout a expanded widget which not prepare layout");
          let mut origin = FlexSize::default();
          let child_size = FlexSize::from_size(rect.size, *direction);
          let line_cross_offset = cross_align.align_value(child_size.cross, line.cross_line_height);
          origin.main += main_offset;
          origin.cross += container_cross_offset + line_cross_offset + cross_pos;
          ctx.update_position(child, origin.to_point(*direction));
          main_offset + step + child_size.main
        });
    });
  }

  fn place_widget(&mut self, child: WidgetId, ctx: &mut LayoutCtx) {
    let mut flex_size = FlexSize { main: 0., cross: 0. };

    let mut min_size = self.min_size;
    min_size.main = 0.;
    let clamp = BoxClamp {
      max: self.max_size.to_size(self.direction),
      min: min_size.to_size(self.direction),
    };

    if let Some(flex) = Self::child_flex(ctx, child) {
      self.current_line.flex_sum += flex;
    } else {
      let size = ctx.perform_child_layout(child, clamp);
      flex_size = FlexSize::from_size(size, self.direction);
    }
    if self.wrap
      && !self.current_line.is_empty()
      && self.current_line.main_width + flex_size.main > self.max_size.main
    {
      self.place_line();
    }
    let mut line = &mut self.current_line;
    line.child_count += 1;
    line.main_width += flex_size.main;
    line.cross_line_height = line.cross_line_height.max(flex_size.cross);
  }

  fn place_line(&mut self) {
    if !self.current_line.is_empty() {
      self.main_max = self.main_max.max(self.current_line.main_width);
      let new_line = MainLineInfo {
        cross_pos: self.current_line.cross_bottom(),
        ..Default::default()
      };
      self
        .lines_info
        .push(std::mem::replace(&mut self.current_line, new_line));
    }
  }

  fn layout_flex_child(
    ctx: &mut LayoutCtx,
    child: WidgetId,
    flex: f32,
    flex_unit: f32,
    mut max_size: FlexSize,
    mut min_size: FlexSize,
    dir: Direction,
    line: &mut MainLineInfo,
  ) {
    let max_main = flex_unit * flex;
    max_size.main = max_size.main.min(max_main);
    min_size.main = 0.;
    let clamp = BoxClamp {
      max: max_size.to_size(dir),
      min: min_size.to_size(dir),
    };
    let new_size = ctx.perform_child_layout(child, clamp);
    let flex_size = FlexSize::from_size(new_size, dir);
    line.main_width += flex_size.main;
    line.cross_line_height = line.cross_line_height.max(flex_size.cross);
  }

  fn best_size(&self) -> FlexSize {
    let cross = self
      .lines_info
      .last()
      .map(|line| line.cross_bottom())
      .unwrap_or(0.);
    FlexSize { cross, main: self.main_max }
  }

  fn box_size(&self) -> FlexSize { self.best_size().clamp(self.min_size, self.max_size) }

  fn child_flex(ctx: &mut LayoutCtx, child: WidgetId) -> Option<f32> {
    let mut flex = None;
    ctx.query_widget_type(child, |expanded: &Expanded| flex = Some(expanded.flex));
    flex
  }
}

#[derive(Default)]
struct MainLineInfo {
  child_count: usize,
  cross_pos: f32,
  main_width: f32,
  flex_sum: f32,
  cross_line_height: f32,
}

impl MainLineInfo {
  fn is_empty(&self) -> bool { self.child_count == 0 }

  fn cross_bottom(&self) -> f32 { self.cross_pos + self.cross_line_height }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::prelude::*;
  use ribir_core::test::*;

  #[test]
  fn horizontal_line() {
    let row = widget! {
      Flex {
        ExprWidget {
          expr: (0..10).map(|_| SizedBox { size: Size::new(10., 20.) })
        }
      }
    };
    expect_layout_result_with_theme(
      row,
      Some(Size::new(500., 500.)),
      material::purple::light(),
      &[LayoutTestItem {
        path: &[0],
        expect: ExpectRect {
          x: Some(0.),
          y: Some(0.),
          width: Some(100.),
          height: Some(20.),
        },
      }],
    );
  }

  #[test]
  fn vertical_line() {
    let col = widget! {
      Flex {
        direction: Direction::Vertical,
        ExprWidget  {
         expr: (0..10).map(|_| SizedBox { size: Size::new(10., 20.) })
        }
      }
    };
    expect_layout_result_with_theme(
      col,
      Some(Size::new(500., 500.)),
      material::purple::light(),
      &[LayoutTestItem {
        path: &[0],
        expect: ExpectRect {
          x: Some(0.),
          y: Some(0.),
          width: Some(10.),
          height: Some(200.),
        },
      }],
    );
  }

  #[test]
  fn row_wrap() {
    let size = Size::new(200., 20.);
    let row = widget! {
      Flex {
        wrap: true,
        ExprWidget {
          expr: (0..3).map(|_| SizedBox { size })
        }
      }
    };

    let layouts = [
      LayoutTestItem {
        path: &[0],
        expect: ExpectRect {
          x: Some(0.),
          y: Some(0.),
          width: Some(400.),
          height: Some(40.),
        },
      },
      LayoutTestItem {
        path: &[0, 0],
        expect: ExpectRect {
          x: Some(0.),
          y: Some(0.),
          width: Some(200.),
          height: Some(20.),
        },
      },
      LayoutTestItem {
        path: &[0, 1],
        expect: ExpectRect {
          x: Some(200.),
          y: Some(0.),
          width: Some(200.),
          height: Some(20.),
        },
      },
      LayoutTestItem {
        path: &[0, 2],
        expect: ExpectRect {
          x: Some(0.),
          y: Some(20.),
          width: Some(200.),
          height: Some(20.),
        },
      },
    ];
    expect_layout_result_with_theme(
      row,
      Some(Size::new(500., 500.)),
      material::purple::light(),
      &layouts,
    );
  }

  #[test]
  fn reverse_row_wrap() {
    let size = Size::new(200., 20.);
    let row = widget! {
      Flex {
        wrap: true,
        reverse: true,
        ExprWidget {
          expr: (0..3).map(|_| SizedBox { size })
        }
      }
    };

    let layouts = [
      LayoutTestItem {
        path: &[0],
        expect: ExpectRect {
          x: Some(0.),
          y: Some(0.),
          width: Some(400.),
          height: Some(40.),
        },
      },
      LayoutTestItem {
        path: &[0, 0],
        expect: ExpectRect {
          x: Some(0.),
          y: Some(20.),
          width: Some(200.),
          height: Some(20.),
        },
      },
      LayoutTestItem {
        path: &[0, 1],
        expect: ExpectRect {
          x: Some(200.),
          y: Some(0.),
          width: Some(200.),
          height: Some(20.),
        },
      },
      LayoutTestItem {
        path: &[0, 2],
        expect: ExpectRect {
          x: Some(0.),
          y: Some(0.),
          width: Some(200.),
          height: Some(20.),
        },
      },
    ];
    expect_layout_result_with_theme(
      row,
      Some(Size::new(500., 500.)),
      material::purple::light(),
      &layouts,
    );
  }

  #[test]
  fn cross_align() {
    fn cross_align_check(align: Align, y_pos: [f32; 3]) {
      let row = widget! {
        Row {
          align_items: align,
          SizedBox { size: Size::new(100., 20.) }
          SizedBox { size: Size::new(100., 30.) }
          SizedBox { size: Size::new(100., 40.) }
        }
      };

      let layouts = [
        LayoutTestItem {
          path: &[0],
          expect: ExpectRect {
            x: Some(0.),
            y: Some(0.),
            width: Some(300.),
            height: Some(40.),
          },
        },
        LayoutTestItem {
          path: &[0, 0],
          expect: ExpectRect {
            x: Some(0.),
            y: Some(y_pos[0]),
            width: Some(100.),
            height: Some(20.),
          },
        },
        LayoutTestItem {
          path: &[0, 1],
          expect: ExpectRect {
            x: Some(100.),
            y: Some(y_pos[1]),
            width: Some(100.),
            height: Some(30.),
          },
        },
        LayoutTestItem {
          path: &[0, 2],
          expect: ExpectRect {
            x: Some(200.),
            y: Some(y_pos[2]),
            width: Some(100.),
            height: Some(40.),
          },
        },
      ];
      expect_layout_result_with_theme(
        row,
        Some(Size::new(500., 500.)),
        material::purple::light(),
        &layouts,
      );
    }
    cross_align_check(Align::Start, [0., 0., 0.]);
    cross_align_check(Align::Center, [10., 5., 0.]);
    cross_align_check(Align::End, [20., 10., 0.]);

    let row = widget! {
      Row {
        align_items: Align::Stretch,
        SizedBox { size: Size::new(100., 20.) }
        SizedBox { size: Size::new(100., 30.) }
        SizedBox { size: Size::new(100., 40.) }
      }
    };

    let layouts = [
      LayoutTestItem {
        path: &[0],
        expect: ExpectRect {
          x: Some(0.),
          y: Some(0.),
          width: Some(300.),
          height: Some(40.),
        },
      },
      LayoutTestItem {
        path: &[0, 0],
        expect: ExpectRect {
          x: Some(0.),
          y: Some(0.),
          width: Some(100.),
          height: Some(40.),
        },
      },
      LayoutTestItem {
        path: &[0, 1],
        expect: ExpectRect {
          x: Some(100.),
          y: Some(0.),
          width: Some(100.),
          height: Some(40.),
        },
      },
      LayoutTestItem {
        path: &[0, 2],
        expect: ExpectRect {
          x: Some(200.),
          y: Some(0.),
          width: Some(100.),
          height: Some(40.),
        },
      },
    ];
    expect_layout_result_with_theme(
      row,
      Some(Size::new(500., 40.)),
      material::purple::light(),
      &layouts,
    );
  }

  #[test]
  fn main_align() {
    fn main_align_check(justify_content: JustifyContent, pos: [(f32, f32); 3]) {
      let item_size = Size::new(100., 20.);
      let root = widget! {
        SizedBox {
          size: INFINITY_SIZE,
          Row {
            justify_content,
            align_items: Align::Start,
            SizedBox { size: item_size }
            SizedBox { size: item_size }
            SizedBox { size: item_size }
          }
        }
      };

      expect_layout_result(
        root,
        Some(Size::new(500., 500.)),
        &[
          LayoutTestItem {
            path: &[0, 0],
            expect: ExpectRect {
              width: Some(500.),
              height: Some(500.),
              ..<_>::default()
            },
          },
          LayoutTestItem {
            path: &[0, 0, 0],
            expect: ExpectRect {
              x: Some(pos[0].0),
              y: Some(pos[0].1),
              ..<_>::default()
            },
          },
          LayoutTestItem {
            path: &[0, 0, 1],
            expect: ExpectRect {
              x: Some(pos[1].0),
              y: Some(pos[1].1),
              ..<_>::default()
            },
          },
          LayoutTestItem {
            path: &[0, 0, 2],
            expect: ExpectRect {
              x: Some(pos[2].0),
              y: Some(pos[2].1),
              ..<_>::default()
            },
          },
        ],
      );
    }

    main_align_check(JustifyContent::Start, [(0., 0.), (100., 0.), (200., 0.)]);
    main_align_check(JustifyContent::Center, [(100., 0.), (200., 0.), (300., 0.)]);
    main_align_check(JustifyContent::End, [(200., 0.), (300., 0.), (400., 0.)]);
    main_align_check(
      JustifyContent::SpaceBetween,
      [(0., 0.), (200., 0.), (400., 0.)],
    );
    let space = 200.0 / 3.0;
    main_align_check(
      JustifyContent::SpaceAround,
      [
        (0.5 * space, 0.),
        (100. + space * 1.5, 0.),
        (2.5 * space + 200., 0.),
      ],
    );
    main_align_check(
      JustifyContent::SpaceEvenly,
      [(50., 0.), (200., 0.), (350., 0.)],
    );
  }

  #[test]
  fn flex_expand() {
    let row = widget! {
      SizedBox {
        size: Size::new(500., 25.),
        Flex {
          direction: Direction::Horizontal,
          Expanded {
            flex: 1.,
            SizedBox {
              size: INFINITY_SIZE,
            }
          }
          SizedBox { size: Size::new(100., 20.) }
          Expanded {
            flex: 3.,
            SizedBox {
              size: INFINITY_SIZE,
            }
          }
        }
      }

    };
    expect_layout_result_with_theme(
      row,
      Some(Size::new(500., 500.)),
      material::purple::light(),
      &[
        LayoutTestItem {
          path: &[0, 0],
          expect: ExpectRect {
            x: Some(0.),
            y: Some(0.),
            width: Some(500.),
            height: Some(25.),
          },
        },
        LayoutTestItem {
          path: &[0, 0, 0],
          expect: ExpectRect {
            x: Some(0.),
            y: Some(0.),
            width: Some(100.),
            height: Some(25.),
          },
        },
        LayoutTestItem {
          path: &[0, 0, 2],
          expect: ExpectRect {
            x: Some(200.),
            y: Some(0.),
            width: Some(300.),
            height: Some(25.),
          },
        },
      ],
    );
  }
}
