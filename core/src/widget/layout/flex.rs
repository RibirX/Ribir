use super::expanded::ExpandedRender;
use crate::prelude::*;
use crate::render::render_tree::*;
use smallvec::{smallvec, SmallVec};

/// How the children should be placed along the cross axis in a flex layout.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum CrossAxisAlign {
  /// Place the children with their start edge aligned with the start side of
  /// the cross axis.
  Start,
  /// Place the children so that their centers align with the middle of the
  /// cross axis.This is the default cross-axis alignment.
  Center,
  /// Place the children as close to the end of the cross axis as possible.
  End,
  /// Require the children to fill the cross axis. This causes the constraints
  /// passed to the children to be tight in the cross axis.
  Stretch,
}

/// How the children should be placed along the main axis in a flex layout.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum MainAxisAlign {
  /// Place the children as close to the start of the main axis as possible.
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

#[derive(Debug, Default)]
pub struct Flex {
  /// Reverse the main axis.
  pub reverse: bool,
  /// Whether flex items are forced onto one line or can wrap onto multiple
  /// lines
  pub wrap: bool,
  /// Sets how flex items are placed in the flex container defining the main
  /// axis and the direction
  pub direction: Direction,
  /// How the children should be placed along the cross axis in a flex layout.
  pub cross_align: CrossAxisAlign,
  /// How the children should be placed along the main axis in a flex layout.
  pub main_align: MainAxisAlign,
  pub children: SmallVec<[BoxWidget; 1]>,
}

#[derive(Debug)]
pub struct FlexRender {
  pub reverse: bool,
  pub direction: Direction,
  cross_align: CrossAxisAlign,
  main_align: MainAxisAlign,
  pub wrap: bool,
}

impl Flex {
  /// Add a children into the flex container.
  #[inline]
  pub fn push<W: Widget>(mut self, child: W) -> Self {
    self.children.push(child.box_it());
    self
  }

  /// Create a new Flex like `self`, but with the give `reverse`.
  #[inline]
  pub fn with_reverse(mut self, reverse: bool) -> Self {
    self.reverse = reverse;
    self
  }

  /// Create a new Flex like `self`, but with the give `direction`.
  #[inline]
  pub fn with_direction(mut self, direction: Direction) -> Self {
    self.direction = direction;
    self
  }

  /// Create a new Flex like `self`, but with the give `cross_align`.
  #[inline]
  pub fn with_cross_align(mut self, cross_align: CrossAxisAlign) -> Self {
    self.cross_align = cross_align;
    self
  }

  /// Create a new Flex like `self`, but with the give `main_align`.
  #[inline]
  pub fn with_main_align(mut self, main_align: MainAxisAlign) -> Self {
    self.main_align = main_align;
    self
  }

  /// Create a new Flex like `self`, but with the give `wrap`.
  #[inline]
  pub fn with_wrap(mut self, wrap: bool) -> Self {
    self.wrap = wrap;
    self
  }
}

impl std::iter::FromIterator<BoxWidget> for Flex {
  fn from_iter<T: IntoIterator<Item = BoxWidget>>(iter: T) -> Self {
    Self {
      children: iter.into_iter().collect(),
      ..Default::default()
    }
  }
}

impl Default for CrossAxisAlign {
  #[inline]
  fn default() -> Self { CrossAxisAlign::Center }
}

impl Default for MainAxisAlign {
  #[inline]
  fn default() -> Self { MainAxisAlign::Start }
}

impl RenderWidget for Flex {
  type RO = FlexRender;
  fn create_render_object(&self) -> Self::RO {
    FlexRender {
      reverse: self.reverse,
      direction: self.direction,
      wrap: self.wrap,
      cross_align: self.cross_align,
      main_align: self.main_align,
    }
  }

  #[inline]
  fn take_children(&mut self) -> Option<SmallVec<[BoxWidget; 1]>> {
    Some(std::mem::replace(&mut self.children, smallvec![]))
  }
}

