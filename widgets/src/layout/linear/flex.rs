use ribir_core::prelude::{log::warn, *};

use super::{Direction, Expanded, JustifyContent};

/// Enum describing how a widget is aligned inside its box.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Align {
  /// The children are aligned to the start edge of the box provided by parent.
  #[default]
  Start,
  /// The children are aligned to the center of the line of the box provide by
  /// parent.
  Center,
  /// The children are aligned to the end edge of the box provided by parent.
  End,
  /// Require the children to fill the whole box of one axis. This causes the
  /// constraints passed to the children to be tight.
  Stretch,
}

impl Align {
  /// Calculate the offset for aligning a child of `child_size` within
  /// `parent_size`.
  pub fn align_value(self, child_size: f32, parent_size: f32) -> f32 {
    match self {
      Align::Start => 0.,
      Align::Center => (parent_size - child_size) / 2.,
      Align::End => parent_size - child_size,
      Align::Stretch => 0.,
    }
  }
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
/// If the flex container's direction is known and it's used in a single-line
/// layout (without wrapping or expanding), consider using [`Row`] or [`Column`]
/// instead.
///
/// The `wrap` property controls whether flex items should wrap onto multiple
/// lines or remain on a single line in the main axis.
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

impl Render for Flex {
  fn measure(&self, clamp: BoxClamp, ctx: &mut MeasureCtx) -> Size {
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
    layouter.measure_children(clamp, ctx)
  }

  fn place_children(&self, size: Size, ctx: &mut PlaceCtx) {
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
    layouter.layout_children(size, ctx)
  }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}
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
  fn measure_children(&mut self, clamp: BoxClamp, ctx: &mut MeasureCtx) -> Size {
    let dir = self.dir;

    let main_max = dir.max_of(&clamp);

    let child_clamp = self.create_child_clamp(clamp);
    self.perform_children_measure(main_max, child_clamp, ctx);
    if self.has_flex {
      let container = dir.container_main(&clamp, self.main_size());
      self.flex_measure(container, main_max, child_clamp, ctx);
    }

    let expect = self.finally_size(main_max);
    clamp.clamp(expect)
  }

  fn layout_children(&mut self, size: Size, ctx: &mut PlaceCtx) {
    let dir = self.dir;
    let main_max = dir.main_of(size);

    // Rebuild lines info from already-measured children (no re-measuring)
    self.rebuild_lines_from_cache(main_max, ctx);

    let expect = self.finally_size(main_max);
    let cross_box_offset = self
      .align_items
      .align_value(dir.cross_of(expect), dir.cross_of(size));
    self.update_children_position(dir.main_of(size), cross_box_offset, ctx);
  }

  /// Creates child constraints based on wrapping behavior:
  /// - Wrapped: No limits on either axis
  /// - Not wrapped: Allows unlimited main axis growth while preserving cross
  ///   axis constraints
  fn create_child_clamp(&self, clamp: BoxClamp) -> BoxClamp {
    if self.wrap {
      BoxClamp::default()
    } else {
      let max_cross = self.dir.cross_max_of(&clamp);
      if self.align_items == Align::Stretch && max_cross.is_finite() {
        self
          .dir
          .with_fixed_cross(BoxClamp::default(), max_cross)
      } else {
        self
          .dir
          .with_cross_max(BoxClamp::default(), max_cross)
      }
    }
  }

  fn perform_children_measure(&mut self, max_main: f32, clamp: BoxClamp, ctx: &mut MeasureCtx) {
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
        .filter(|e| (e.flex.is_normal() && e.flex > 0.) || e.flex == 0.);

      let size = if expanded.is_some_and(|e| e.defer_alloc) {
        Size::zero()
      } else {
        ctx.layout_child(c, clamp)
      };
      let main = dir.main_of(size);
      if wrap && !line.is_empty() && line.main + main > max_main {
        self.place_line();
      } else {
        line.main += gap;
      }

      let line = &mut self.current_line;
      line.main += main;

      let flex = expanded.map(|e| {
        self.current_line.has_flex = true;
        self.has_flex = true;
        e.flex
      });
      let info = FlexLayoutInfo {
        flex,
        main_pos: 0.,
        cross_pos: 0.,
        size,
        defer_layout: expanded.is_some_and(|e| e.defer_alloc),
      };
      self.current_line.items_info.push(info);
    }

