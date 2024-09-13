use ribir_core::prelude::{log::warn, *};

use super::{Direction, Expanded};

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
  /// Define item between gap in main axis
  #[declare(default)]
  pub item_gap: f32,
  /// Define item between gap in cross axis
  #[declare(default)]
  pub line_gap: f32,
}

/// A type help to declare flex widget as horizontal.
pub struct Row;

/// A type help to declare flex widget as Vertical.
pub struct Column;

impl Declare for Row {
  type Builder = FlexDeclarer;
  fn declarer() -> Self::Builder { Flex::declarer().direction(Direction::Horizontal) }
}

impl Declare for Column {
  type Builder = FlexDeclarer;
  fn declarer() -> Self::Builder { Flex::declarer().direction(Direction::Vertical) }
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
      main_axis_gap: self.item_gap,
      cross_axis_gap: self.line_gap,
      current_line: <_>::default(),
      lines: vec![],
    };
    layouter.layout(ctx)
  }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}
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

  fn zero() -> Self { Self { main: 0., cross: 0. } }
}

impl std::ops::Sub for FlexSize {
  type Output = Self;
  fn sub(self, rhs: Self) -> Self::Output {
    FlexSize { main: self.main - rhs.main, cross: self.cross - rhs.cross }
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
  main_axis_gap: f32,
  cross_axis_gap: f32,
}

impl FlexLayouter {
  fn layout(&mut self, ctx: &mut LayoutCtx) -> Size {
    self.perform_children_layout(ctx);
    self.flex_children_layout(ctx);

    // cross direction need calculate cross_axis_gap but last line don't need.
    let cross = self
      .lines
      .iter()
      .fold(-self.cross_axis_gap, |sum, l| sum + l.cross_line_height + self.cross_axis_gap);
    let main = match self.justify_content {
      JustifyContent::Start | JustifyContent::Center | JustifyContent::End => self
        .lines
        .iter()
        .fold(0f32, |max, l| max.max(l.main_width)),
      JustifyContent::SpaceBetween | JustifyContent::SpaceAround | JustifyContent::SpaceEvenly => {
        self.max.main
      }
    };
    let size = FlexSize { cross, main };
    let &mut Self { max, min, dir, .. } = self;
    let size = size
      .to_size(dir)
      .clamp(min.to_size(dir), max.to_size(dir));
    self.update_children_position(FlexSize::from_size(size, dir), ctx);
    size
  }

  fn perform_children_layout(&mut self, ctx: &mut LayoutCtx) {
    let (ctx, children) = ctx.split_children();
    let &mut Self { max, min, wrap, dir, .. } = self;
    let min = if self.align_items == Align::Stretch {
      FlexSize { main: 0., cross: min.cross }
    } else {
      FlexSize::zero()
    };
    let mut children = children.peekable();
    while let Some(c) = children.next() {
      let mut max = max;
      if !wrap {
        max.main -= self.current_line.main_width;
      }

      let clamp = BoxClamp { max: max.to_size(dir), min: min.to_size(dir) };

      let mut info = FlexLayoutInfo {
        flex: ctx
          .query_of_widget::<Expanded>(c)
          .map(|expanded| expanded.flex),
        pos: <_>::default(),
        size: <_>::default(),
      };

      let gap = if children.peek().is_some() && !FlexLayouter::is_space_layout(self.justify_content)
      {
        self.main_axis_gap
      } else {
        0.
      };

      // flex-item need use empty space to resize after all fixed widget performed
      // layout.
      let line = &mut self.current_line;
      if let Some(flex) = info.flex {
        line.flex_sum += flex;
        line.main_width += gap;
      } else {
        let size = ctx.perform_child_layout(c, clamp);
        let size = FlexSize::from_size(size, dir);
        info.size = size;

        if wrap && !line.is_empty() && line.main_width + size.main > max.main {
          self.place_line();
        } else {
          line.main_width += gap;
        }

        let line = &mut self.current_line;
        line.main_width += size.main;
        line.cross_line_height = line.cross_line_height.max(size.cross);
      }

      self.current_line.items_info.push(info);
    }

    self.place_line();
  }

