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
      max: max_size,
      min: min_size,
      reverse: self.reverse,
      dir: direction,
      align_items: self.align_items,
      justify_content: self.justify_content,
      wrap: self.wrap,
      current_line: <_>::default(),
      lines: vec![],
    };
    layouter.layout(ctx)
  }

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
  max: FlexSize,
  min: FlexSize,
  reverse: bool,
  dir: Direction,
  align_items: Align,
  justify_content: JustifyContent,
  wrap: bool,
  current_line: MainLineInfo,
  lines: Vec<MainLineInfo>,
}

impl FlexLayouter {
  fn layout(&mut self, ctx: &mut LayoutCtx) -> Size {
    self.perform_children_layout(ctx);
    self.flex_children_layout(ctx);

    let cross = self
      .lines
      .iter()
      .fold(0., |sum, l| sum + l.cross_line_height);
    let main = match self.justify_content {
      JustifyContent::Start | JustifyContent::Center | JustifyContent::End => {
        self.lines.iter().fold(0f32, |max, l| max.max(l.main_width))
      }
      JustifyContent::SpaceBetween | JustifyContent::SpaceAround | JustifyContent::SpaceEvenly => {
        self.max.main
      }
    };
    let size = FlexSize { cross, main };
    let &mut Self { max, min, dir, .. } = self;
    let size = size.to_size(dir).clamp(min.to_size(dir), max.to_size(dir));
    self.update_children_position(FlexSize::from_size(size, dir), ctx);
    size
  }

  fn perform_children_layout(&mut self, ctx: &mut LayoutCtx) {
    // All children perform layout.
    let mut layouter = ctx.first_child_layouter();
    let &mut Self { max, min, wrap, dir, .. } = self;
    let min = FlexSize { main: 0., cross: min.cross };
    while let Some(mut l) = layouter {
      let mut max = max;
      if !wrap {
        max.main -= self.current_line.main_width;
      }

      let clamp = BoxClamp {
        max: max.to_size(dir),
        min: min.to_size(dir),
      };

      let size = l.perform_widget_layout(clamp);
      let size = FlexSize::from_size(size, dir);
      let mut flex = None;
      l.query_widget_type(|expanded: &Expanded| flex = Some(expanded.flex));

      // flex-item need use empty space  to resize after all fixed widget performed
      // layout.
      let line = &mut self.current_line;
      if let Some(flex) = flex {
        line.flex_sum += flex;
      } else {
        if wrap && !line.is_empty() && line.main_width + size.main > max.main {
          self.place_line();
        }

        let mut line = &mut self.current_line;
        line.main_width += size.main;
        line.cross_line_height = line.cross_line_height.max(size.cross);
      }
      self
        .current_line
        .items_info
        .push(FlexLayoutInfo { size, flex, pos: <_>::default() });
      layouter = l.into_next_sibling();
    }
    self.place_line();
  }

  fn flex_children_layout(&mut self, ctx: &mut LayoutCtx) {
    let mut layouter = ctx.first_child_layouter();
    self.lines.iter_mut().for_each(|line| {
      let flex_unit = (self.max.main - line.main_width) / line.flex_sum;
      line.items_info.iter_mut().for_each(|info| {
        let mut l = layouter.take().unwrap();
        if let Some(flex) = info.flex {
          let &mut Self { mut max, mut min, dir, .. } = self;
          let main = flex_unit * flex;
          max.main = main;
          min.main = main;
          let clamp = BoxClamp {
            max: max.to_size(dir),
            min: min.to_size(dir),
          };
          let size = l.perform_widget_layout(clamp);
          info.size = FlexSize::from_size(size, dir);
          line.main_width += info.size.main;
          line.cross_line_height = line.cross_line_height.max(info.size.cross);
        }

        layouter = l.into_next_sibling();
      });
    });
  }

  fn update_children_position(&mut self, bound: FlexSize, ctx: &mut LayoutCtx) {
    let Self {
      reverse,
      dir,
      align_items,
      justify_content,
      lines,
      ..
    } = self;

    let cross_size = lines.iter().map(|l| l.cross_line_height).sum();
    let cross_offset = align_items.align_value(cross_size, bound.cross);

    macro_rules! update_position {
      ($($rev: ident)?) => {
        let mut cross = cross_offset;
        lines.iter_mut()$(.$rev())?.for_each(|line| {
          let (mut main, step) = line.place_args(bound.main, *justify_content);
          line.items_info.iter_mut()$(.$rev())?.for_each(|item| {
            let item_cross_offset =
              align_items.align_value(item.size.cross, line.cross_line_height);
            item.pos.cross = cross + item_cross_offset;
            item.pos.main = main;
            main = main + item.size.main + step;
          });
          cross += line.cross_line_height;
        });
      };
    }
    if *reverse {
      update_position!(rev);
    } else {
      update_position!();
    }

    let mut layouter = ctx.first_child_layouter();
    self.lines.iter_mut().for_each(|line| {
      line.items_info.iter_mut().for_each(|info| {
        let mut l = layouter.take().unwrap();
        l.update_position(info.pos.to_size(*dir).to_vector().to_point());
        layouter = l.into_next_sibling();
      })
    });
  }

  fn place_line(&mut self) {
    if !self.current_line.is_empty() {
      self.lines.push(std::mem::take(&mut self.current_line));
    }
  }
}

#[derive(Default)]
struct MainLineInfo {
  main_width: f32,
  items_info: Vec<FlexLayoutInfo>,
  flex_sum: f32,
  cross_line_height: f32,
}