    self.place_line();
  }

  /// Rebuild lines info from already-measured children for the layout phase.
  /// This method uses widget_box_size instead of perform_child_layout to avoid
  /// re-measuring children during the layout phase.
  fn rebuild_lines_from_cache(&mut self, max_main: f32, ctx: &mut PlaceCtx) {
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
        .filter(|e| (e.flex.is_normal() && e.flex > 0.) || e.flex == 0.);

      // Use cached size instead of re-measuring
      let size = ctx.widget_box_size(c).unwrap_or(Size::zero());
      let main = dir.main_of(size);
      if wrap && !line.is_empty() && line.main + main > max_main {
        self.place_line();
      } else {
        line.main += gap;
      }

      let line = &mut self.current_line;
      line.main += main;

      let flex = expanded.map(|e| {
        self.current_line.has_flex = true;
        self.has_flex = true;
        e.flex
      });
      let info = FlexLayoutInfo {
        flex,
        main_pos: 0.,
        cross_pos: 0.,
        size,
        defer_layout: expanded.is_some_and(|e| e.defer_alloc),
      };
      self.current_line.items_info.push(info);
    }

    self.place_line();
  }

  fn flex_measure(&mut self, container: f32, max: f32, clamp: BoxClamp, ctx: &mut MeasureCtx) {
    let (ctx, mut children) = ctx.split_children();
    let dir = self.dir;

    self.lines.iter_mut().for_each(|line| {
      let line_clamp = clamp;
      let (flex_unit, mut space_left) = line.calc_flex_unit_and_space_left(self.dir, container);
      for info in line.items_info.iter_mut() {
        let child = children.next().unwrap();
        let item_main = dir.main_of(info.size);
        let mut item_clamp = line_clamp;
        if info.defer_layout {
          if flex_unit == 0. && max > container {
            let max = max - container + space_left;
            item_clamp = dir.with_max(item_clamp, max);
          } else {
            let main = info.flex.unwrap() * flex_unit;
            item_clamp = dir.with_fixed_main(item_clamp, main);
          }
        } else if info.flex.is_some() && space_left > 0. {
          let main = (info.flex.unwrap() * flex_unit).min(item_main + space_left);
          space_left -= main - item_main;
          item_clamp = dir.with_fixed_main(item_clamp, main);
        } else {
          continue;
        };

        info.size = ctx.layout_child(child, item_clamp);
        line.main += dir.main_of(info.size) - item_main;
      }
    });
  }

  fn finally_size(&self, main_max: f32) -> Size {
    let main = if main_max.is_finite() && self.justify_content.is_space_layout() {
      main_max
    } else {
      self
        .lines
        .iter()
        .fold(0f32, |max, l| max.max(l.main))
    };

    let cross = if self.lines.is_empty() {
      0.0
    } else {
      self
        .lines
        .iter()
        .map(|l| l.max_cross(self.dir))
        .sum::<f32>()
        + self.cross_axis_gap * (self.lines.len() - 1) as f32
    };

    self.dir.to_size(main, cross)
  }

  fn update_children_position(&mut self, container: f32, cross_offset: f32, ctx: &mut PlaceCtx) {
    let Self { reverse, dir, align_items, justify_content, cross_axis_gap, main_axis_gap, .. } =
      *self;
    let mut cross = cross_offset - cross_axis_gap;
    self.for_each_line(|line| {
      let (mut main, mut step) =
        justify_content.item_offset_and_step(container - line.main, line.items_info.len());
      if !justify_content.is_space_layout() {
        step += main_axis_gap;
      }

      let line_cross = line.max_cross(dir);
      line.for_each_item(reverse, |item| {
        let (item_main, item_cross) = dir.main_cross_of(item.size);
        let item_cross_offset = align_items.align_value(item_cross, line_cross);

        item.cross_pos = cross + item_cross_offset + cross_axis_gap;
        item.main_pos = main;
        main = main + item_main + step;
      });
      cross += line_cross + cross_axis_gap;
    });

    let (ctx, mut children) = ctx.split_children();

    self.lines.iter_mut().for_each(|line| {
      line.items_info.iter_mut().for_each(|info| {
        let child = children.next().unwrap();
        let pos = dir.to_point(info.main_pos, info.cross_pos);
        ctx.update_position(child, pos);
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

  fn main_size(&self) -> f32 { self.lines.iter().fold(0., |m, l| m.max(l.main)) }

  fn for_each_line(&mut self, mut f: impl FnMut(&mut MainLineInfo)) {
    if self.reverse {
      self.lines.iter_mut().rev().for_each(&mut f);
    } else {
      self.lines.iter_mut().for_each(&mut f);
    }
  }
}

#[derive(Default)]
struct MainLineInfo {
  main: f32,
  items_info: Vec<FlexLayoutInfo>,
  has_flex: bool,
}

#[derive(Debug)]
struct FlexLayoutInfo {
  main_pos: f32,
  cross_pos: f32,
  size: Size,
  flex: Option<f32>,
  defer_layout: bool,
}

impl MainLineInfo {
  fn is_empty(&self) -> bool { self.items_info.is_empty() }

  // calc the flex unit and space left for all flex items with defer_alloc is
  // false to try to expand to size according to flex.
  // - return (flex_unit, space_left), all the flex items will try to expanded to
  //   size according to flex.
  // - return (flex_unit, 0.), only the expanded items with defer_alloc is true
  //   will try to expanded to size according to flex.
  fn calc_flex_unit_and_space_left(&mut self, dir: Direction, max: f32) -> (f32, f32) {
    if !self.has_flex {
      return (0., 0.);
    }

    // no space left, it's compact. no need to re-layout for flex with defer_alloc
    // is false.
    if self.main >= max {
      return (0., 0.);
    }

    let (mut flex_alloc, mut flex_defer, mut flex_width) = (0., 0., 0.);
    for item in self
      .items_info
      .iter()
      .filter(|info| info.flex.is_some())
    {
      if item.defer_layout {
        flex_defer += item.flex.unwrap();
      } else {
        flex_alloc += item.flex.unwrap();
      }
      flex_width += dir.main_of(item.size);
    }

    if flex_alloc + flex_defer <= 0. {
      return (0., 0.);
    }

    let unit = (max - self.main + flex_width) / (flex_alloc + flex_defer);
    if unit * flex_alloc >= flex_width {
      (unit, unit * flex_alloc - flex_width)
    } else if flex_defer > 0. {
      ((max - self.main) / flex_defer, 0.)
    } else {
      (0., 0.)
    }
  }

  fn max_cross(&self, dir: Direction) -> f32 {
    self
      .items_info
      .iter()
      .fold(0., |acc, info| acc.max(dir.cross_of(info.size)))
  }

  fn for_each_item(&mut self, reverse: bool, mut f: impl FnMut(&mut FlexLayoutInfo)) {
    if reverse {
      self.items_info.iter_mut().rev().for_each(&mut f);
    } else {
      self.items_info.iter_mut().for_each(&mut f);
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
          (0..10).map(|_| @Container { size: Size::new(10., 20.) })
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
        @{ (0..10).map(|_| @Container { size: Size::new(10., 20.) })}
      }
    }),
    LayoutCase::default().with_size(Size::new(10., 200.))
  );

  widget_layout_test!(
    row_wrap,
    WidgetTester::new(fn_widget! {
      @Flex {
        wrap: true,
        @{ (0..3).map(|_| @Container { size: Size::new(200., 20.) }) }
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
        @{ (0..3).map(|_| @Container { size: Size::new(200., 20.) }) }
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
      @Flex {
        item_gap: 15.,
        @Container { size: Size::new(120., 20.) }
        @Container { size: Size::new(80., 20.) }
        @Container { size: Size::new(30., 20.) }
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
      @Flex {
        item_gap: 15.,
        reverse: true,
        @Container { size: Size::new(120., 20.) }
        @Container { size: Size::new(80., 20.) }
        @Container { size: Size::new(30., 20.) }
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
      @Flex {
        clamp: BoxClamp::EXPAND_X,
        item_gap: 15.,
        @Container { size: Size::new(120., 20.) }
        @Expanded {
          flex: 1.,
          @Container { size: Size::new(10., 20.) }
        }
        @Container { size: Size::new(80., 20.) }
        @Expanded {
          flex: 2.,
          @Container { size: Size::new(10., 20.) }
        }
        @Container { size: Size::new(30., 20.) }
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
        @{ (0..3).map(|_| @Container { size: Size::new(200., 20.) }) }
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
      @Flex {
        align_items: align,
        @Container { size: Size::new(100., 20.) }
        @Container { size: Size::new(100., 30.) }
        @Container { size: Size::new(100., 40.) }
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
      @Container {
        size: Size::new(500., 500.),
        @Flex {
          justify_content,
          align_items: Align::Start,
          @Container { size: item_size }
          @Container { size: item_size }
          @Container { size: item_size }
        }
      }
    })
    .with_wnd_size(Size::new(500., 500.))
  }

  widget_layout_test!(
    start_main_align,
    main_align(JustifyContent::Compact),
    LayoutCase::new(&[0, 0]).with_size(Size::new(300., 20.)),
    LayoutCase::new(&[0, 0, 0]).with_x(0.),
    LayoutCase::new(&[0, 0, 1]).with_x(100.),
    LayoutCase::new(&[0, 0, 2]).with_x(200.)
  );

  widget_layout_test!(
    space_between_align,
    main_align(JustifyContent::SpaceBetween),
    LayoutCase::new(&[0, 0]).with_size(Size::new(500., 20.)),
    LayoutCase::new(&[0, 0, 0]).with_x(0.),
    LayoutCase::new(&[0, 0, 1]).with_x(200.),
    LayoutCase::new(&[0, 0, 2]).with_x(400.)
  );

  const AROUND_SPACE: f32 = 200.0 / 3.0;
  widget_layout_test!(
    space_around_align,
    main_align(JustifyContent::SpaceAround),
    LayoutCase::new(&[0, 0]).with_size(Size::new(500., 20.)),
    LayoutCase::new(&[0, 0, 0]).with_x(0.5 * AROUND_SPACE),
    LayoutCase::new(&[0, 0, 1]).with_x(100. + AROUND_SPACE * 1.5),
    LayoutCase::new(&[0, 0, 2]).with_x(2.5 * AROUND_SPACE + 200.)
  );

  widget_layout_test!(
    space_evenly_align,
    main_align(JustifyContent::SpaceEvenly),
    LayoutCase::new(&[0, 0]).with_size(Size::new(500., 20.)),
    LayoutCase::new(&[0, 0, 0]).with_x(50.),
    LayoutCase::new(&[0, 0, 1]).with_x(200.),
    LayoutCase::new(&[0, 0, 2]).with_x(350.)
  );

  widget_layout_test!(
    flex_expand,
    WidgetTester::new(fn_widget! {
      @Container {
        size: Size::new(500., 25.),
        @Flex {
          direction: Direction::Horizontal,
          @Expanded {
            defer_alloc: false,
            flex: 2.,
            @Container { size: Size::splat(100.),}
          }
          @Expanded {
            defer_alloc: false,
            flex: 1.,
            @Container { size: Size::splat(50.),}
          }
          @Container { size: Size::new(100., 20.) }
          @Expanded {
            defer_alloc: false,
            flex: 1.,
            @Container { size: Size::splat(100.), }
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
      @Flex {
        direction: Direction::Vertical,
        item_gap: 50.,
        @Container { size: Size::new(100., 100.) }
        @Container { size: Size::new(100., 500.) }
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
    WidgetTester::new(flex! {
      clamp: BoxClamp::min_height(500.),
      align_items: Align::Center,
      @Container { size: Size::new(100., 100.) }
    })
    .with_wnd_size(Size::new(500., 500.)),
    LayoutCase::default().with_height(500.),
    LayoutCase::new(&[0, 0]).with_rect(ribir_geom::rect(0., 200., 100., 100.))
  );

  widget_layout_test!(
    flex_when_zero_space,
    WidgetTester::new(flex! {
      direction: Direction::Vertical,
      @Container {
        size: Size::new(60., 500.),
      }
      @Expanded {
        flex: 1.,
        @Container {
          size: Size::new(50., 140.),
        }
      }
      @Expanded {
        flex: 1.,
        defer_alloc: false,
        @Container {
          size: Size::new(50., 140.),
        }
      }
    })
    .with_wnd_size(Size::new(500., 500.)),
    LayoutCase::default().with_height(500.),
    LayoutCase::new(&[0, 0]).with_rect(ribir_geom::rect(0., 0., 60., 500.)),
    LayoutCase::new(&[0, 1]).with_rect(ribir_geom::rect(0., 500., 50., 0.)),
    LayoutCase::new(&[0, 2]).with_rect(ribir_geom::rect(0., 500., 50., 140.))
  );

  widget_layout_test!(
    fix_defer_alloc_has_size_in_unlimited_clamp,
    WidgetTester::new(unconstrained_box! {
      @Flex {
        @Container {
          size: Size::new(300., 300.),
        }
        @Expanded {
          flex: 1.,
          defer_alloc: true,
          @Container {
            size: Size::new(50., 140.),
          }
        }
      }
    })
    .with_wnd_size(Size::splat(300.)),
    LayoutCase::new(&[0, 0, 1]).with_size(Size::new(50., 140.)),
  );
}