  fn is_space_layout(justify_content: JustifyContent) -> bool {
    matches!(
      justify_content,
      JustifyContent::SpaceAround | JustifyContent::SpaceBetween | JustifyContent::SpaceEvenly
    )
  }

  fn flex_children_layout(&mut self, ctx: &mut LayoutCtx) {
    let (ctx, mut children) = ctx.split_children();
    self.lines.iter_mut().for_each(|line| {
      let flex_sum = if line.flex_sum.is_normal() { line.flex_sum } else { 1. };
      let flex_unit = (self.max.main - line.main_width) / flex_sum;
      line.items_info.iter_mut().for_each(|info| {
        let child = children.next().unwrap();
        if let Some(flex) = info.flex {
          let &mut Self { mut max, mut min, dir, .. } = self;
          // If the maximum size is not specified, we are unable to calculate the flex
          // size.
          if flex_unit.is_normal() {
            let main = flex_unit * flex;
            max.main = main;
            min.main = main;
          };
          let clamp = BoxClamp { max: max.to_size(dir), min: min.to_size(dir) };
          let size = ctx.perform_child_layout(child, clamp);
          info.size = FlexSize::from_size(size, dir);
          line.main_width += info.size.main;
          line.cross_line_height = line.cross_line_height.max(info.size.cross);
        }
      });
    });
  }

  fn update_children_position(&mut self, bound: FlexSize, ctx: &mut LayoutCtx) {
    let Self { reverse, dir, align_items, justify_content, lines, .. } = self;

    let cross_size = lines.iter().map(|l| l.cross_line_height).sum();
    // cross gap don't use calc offset
    let cross_gap_count =
      if !lines.is_empty() { (lines.len() - 1) as f32 * self.cross_axis_gap } else { 0. };
    let cross_offset = align_items.align_value(cross_size, bound.cross - cross_gap_count);

    macro_rules! update_position {
      ($($rev: ident)?) => {
        let mut cross = cross_offset - self.cross_axis_gap;
        lines.iter_mut()$(.$rev())?.for_each(|line| {
          let (mut main, step) = line.place_args(bound.main, *justify_content, self.main_axis_gap);
          line.items_info.iter_mut()$(.$rev())?.for_each(|item| {
            let item_cross_offset =
              align_items.align_value(item.size.cross, line.cross_line_height);

            item.pos.cross = cross + item_cross_offset + self.cross_axis_gap;
            item.pos.main = main;
            main = main + item.size.main + step;
          });
          cross += line.cross_line_height + self.cross_axis_gap;
        });
      };
    }
    if *reverse {
      update_position!(rev);
    } else {
      update_position!();
    }

    let (ctx, mut children) = ctx.split_children();

    self.lines.iter_mut().for_each(|line| {
      line.items_info.iter_mut().for_each(|info| {
        let child = children.next().unwrap();
        ctx.update_position(child, info.pos.to_size(*dir).to_vector().to_point());
      })
    });

    for child in children {
      self.lines.iter_mut().for_each(|line| {
        line.items_info.iter_mut().for_each(|info| {
          ctx.update_position(child, info.pos.to_size(*dir).to_vector().to_point());
        })
      });
    }
  }