impl RenderObject for FlexRender {
  type Owner = Flex;
  fn update(&mut self, owner: &Self::Owner, ctx: &mut UpdateCtx) {
    if self.wrap != owner.wrap {
      self.wrap = owner.wrap;
      ctx.mark_needs_layout();
    }
    if self.reverse != owner.reverse {
      self.reverse = owner.reverse;
      ctx.mark_needs_layout();
    }
    if self.direction != owner.direction {
      self.direction = owner.direction;
      ctx.mark_needs_layout();
    }
    if self.cross_align != owner.cross_align {
      self.cross_align = owner.cross_align;
      ctx.mark_needs_layout();
    }

    if self.main_align != owner.main_align {
      self.main_align = owner.main_align;
      ctx.mark_needs_layout();
    }
  }

  fn perform_layout(&mut self, clamp: BoxClamp, ctx: &mut RenderCtx) -> Size {
    let direction = self.direction;
    let mut layouter = FlexLayouter {
      max_size: FlexSize::from_size(clamp.max, direction),
      min_size: FlexSize::from_size(clamp.min, direction),
      direction,
      reverse: self.reverse,
      wrap: self.wrap,
      main_max: 0.,
      current_line: <_>::default(),
      lines_info: vec![],
      cross_align: self.cross_align,
      main_align: self.main_align,
    };
    layouter.layout(ctx)
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  #[inline]
  fn paint<'a>(&'a self, _: &mut PaintingContext<'a>) {}
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
      Direction::Horizontal => Self {
        main: size.width,
        cross: size.height,
      },
      Direction::Vertical => Self {
        cross: size.width,
        main: size.height,
      },
    }
  }

  fn to_point(self, dir: Direction) -> Point { self.to_size(dir).to_vector().to_point() }

  fn from_point(pos: Point, dir: Direction) -> Self {
    FlexSize::from_size(Size::new(pos.x, pos.y), dir)
  }

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
  cross_align: CrossAxisAlign,
  main_align: MainAxisAlign,
}

impl FlexLayouter {
  fn layout(&mut self, ctx: &mut RenderCtx) -> Size {
    if self.reverse {
      self.children_perform(ctx.reverse_children());
      self.relayout_if_need(ctx.reverse_children());
      let size = self.box_size();
      self.line_inner_align(ctx.reverse_children(), size);
      size.to_size(self.direction)
    } else {
      self.children_perform(ctx.children());
      self.relayout_if_need(ctx.children());
      let size = self.box_size();
      self.line_inner_align(ctx.children(), size);
      size.to_size(self.direction)
    }
  }

