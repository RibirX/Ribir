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

/// The `Flex` is a layout container that arranges its children in a
/// one-dimensional manner. It distributes space among the children and provides
/// alignment options in two axes.
///
/// The flex container consists of a main axis and a cross axis. The main axis
/// is determined by the `direction` property, while the cross axis is
/// perpendicular to it. The `direction` property can be set to
/// `Direction::Horizontal` or `Direction::Vertical`, and setting `reverse` to
/// true will reverse the main axis.
///
/// If the direction of the flex container is known, consider using [`Row`] or
/// [`Column`] instead. The `wrap` property controls whether flex items should
/// wrap onto multiple lines or remain on a single line in the main axis.
///
/// The `align_items` property specifies how flex items are positioned in the
/// flex container along the cross axis, while `justify_content` determines
/// their placement along the main axis.
///
/// Adjust the `item_gap` property to set the gap between items in the main
/// axis, and the `line_gap` property for the gap between lines in the cross
/// axis.
///
/// Regarding expansion and shrinking, use an [`Expanded`] widget to make a
/// child expand to fill the available space along the main axis. The space is
/// distributed to expanded children based on their `flex` value, with the
/// available space being the remaining area in the main axis after allocating
/// space for all children.
///
/// Therefore, the `Expanded` widget will expand only within a fixed-size
/// container.

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

#[macro_export]
macro_rules! row {
  ($($t: tt)*) => { fn_widget! { @Row { $($t)* } } };
}

#[macro_export]
macro_rules! column {
  ($($t: tt)*) => { fn_widget! { @Column { $($t)* } } };
}

pub use column;
pub use row;

impl Declare for Row {
  type Builder = FlexDeclarer;
  fn declarer() -> Self::Builder {
    let mut f = Flex::declarer();
    f.direction(Direction::Horizontal);
    f
  }
}

impl Declare for Column {
  type Builder = FlexDeclarer;
  fn declarer() -> Self::Builder {
    let mut f = Flex::declarer();
    f.direction(Direction::Vertical);
    f
  }
}

impl Render for Flex {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    if Align::Stretch == self.align_items && self.wrap {
      warn!("stretch align and wrap property is conflict");
    }

    let mut layouter = FlexLayouter {
      reverse: self.reverse,
      dir: self.direction,
      align_items: self.align_items,
      justify_content: self.justify_content,
      wrap: self.wrap,
      main_axis_gap: self.item_gap,
      cross_axis_gap: self.line_gap,
      current_line: <_>::default(),
      lines: vec![],
      has_flex: false,
    };
    layouter.layout(clamp, ctx)
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
}

impl std::ops::Sub for FlexSize {
  type Output = Self;
  fn sub(self, rhs: Self) -> Self::Output {
    FlexSize { main: self.main - rhs.main, cross: self.cross - rhs.cross }
  }
}

struct FlexLayouter {
  reverse: bool,
  dir: Direction,
  align_items: Align,
  justify_content: JustifyContent,
  wrap: bool,
  current_line: MainLineInfo,
  lines: Vec<MainLineInfo>,
  main_axis_gap: f32,
  cross_axis_gap: f32,
  has_flex: bool,
}

impl FlexLayouter {
  fn layout(&mut self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    // Perform children layout without limit its main axis, and if its cross
    // axis is stretch the children need to align in cross axis so we also not limit
    // the cross axis.
    let dir = self.dir;
    let flex_max = FlexSize::from_size(clamp.max, dir);
    let flex_min = FlexSize::from_size(clamp.min, dir);

    let child_clamp = self.create_child_clamp(clamp);
    self.perform_children_layout(flex_max.main, child_clamp, ctx);
    if self.need_secondary_layout() {
      // Uses minimum main axis size as distribution basis when max constraint
      // is unbounded
      let flex_main = if flex_max.main.is_finite() { flex_max.main } else { flex_min.main };
      self.stretch_and_flex_layout(flex_main, child_clamp, ctx);
    }

    let expect = self.finally_size(flex_max);
    let real_size = clamp.clamp(expect.to_size(dir));
    let real = FlexSize::from_size(real_size, dir);
    let cross_box_offset = self
      .align_items
      .align_value(expect.cross, real.cross);
    self.update_children_position(real.main, cross_box_offset, ctx);
    real_size
  }