struct FlexLayoutInfo {
  pos: FlexSize,
  size: FlexSize,
  flex: Option<f32>,
}

impl MainLineInfo {
  fn is_empty(&self) -> bool { self.items_info.is_empty() }

  fn place_args(&self, main_max: f32, justify_content: JustifyContent) -> (f32, f32) {
    if self.items_info.is_empty() {
      return (0., 0.);
    }

    let item_cnt = self.items_info.len() as f32;
    match justify_content {
      JustifyContent::Start => (0., 0.),
      JustifyContent::Center => ((main_max - self.main_width) / 2., 0.),
      JustifyContent::End => (main_max - self.main_width, 0.),
      JustifyContent::SpaceAround => {
        let step = (main_max - self.main_width) / item_cnt;
        (step / 2., step)
      }
      JustifyContent::SpaceBetween => {
        let step = (main_max - self.main_width) / (item_cnt - 1.);
        (0., step)
      }
      JustifyContent::SpaceEvenly => {
        let step = (main_max - self.main_width) / (item_cnt + 1.);
        (step, step)
      }
    }
  }
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
        DynWidget {
          dyns: (0..10).map(|_| SizedBox { size: Size::new(10., 20.) })
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
        DynWidget  {
         dyns: (0..10).map(|_| SizedBox { size: Size::new(10., 20.) })
        }
      }
    };
    expect_layout_result_with_theme(
      col,
      Some(Size::new(500., 500.)),
      material::purple::light(),
      &[LayoutTestItem {
        path: &[0],
        expect: ExpectRect::new(0., 0., 10., 200.),
      }],
    );
  }

  #[test]
  fn row_wrap() {
    let size = Size::new(200., 20.);
    let row = widget! {
      Flex {
        wrap: true,
        DynWidget {
          dyns: (0..3).map(|_| SizedBox { size })
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

    expect_layout_result_with_theme(
      widget! {
        Flex {
          wrap: true,
          reverse: true,
          DynWidget {
            dyns: (0..3).map(|_| SizedBox { size })
          }
        }
      },
      Some(Size::new(500., 500.)),
      material::purple::light(),
      &[
        LayoutTestItem {
          path: &[0],
          expect: ExpectRect::from_size(Size::new(400., 40.)),
        },
        LayoutTestItem {
          path: &[0, 0],
          expect: ExpectRect::new(200., 20., 200., 20.),
        },
        LayoutTestItem {
          path: &[0, 1],
          expect: ExpectRect::new(0., 20., 200., 20.),
        },
        LayoutTestItem {
          path: &[0, 2],
          expect: ExpectRect::new(0., 0., 200., 20.),
        },
      ],
    );
  }

  #[test]
  fn cross_align() {
    fn cross_align_check(align: Align, y_pos: [f32; 3]) {
      expect_layout_result_with_theme(
        widget! {
          Row {
            align_items: align,
            SizedBox { size: Size::new(100., 20.) }
            SizedBox { size: Size::new(100., 30.) }
            SizedBox { size: Size::new(100., 40.) }
          }
        },
        Some(Size::new(500., 40.)),
        material::purple::light(),
        &[
          LayoutTestItem {
            path: &[0],
            expect: ExpectRect::from_size(Size::new(300., 40.)),
          },
          LayoutTestItem {
            path: &[0, 0],
            expect: ExpectRect::new(0., y_pos[0], 100., 20.),
          },
          LayoutTestItem {
            path: &[0, 1],
            expect: ExpectRect::new(100., y_pos[1], 100., 30.),
          },
          LayoutTestItem {
            path: &[0, 2],
            expect: ExpectRect::new(200., y_pos[2], 100., 40.),
          },
        ],
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

    expect_layout_result_with_theme(
      row,
      Some(Size::new(500., 40.)),
      material::purple::light(),
      &[
        LayoutTestItem {
          path: &[0],
          expect: ExpectRect::from_size(Size::new(300., 40.)),
        },
        LayoutTestItem {
          path: &[0, 0],
          expect: ExpectRect::new(0., 0., 100., 40.),
        },
        LayoutTestItem {
          path: &[0, 1],
          expect: ExpectRect::new(100., 0., 100., 40.),
        },
        LayoutTestItem {
          path: &[0, 2],
          expect: ExpectRect::new(200., 0., 100., 40.),
        },
      ],
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
            expect: ExpectRect::from_size(Size::new(500., 500.)),
          },
          LayoutTestItem {
            path: &[0, 0, 0],
            expect: ExpectRect::from_point(pos[0].into()),
          },
          LayoutTestItem {
            path: &[0, 0, 1],
            expect: ExpectRect::from_point(pos[1].into()),
          },
          LayoutTestItem {
            path: &[0, 0, 2],
            expect: ExpectRect::from_point(pos[2].into()),
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
            SizedBox { size: INFINITY_SIZE,}
          }
          SizedBox { size: Size::new(100., 20.) }
          Expanded {
            flex: 3.,
            SizedBox { size: INFINITY_SIZE, }
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
          expect: ExpectRect::new(0., 0., 500., 25.),
        },
        LayoutTestItem {
          path: &[0, 0, 0],
          expect: ExpectRect::new(0., 0., 100., 25.),
        },
        LayoutTestItem {
          path: &[0, 0, 2],
          expect: ExpectRect::new(200., 0., 300., 25.),
        },
      ],
    );
  }
}