  fn children_perform<'a>(&mut self, children: impl Iterator<Item = RenderCtx<'a>>) {
    let clamp = BoxClamp {
      max: self.max_size.to_size(self.direction),
      min: Size::zero(),
    };

    children.for_each(|mut child_ctx| {
      let size = child_ctx.perform_layout(clamp);
      let flex_size = FlexSize::from_size(size, self.direction);
      if self.wrap
        && !self.current_line.is_empty()
        && self.current_line.main_width + flex_size.main > self.max_size.main
      {
        self.place_line();
      }
      child_ctx.update_position(
        FlexSize {
          main: self.current_line.main_width,
          cross: self.current_line.cross_pos,
        }
        .to_point(self.direction),
      );
      self.place_widget(flex_size, &child_ctx);
    });
    self.place_line();
  }

  fn relayout_if_need<'a>(&mut self, mut children: impl Iterator<Item = RenderCtx<'a>>) {
    let Self {
      lines_info,
      direction,
      cross_align,
      max_size,
      main_max,
      ..
    } = self;
    lines_info.iter_mut().for_each(|line| {
      (0..line.child_count)
        .map(|_| children.next().unwrap())
        .fold(0.0f32, |main_offset, mut child_ctx| {
          Self::obj_real_rect_with_main_start(
            &mut child_ctx,
            line,
            main_offset,
            *direction,
            *cross_align,
            *max_size,
          )
        });
      *main_max = main_max.max(line.main_width);
    });
  }

  fn line_inner_align<'a>(
    &mut self,
    mut children: impl Iterator<Item = RenderCtx<'a>>,
    size: FlexSize,
  ) {
    let real_size = self.best_size();
    let Self {
      lines_info,
      main_align,
      direction,
      cross_align,
      ..
    } = self;
    let container_cross_offset = match cross_align {
      CrossAxisAlign::Start | CrossAxisAlign::Stretch => 0.,
      CrossAxisAlign::Center => (size.cross - real_size.cross) / 2.,
      CrossAxisAlign::End => size.cross - real_size.cross,
    };
    lines_info.iter_mut().for_each(|line| {
      let (offset, step) = match main_align {
        MainAxisAlign::Start => (0., 0.),
        MainAxisAlign::Center => ((size.main - line.main_width) / 2., 0.),
        MainAxisAlign::End => (size.main - line.main_width, 0.),
        MainAxisAlign::SpaceAround => {
          let step = (size.main - line.main_width) / line.child_count as f32;
          (step / 2., step)
        }
        MainAxisAlign::SpaceBetween => {
          let step = (size.main - line.main_width) / (line.child_count - 1) as f32;
          (0., step)
        }
        MainAxisAlign::SpaceEvenly => {
          let step = (size.main - line.main_width) / (line.child_count + 1) as f32;
          (step, step)
        }
      };

      (0..line.child_count)
        .map(|_| children.next().unwrap())
        .fold(offset, |main_offset: f32, mut child_ctx| {
          let rect = child_ctx
            .box_rect()
            .expect("relayout a expanded widget which not prepare layout");
          let mut origin = FlexSize::from_point(rect.origin, *direction);
          let child_size = FlexSize::from_size(rect.size, *direction);

          let line_cross_offset = match cross_align {
            CrossAxisAlign::Start | CrossAxisAlign::Stretch => 0.,
            CrossAxisAlign::Center => (line.cross_line_height - child_size.cross) / 2.,
            CrossAxisAlign::End => line.cross_line_height - child_size.cross,
          };
          origin.main += main_offset;
          origin.cross += container_cross_offset + line_cross_offset;
          child_ctx.update_position(origin.to_point(*direction));
          main_offset + step
        });
    });
  }

  fn place_widget(&mut self, size: FlexSize, child_ctx: &RenderCtx) {
    let mut line = &mut self.current_line;
    line.main_width += size.main;
    line.cross_line_height = line.cross_line_height.max(size.cross);
    line.child_count += 1;
    if let Some(flex) = Self::child_flex(child_ctx) {
      line.flex_sum += flex;
      line.flex_main_width += size.main;
    }
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

  // relayout child to get the real size, and return the new offset in main axis
  // for next siblings.
  fn obj_real_rect_with_main_start(
    child_ctx: &mut RenderCtx,
    line: &mut MainLineInfo,
    main_offset: f32,
    dir: Direction,
    cross_align: CrossAxisAlign,
    max_size: FlexSize,
  ) -> f32 {
    let pre_layout_rect = child_ctx
      .box_rect()
      .expect("relayout a expanded widget which not prepare layout");

    let pre_size = FlexSize::from_size(pre_layout_rect.size, dir);
    let mut prefer_main = pre_size.main;
    if let Some(flex) = Self::child_flex(&child_ctx) {
      let remain_space = max_size.main - line.main_width + line.flex_main_width;
      prefer_main = remain_space * (flex / line.flex_sum);
      line.flex_sum -= flex;
      line.flex_main_width -= pre_size.main;
    }
    prefer_main = prefer_main.max(pre_size.main);

    let clamp_max = FlexSize {
      main: prefer_main,
      cross: line.cross_line_height,
    };
    let mut clamp_min = FlexSize {
      main: prefer_main,
      cross: 0.,
    };
    if CrossAxisAlign::Stretch == cross_align {
      clamp_min.cross = line.cross_line_height;
    }

    let real_size = if prefer_main > pre_size.main || clamp_min.cross > pre_size.cross {
      // Relayout only if the child object size may change.
      let new_size = child_ctx.perform_layout(BoxClamp {
        max: clamp_max.to_size(dir),
        min: clamp_min.to_size(dir),
      });
      FlexSize::from_size(new_size, dir)
    } else {
      pre_size
    };

    let main_diff = real_size.main - pre_size.main;
    line.main_width += main_diff;

    let mut new_pos = FlexSize::from_point(pre_layout_rect.origin, dir);
    new_pos.main += main_offset;
    let new_pos = new_pos.to_point(dir);

    if pre_layout_rect.origin != new_pos {
      child_ctx.update_position(new_pos);
    }

    main_offset + main_diff
  }

  fn best_size(&self) -> FlexSize {
    let cross = self
      .lines_info
      .last()
      .map(|line| line.cross_bottom())
      .unwrap_or(0.);
    FlexSize {
      cross,
      main: self.main_max,
    }
  }

  fn box_size(&self) -> FlexSize { self.best_size().clamp(self.min_size, self.max_size) }

  fn child_flex(ctx: &RenderCtx) -> Option<f32> {
    ctx
      .render_obj()
      .downcast_ref::<ExpandedRender>()
      .map(|expanded| expanded.flex)
  }
}