  /// Creates child constraints based on wrapping behavior:
  /// - Wrapped: No limits on either axis
  /// - Not wrapped: Allows unlimited main axis growth while preserving cross
  ///   axis constraints
  fn create_child_clamp(&self, clamp: BoxClamp) -> BoxClamp {
    if self.wrap {
      BoxClamp::default()
    } else {
      match self.dir {
        Direction::Horizontal => BoxClamp::max_height(clamp.max.height),
        Direction::Vertical => BoxClamp::max_width(clamp.max.width),
      }
    }
  }

  fn need_secondary_layout(&self) -> bool { self.has_flex || self.align_items == Align::Stretch }

  fn perform_children_layout(&mut self, max_main: f32, clamp: BoxClamp, ctx: &mut LayoutCtx) {
    let (ctx, children) = ctx.split_children();
    let &mut Self { wrap, dir, .. } = self;
    let mut children = children.peekable();
    while let Some(c) = children.next() {
      let gap = if children.peek().is_some() && !self.justify_content.is_space_layout() {
        self.main_axis_gap
      } else {
        0.
      };

      let line = &mut self.current_line;

      let expanded = ctx
        .query_of_widget::<Expanded>(c)
        .map(|e| *e)
        .filter(|e| e.flex.is_normal() && e.flex > 0.);

      let size = if expanded.is_some_and(|e| e.defer_alloc) {
        FlexSize::default()
      } else {
        let size = ctx.perform_child_layout(c, clamp);
        FlexSize::from_size(size, dir)
      };

      if wrap && !line.is_empty() && line.main_width + size.main > max_main {
        self.place_line();
      } else {
        line.main_width += gap;
      }

      let line = &mut self.current_line;
      line.main_width += size.main;

      let flex = expanded.map(|e| {
        self.current_line.has_flex = true;
        self.has_flex = true;
        e.flex
      });
      let info = FlexLayoutInfo { flex, pos: <_>::default(), size };
      self.current_line.items_info.push(info);
    }

    self.place_line();
  }

  fn stretch_and_flex_layout(&mut self, main_width: f32, clamp: BoxClamp, ctx: &mut LayoutCtx) {
    let (ctx, mut children) = ctx.split_children();
    let dir = self.dir;

    self.lines.iter_mut().for_each(|line| {
      let mut line_clamp = clamp;
      if self.align_items == Align::Stretch {
        let cross = line.max_cross();
        line_clamp = match dir {
          Direction::Horizontal => line_clamp.with_fixed_height(cross),
          Direction::Vertical => line_clamp.with_fixed_width(cross),
        };
      }
      let flex_unit = line.calc_flex_unit_and_remove_useless_flex(main_width);

      line.items_info.iter_mut().for_each(|info| {
        let child = children.next().unwrap();
        let mut item_clamp = line_clamp;
        if let (Some(flex), Some(unit)) = (info.flex, flex_unit) {
          let main = unit * flex;
          item_clamp = match dir {
            Direction::Horizontal => item_clamp.with_fixed_width(main),
            Direction::Vertical => item_clamp.with_fixed_height(main),
          };
        }

        if self.align_items == Align::Stretch
          || (info.flex.is_some() && flex_unit.is_some())
          || ctx.box_size().is_none()
        {
          let size = ctx.perform_child_layout(child, item_clamp);
          let new_size = FlexSize::from_size(size, dir);
          line.main_width += new_size.main - info.size.main;
          info.size = new_size;
        }
      });
    });
  }

  fn finally_size(&self, flex_max: FlexSize) -> FlexSize {
    let main = match self.justify_content {
      JustifyContent::SpaceBetween | JustifyContent::SpaceAround | JustifyContent::SpaceEvenly
        if flex_max.main.is_finite() =>
      {
        flex_max.main
      }
      _ => self
        .lines
        .iter()
        .fold(0f32, |max, l| max.max(l.main_width)),
    };

    let cross = if self.lines.is_empty() {
      0.0
    } else {
      self
        .lines
        .iter()
        .map(|l| l.max_cross())
        .sum::<f32>()
        + self.cross_axis_gap * (self.lines.len() - 1) as f32
    };
    FlexSize { main, cross }
  }