  fn place_line(&mut self) {
    if !self.current_line.is_empty() {
      self
        .lines
        .push(std::mem::take(&mut self.current_line));
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

  fn place_args(&self, main_max: f32, justify_content: JustifyContent, gap: f32) -> (f32, f32) {
    if self.items_info.is_empty() {
      return (0., 0.);
    }

    let item_cnt = self.items_info.len() as f32;
    match justify_content {
      JustifyContent::Start => (0., gap),
      JustifyContent::Center => ((main_max - self.main_width) / 2., gap),
      JustifyContent::End => (main_max - self.main_width, gap),
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
  use ribir_core::test_helper::*;
  use ribir_dev_helper::*;

  use super::*;
  use crate::prelude::*;

  widget_layout_test!(
    horizontal_line,
    WidgetTester::new(fn_widget! {
      @Flex {
        @{
          (0..10).map(|_| SizedBox { size: Size::new(10., 20.) })
        }
      }
    }),
    LayoutCase::default().with_size(Size::new(100., 20.))
  );

  widget_layout_test!(
    vertical_line,
    WidgetTester::new(fn_widget! {
      @Flex {
        direction: Direction::Vertical,
        @{ (0..10).map(|_| SizedBox { size: Size::new(10., 20.) })}
      }
    }),
    LayoutCase::default().with_size(Size::new(10., 200.))
  );

  widget_layout_test!(
    row_wrap,
    WidgetTester::new(fn_widget! {
      @Flex {
        wrap: true,
        @{ (0..3).map(|_| SizedBox { size: Size::new(200., 20.) }) }
      }
    })
    .with_wnd_size(Size::new(500., 500.)),
    LayoutCase::default().with_size(Size::new(400., 40.)),
    LayoutCase::new(&[0, 0]).with_size(Size::new(200., 20.)),
    LayoutCase::new(&[0, 1])
      .with_size(Size::new(200., 20.))
      .with_x(200.),
    LayoutCase::new(&[0, 2]).with_rect(ribir_geom::rect(0., 20., 200., 20.))
  );

  widget_layout_test!(
    reverse_row_wrap,
    WidgetTester::new(fn_widget! {
      @Flex {
        wrap: true,
        reverse: true,
        @{ (0..3).map(|_| SizedBox { size: Size::new(200., 20.) }) }
      }
    })
    .with_wnd_size(Size::new(500., 500.)),
    LayoutCase::default().with_size(Size::new(400., 40.)),
    LayoutCase::new(&[0, 0]).with_rect(ribir_geom::rect(200., 20., 200., 20.)),
    LayoutCase::new(&[0, 1]).with_rect(ribir_geom::rect(0., 20., 200., 20.)),
    LayoutCase::new(&[0, 2]).with_rect(ribir_geom::rect(0., 0., 200., 20.))
  );

  widget_layout_test!(
    main_axis_gap,
    WidgetTester::new(fn_widget! {
      @Row {
        item_gap: 15.,
        @SizedBox { size: Size::new(120., 20.) }
        @SizedBox { size: Size::new(80., 20.) }
        @SizedBox { size: Size::new(30., 20.) }
      }
    })
    .with_wnd_size(Size::new(500., 500.)),
    LayoutCase::new(&[0, 0]).with_rect(ribir_geom::rect(0., 0., 120., 20.)),
    LayoutCase::new(&[0, 1]).with_rect(ribir_geom::rect(135., 0., 80., 20.)),
    LayoutCase::new(&[0, 2]).with_rect(ribir_geom::rect(230., 0., 30., 20.))
  );

  widget_layout_test!(
    main_axis_reverse_gap,
    WidgetTester::new(fn_widget! {
      @Row {
        item_gap: 15.,
        reverse: true,
        @SizedBox { size: Size::new(120., 20.) }
        @SizedBox { size: Size::new(80., 20.) }
        @SizedBox { size: Size::new(30., 20.) }
      }
    })
    .with_wnd_size(Size::new(500., 500.)),
    LayoutCase::new(&[0, 0]).with_rect(ribir_geom::rect(140., 0., 120., 20.)),
    LayoutCase::new(&[0, 1]).with_rect(ribir_geom::rect(45., 0., 80., 20.)),
    LayoutCase::new(&[0, 2]).with_rect(ribir_geom::rect(0., 0., 30., 20.))
  );

  widget_layout_test!(
    main_axis_expand,
    WidgetTester::new(fn_widget! {
      @Row {
        item_gap: 15.,
        @SizedBox { size: Size::new(120., 20.) }
        @Expanded {
          flex: 1.,
          @SizedBox { size: Size::new(10., 20.) }
        }
        @SizedBox { size: Size::new(80., 20.) }
        @Expanded {
          flex: 2.,
          @SizedBox { size: Size::new(10., 20.) }
        }
        @SizedBox { size: Size::new(30., 20.) }
      }
    })
    .with_wnd_size(Size::new(500., 500.)),
    LayoutCase::new(&[0, 0]).with_rect(ribir_geom::rect(0., 0., 120., 20.)),
    LayoutCase::new(&[0, 1]).with_rect(ribir_geom::rect(135., 0., 70., 20.)),
    LayoutCase::new(&[0, 2]).with_rect(ribir_geom::rect(220., 0., 80., 20.)),
    LayoutCase::new(&[0, 3]).with_rect(ribir_geom::rect(315., 0., 140., 20.)),
    LayoutCase::new(&[0, 4]).with_rect(ribir_geom::rect(470., 0., 30., 20.))
  );

  widget_layout_test!(
    cross_axis_gap,
    WidgetTester::new(fn_widget! {
      @Flex {
        wrap: true,
        line_gap: 10.,
        align_items: Align::Center,
        @{ (0..3).map(|_| SizedBox { size: Size::new(200., 20.) }) }
      }
    })
    .with_wnd_size(Size::new(500., 500.)),
    LayoutCase::default().with_rect(ribir_geom::rect(0., 0., 400., 50.)),
    LayoutCase::new(&[0, 0]).with_rect(ribir_geom::rect(0., 0., 200., 20.)),
    LayoutCase::new(&[0, 1]).with_rect(ribir_geom::rect(200., 0., 200., 20.)),
    LayoutCase::new(&[0, 2]).with_rect(ribir_geom::rect(0., 30., 200., 20.))
  );

  fn cross_align(align: Align) -> WidgetTester {
    WidgetTester::new(fn_widget! {
      @Row {
        align_items: align,
        @SizedBox { size: Size::new(100., 20.) }
        @SizedBox { size: Size::new(100., 30.) }
        @SizedBox { size: Size::new(100., 40.) }
      }
    })
    .with_wnd_size(Size::new(500., 40.))
  }

  widget_layout_test!(
    start_cross_align,
    cross_align(Align::Start),
    LayoutCase::default().with_size(Size::new(300., 40.)),
    LayoutCase::default().with_size(Size::new(300., 40.)),
    LayoutCase::new(&[0, 0]).with_rect(ribir_geom::rect(0., 0., 100., 20.)),
    LayoutCase::new(&[0, 1]).with_rect(ribir_geom::rect(100., 0., 100., 30.)),
    LayoutCase::new(&[0, 2]).with_rect(ribir_geom::rect(200., 0., 100., 40.))
  );

  widget_layout_test!(
    center_cross_align,
    cross_align(Align::Center),
    LayoutCase::default().with_size(Size::new(300., 40.)),
    LayoutCase::new(&[0, 0]).with_rect(ribir_geom::rect(0., 10., 100., 20.)),
    LayoutCase::new(&[0, 1]).with_rect(ribir_geom::rect(100., 5., 100., 30.)),
    LayoutCase::new(&[0, 2]).with_rect(ribir_geom::rect(200., 0., 100., 40.))
  );

  widget_layout_test!(
    end_cross_align,
    cross_align(Align::End),
    LayoutCase::default().with_size(Size::new(300., 40.)),
    LayoutCase::new(&[0, 0]).with_rect(ribir_geom::rect(0., 20., 100., 20.)),
    LayoutCase::new(&[0, 1]).with_rect(ribir_geom::rect(100., 10., 100., 30.)),
    LayoutCase::new(&[0, 2]).with_rect(ribir_geom::rect(200., 0., 100., 40.))
  );

  widget_layout_test!(
    stretch_cross_align,
    cross_align(Align::Stretch),
    LayoutCase::default().with_size(Size::new(300., 40.)),
    LayoutCase::new(&[0, 0]).with_rect(ribir_geom::rect(0., 0., 100., 40.)),
    LayoutCase::new(&[0, 1]).with_rect(ribir_geom::rect(100., 0., 100., 40.)),
    LayoutCase::new(&[0, 2]).with_rect(ribir_geom::rect(200., 0., 100., 40.))
  );

  fn main_align(justify_content: JustifyContent) -> WidgetTester {
    WidgetTester::new(fn_widget! {
      let item_size = Size::new(100., 20.);
      @SizedBox {
        size: Size::new(500., 500.),
        @Row {
          justify_content,
          align_items: Align::Start,
          @SizedBox { size: item_size }
          @SizedBox { size: item_size }
          @SizedBox { size: item_size }
        }
      }
    })
    .with_wnd_size(Size::new(500., 500.))
  }

  widget_layout_test!(
    start_main_align,
    main_align(JustifyContent::Start),
    LayoutCase::new(&[0, 0]).with_size(Size::new(500., 500.)),
    LayoutCase::new(&[0, 0, 0]).with_x(0.),
    LayoutCase::new(&[0, 0, 1]).with_x(100.),
    LayoutCase::new(&[0, 0, 2]).with_x(200.)
  );

  widget_layout_test!(
    center_main_align,
    main_align(JustifyContent::Center),
    LayoutCase::new(&[0, 0]).with_size(Size::new(500., 500.)),
    LayoutCase::new(&[0, 0, 0]).with_x(100.),
    LayoutCase::new(&[0, 0, 1]).with_x(200.),
    LayoutCase::new(&[0, 0, 2]).with_x(300.)
  );

  widget_layout_test!(
    end_main_align,
    main_align(JustifyContent::End),
    LayoutCase::new(&[0, 0]).with_size(Size::new(500., 500.)),
    LayoutCase::new(&[0, 0, 0]).with_x(200.),
    LayoutCase::new(&[0, 0, 1]).with_x(300.),
    LayoutCase::new(&[0, 0, 2]).with_x(400.)
  );

  widget_layout_test!(
    space_between_align,
    main_align(JustifyContent::SpaceBetween),
    LayoutCase::new(&[0, 0]).with_size(Size::new(500., 500.)),
    LayoutCase::new(&[0, 0, 0]).with_x(0.),
    LayoutCase::new(&[0, 0, 1]).with_x(200.),
    LayoutCase::new(&[0, 0, 2]).with_x(400.)
  );

  const AROUND_SPACE: f32 = 200.0 / 3.0;
  widget_layout_test!(
    space_around_align,
    main_align(JustifyContent::SpaceAround),
    LayoutCase::new(&[0, 0]).with_size(Size::new(500., 500.)),
    LayoutCase::new(&[0, 0, 0]).with_x(0.5 * AROUND_SPACE),
    LayoutCase::new(&[0, 0, 1]).with_x(100. + AROUND_SPACE * 1.5),
    LayoutCase::new(&[0, 0, 2]).with_x(2.5 * AROUND_SPACE + 200.)
  );

  widget_layout_test!(
    space_evenly_align,
    main_align(JustifyContent::SpaceEvenly),
    LayoutCase::new(&[0, 0]).with_size(Size::new(500., 500.)),
    LayoutCase::new(&[0, 0, 0]).with_x(50.),
    LayoutCase::new(&[0, 0, 1]).with_x(200.),
    LayoutCase::new(&[0, 0, 2]).with_x(350.)
  );

  widget_layout_test!(
    flex_expand,
    WidgetTester::new(fn_widget! {
      @SizedBox {
        size: Size::new(500., 25.),
        @Flex {
          direction: Direction::Horizontal,
          @Expanded {
            flex: 1.,
            @SizedBox { size: INFINITY_SIZE,}
          }
          @SizedBox { size: Size::new(100., 20.) }
          @Expanded {
            flex: 3.,
            @SizedBox { size: INFINITY_SIZE, }
          }
        }
      }
    })
    .with_wnd_size(Size::new(500., 500.)),
    LayoutCase::new(&[0, 0]).with_rect(ribir_geom::rect(0., 0., 500., 25.)),
    LayoutCase::new(&[0, 0, 0]).with_rect(ribir_geom::rect(0., 0., 100., 25.)),
    LayoutCase::new(&[0, 0, 2]).with_rect(ribir_geom::rect(200., 0., 300., 25.))
  );

  widget_layout_test!(
    fix_flex_gap,
    WidgetTester::new(fn_widget! {
      @Column {
        item_gap: 50.,
        @SizedBox { size: Size::new(100., 100.) }
        @SizedBox { size: Size::new(100., 500.) }
      }
    })
    .with_wnd_size(Size::new(500., 500.)),
    LayoutCase::default().with_height(500.),
    LayoutCase::new(&[0, 0])
      .with_y(0.)
      .with_height(100.),
    LayoutCase::new(&[0, 1])
      .with_y(150.)
      .with_height(350.)
  );
}