#[derive(Default)]
struct MainLineInfo {
  child_count: usize,
  cross_pos: f32,
  main_width: f32,
  flex_sum: f32,
  flex_main_width: f32,
  cross_line_height: f32,
}

impl MainLineInfo {
  fn is_empty(&self) -> bool { self.child_count == 0 || self.main_width == 0. }

  fn cross_bottom(&self) -> f32 { self.cross_pos + self.cross_line_height }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::widget_and_its_children_box_rect;

  #[test]
  fn horizontal_line() {
    let row = (0..10)
      .map(|_| SizedBox::empty_box(Size::new(10., 20.)).box_it())
      .collect::<Flex>();
    let (rect, _) = widget_and_its_children_box_rect(row, Size::new(500., 500.));
    assert_eq!(rect.size, Size::new(100., 20.));
  }

  #[test]
  fn vertical_line() {
    let col = (0..10)
      .map(|_| SizedBox::empty_box(Size::new(10., 20.)).box_it())
      .collect::<Flex>()
      .with_direction(Direction::Vertical);
    let (rect, _) = widget_and_its_children_box_rect(col, Size::new(500., 500.));
    assert_eq!(rect.size, Size::new(10., 200.));
  }

  #[test]
  fn row_wrap() {
    let size = Size::new(200., 20.);
    let row = (0..3)
      .map(|_| SizedBox::empty_box(size).box_it())
      .collect::<Flex>()
      .with_wrap(true);
    let (rect, children) = widget_and_its_children_box_rect(row, Size::new(500., 500.));
    assert_eq!(rect.size, Size::new(400., 40.));
    assert_eq!(
      children,
      vec![
        Rect::from_size(size),
        Rect {
          origin: Point::new(200., 0.),
          size
        },
        Rect {
          origin: Point::new(0., 20.),
          size
        },
      ]
    );
  }

  #[test]
  fn reverse_row_wrap() {
    let size = Size::new(200., 20.);
    let row = (0..3)
      .map(|_| SizedBox::empty_box(size).box_it())
      .collect::<Flex>()
      .with_wrap(true)
      .with_reverse(true);
    let (rect, children) = widget_and_its_children_box_rect(row, Size::new(500., 500.));
    assert_eq!(rect.size, Size::new(400., 40.));
    assert_eq!(
      children,
      vec![
        Rect {
          origin: Point::new(0., 20.),
          size
        },
        Rect {
          origin: Point::new(200., 0.),
          size
        },
        Rect::from_size(size),
      ]
    );
  }