  fn update_children_position(
    &mut self, main_bound: f32, cross_box_offset: f32, ctx: &mut LayoutCtx,
  ) {
    let Self { reverse, dir, align_items, justify_content, lines, .. } = self;

    macro_rules! update_position {
      ($($rev: ident)?) => {
        let mut cross = cross_box_offset - self.cross_axis_gap;
        lines.iter_mut()$(.$rev())?.for_each(|line| {
          let (mut main, step) = line.place_args(main_bound, *justify_content, self.main_axis_gap);
          let line_cross = line.max_cross();
          line.items_info.iter_mut()$(.$rev())?.for_each(|item| {
            let item_cross_offset =
              align_items.align_value(item.size.cross, line_cross);

            item.pos.cross = cross + item_cross_offset + self.cross_axis_gap;
            item.pos.main = main;
            main = main + item.size.main + step;
          });
          cross += line_cross + self.cross_axis_gap;
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
  has_flex: bool,
}

struct FlexLayoutInfo {
  pos: FlexSize,
  size: FlexSize,
  flex: Option<f32>,
}

impl MainLineInfo {
  fn is_empty(&self) -> bool { self.items_info.is_empty() }

  fn calc_flex_unit_and_remove_useless_flex(&mut self, max: f32) -> Option<f32> {
    if !self.has_flex || self.main_width >= max {
      return None;
    }

    let unit = self.flex_unit(max)?;
    let mut unused_flex = false;
    self
      .items_info
      .iter_mut()
      .for_each(|item: &mut FlexLayoutInfo| {
        if item
          .flex
          .is_some_and(|flex| flex * unit < item.size.main)
        {
          item.flex = None;
          unused_flex = true;
        }
      });

    if unused_flex { self.flex_unit(max) } else { Some(unit) }
  }

  fn flex_unit(&self, max: f32) -> Option<f32> {
    let (flex_sum, flex_width) = self
      .items_info
      .iter()
      .filter_map(|info| info.flex.map(|flex| (flex, info.size.main)))
      .fold((0., 0.), |sum, (flex, size)| (sum.0 + flex, sum.1 + size));

    let available_space = max - self.main_width + flex_width;
    Some(available_space / flex_sum)
  }

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

  fn max_cross(&self) -> f32 {
    self
      .items_info
      .iter()
      .map(|info| info.size.cross)
      .max_by(|a, b| a.total_cmp(b))
      .unwrap_or(0.)
  }
}

impl JustifyContent {
  fn is_space_layout(&self) -> bool {
    matches!(
      self,
      JustifyContent::SpaceAround | JustifyContent::SpaceBetween | JustifyContent::SpaceEvenly
    )
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
        h_align: HAlign::Stretch,
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
            defer_alloc: false,
            flex: 2.,
            @SizedBox { size: Size::splat(100.),}
          }
          @Expanded {
            defer_alloc: false,
            flex: 1.,
            @SizedBox { size: Size::splat(50.),}
          }
          @SizedBox { size: Size::new(100., 20.) }
          @Expanded {
            defer_alloc: false,
            // The flex will be ignored, because the flex is not enough
            flex: 0.5,
            @SizedBox { size: Size::splat(100.), }
          }
        }
      }
    })
    .with_wnd_size(Size::new(500., 500.)),
    LayoutCase::new(&[0, 0]).with_rect(ribir_geom::rect(0., 0., 500., 25.)),
    LayoutCase::new(&[0, 0, 0]).with_rect(ribir_geom::rect(0., 0., 200., 25.)),
    LayoutCase::new(&[0, 0, 1]).with_rect(ribir_geom::rect(200., 0., 100., 25.)),
    LayoutCase::new(&[0, 0, 2]).with_rect(ribir_geom::rect(300., 0., 100., 20.)),
    LayoutCase::new(&[0, 0, 3]).with_rect(ribir_geom::rect(400., 0., 100., 25.))
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
      .with_height(500.)
  );

  widget_layout_test!(
    cross_greater_than_children,
    WidgetTester::new(row! {
      clamp: BoxClamp::min_height(500.),
      align_items: Align::Center,
      @Container { size: Size::new(100., 100.) }
    })
    .with_wnd_size(Size::new(500., 500.)),
    LayoutCase::default().with_height(500.),
    LayoutCase::new(&[0, 0]).with_rect(ribir_geom::rect(0., 200., 100., 100.))
  );
}