  #[test]
  fn cross_align() {
    fn cross_align_check(align: CrossAxisAlign, y_pos: [f32; 3]) {
      let row = Row::default()
        .with_cross_align(align)
        .push(SizedBox::empty_box(Size::new(100., 20.)))
        .push(SizedBox::empty_box(Size::new(100., 30.)))
        .push(SizedBox::empty_box(Size::new(100., 40.)));

      let (rect, children) = widget_and_its_children_box_rect(row, Size::new(500., 500.));
      assert_eq!(rect.size, Size::new(300., 40.));
      assert_eq!(
        children,
        vec![
          Rect {
            origin: Point::new(0., y_pos[0]),
            size: Size::new(100., 20.)
          },
          Rect {
            origin: Point::new(100., y_pos[1]),
            size: Size::new(100., 30.)
          },
          Rect {
            origin: Point::new(200., y_pos[2]),
            size: Size::new(100., 40.)
          },
        ]
      );
    }
    cross_align_check(CrossAxisAlign::Start, [0., 0., 0.]);
    cross_align_check(CrossAxisAlign::Center, [10., 5., 0.]);
    cross_align_check(CrossAxisAlign::End, [20., 10., 0.]);

    let row = Row::default()
      .with_cross_align(CrossAxisAlign::Stretch)
      .push(SizedBox::empty_box(Size::new(100., 20.)))
      .push(SizedBox::empty_box(Size::new(100., 30.)))
      .push(SizedBox::empty_box(Size::new(100., 40.)));

    let (rect, children) = widget_and_its_children_box_rect(row, Size::new(500., 500.));
    assert_eq!(rect.size, Size::new(300., 40.));
    assert_eq!(
      children,
      vec![
        Rect {
          origin: Point::new(0., 0.),
          size: Size::new(100., 40.)
        },
        Rect {
          origin: Point::new(100., 0.),
          size: Size::new(100., 40.)
        },
        Rect {
          origin: Point::new(200., 0.),
          size: Size::new(100., 40.)
        },
      ]
    );
  }

  #[test]
  fn main_align() {
    fn main_align_check(align: MainAxisAlign, pos: [(f32, f32); 3]) {
      let item_size = Size::new(100., 20.);
      let row = Row::default()
        .with_main_align(align)
        .with_cross_align(CrossAxisAlign::Start)
        .push(SizedBox::empty_box(item_size))
        .push(SizedBox::empty_box(item_size))
        .push(SizedBox::empty_box(item_size));

      let mut wnd = window::Window::without_render(SizedBox::expanded(row), Size::new(500., 500.));
      wnd.render_ready();
      let r_tree = wnd.render_tree();
      let row_obj = r_tree
        .root()
        .unwrap()
        .children(&*r_tree)
        .take(1)
        .next()
        .unwrap();
      let rect = row_obj.layout_box_rect(&*r_tree).unwrap();
      let children = row_obj
        .children(&*r_tree)
        .map(|rid| rid.layout_box_rect(&*r_tree).unwrap())
        .collect::<Vec<_>>();

      assert_eq!(rect.size, Size::new(500., 500.));
      assert_eq!(
        children,
        vec![
          Rect {
            origin: pos[0].into(),
            size: item_size
          },
          Rect {
            origin: pos[1].into(),
            size: item_size
          },
          Rect {
            origin: pos[2].into(),
            size: item_size
          },
        ]
      );
    }

    main_align_check(MainAxisAlign::Start, [(0., 0.), (100., 0.), (200., 0.)]);
    main_align_check(MainAxisAlign::Center, [(100., 0.), (200., 0.), (300., 0.)]);
    main_align_check(MainAxisAlign::End, [(200., 0.), (300., 0.), (400., 0.)]);
    main_align_check(
      MainAxisAlign::SpaceBetween,
      [(0., 0.), (200., 0.), (400., 0.)],
    );
    let space = 200.0 / 3.0;
    main_align_check(
      MainAxisAlign::SpaceAround,
      [
        (0.5 * space, 0.),
        (100. + space * 1.5, 0.),
        (2.5 * space + 200., 0.),
      ],
    );
    main_align_check(
      MainAxisAlign::SpaceEvenly,
      [(50., 0.), (200., 0.), (350., 0.)],
    );
  }
}
