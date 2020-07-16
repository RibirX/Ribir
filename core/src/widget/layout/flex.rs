use super::expanded::ExpandedRender;
use crate::prelude::*;
use crate::render::render_tree::*;
use smallvec::{smallvec, SmallVec};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Direction {
  /// Left and right.
  Horizontal,
  /// Up and down.
  Vertical,
}

#[derive(Debug)]
pub struct Flex {
  pub reverse: bool,
  pub wrap: bool,
  pub direction: Direction,
  pub children: SmallVec<[BoxWidget; 1]>,
}

#[derive(Debug)]
pub struct FlexRender {
  pub reverse: bool,
  pub direction: Direction,
  pub wrap: bool,
}

impl Flex {
  #[inline]
  pub fn push<W: Widget>(&mut self, child: W) -> &mut Self {
    self.children.push(child.box_it());
    self
  }

  #[inline]
  pub fn with_reverse(mut self, reverse: bool) -> Self {
    self.reverse = reverse;
    self
  }

  #[inline]
  pub fn with_direction(mut self, direction: Direction) -> Self {
    self.direction = direction;
    self
  }

  #[inline]
  pub fn with_wrap(mut self, wrap: bool) -> Self {
    self.wrap = wrap;
    self
  }
}

impl std::iter::FromIterator<BoxWidget> for Flex {
  fn from_iter<T: IntoIterator<Item = BoxWidget>>(iter: T) -> Self {
    Self {
      reverse: false,
      wrap: false,
      direction: Direction::Horizontal,
      children: iter.into_iter().collect(),
    }
  }
}

impl Default for Flex {
  fn default() -> Self {
    Self {
      reverse: false,
      wrap: false,
      direction: Direction::Horizontal,
      children: smallvec![],
    }
  }
}
impl Default for Direction {
  #[inline]
  fn default() -> Self { Direction::Horizontal }
}

render_widget_base_impl!(Flex);

impl RenderWidget for Flex {
  type RO = FlexRender;
  fn create_render_object(&self) -> Self::RO {
    FlexRender {
      reverse: self.reverse,
      direction: self.direction,
      wrap: self.wrap,
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
  }

  fn perform_layout(&mut self, clamp: BoxClamp, ctx: &mut RenderCtx) -> Size {
    let mut layouter = FlexLayouter::new(clamp, self.direction, self.wrap);
    if self.reverse {
      layouter.children_perform(ctx.reverse_children());
      layouter.expanded_widget_flex(ctx.reverse_children())
    } else {
      layouter.children_perform(ctx.children());
      layouter.expanded_widget_flex(ctx.children())
    }
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

#[derive(Default)]
struct FlexLayouter {
  clamp: BoxClamp,
  direction: Direction,
  /// the max of child touch in main axis
  main_max: f32,
  wrap: bool,
  current_line: MainLineInfo,
  lines_info: Vec<MainLineInfo>,
}

impl FlexLayouter {
  fn new(clamp: BoxClamp, direction: Direction, wrap: bool) -> Self {
    Self {
      clamp,
      direction,
      wrap,
      ..Default::default()
    }
  }

  fn children_perform<'a>(&mut self, children: impl Iterator<Item = RenderCtx<'a>>) {
    let max = self.clamp.max;
    let boundary = FlexSize::from_size(max, self.direction);
    let clamp = BoxClamp {
      max,
      min: Size::zero(),
    };

    children.for_each(|mut child_ctx| {
      let size = child_ctx.perform_layout(clamp);
      let flex_size = FlexSize::from_size(size, self.direction);
      if self.wrap
        && !self.current_line.is_empty()
        && self.current_line.main_width + flex_size.main > boundary.main
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

  fn expanded_widget_flex<'a>(
    &mut self,
    mut children: impl Iterator<Item = RenderCtx<'a>>,
  ) -> Size {
    let size = FlexSize::from_size(self.clamp.max, self.direction);

    let Self {
      lines_info,
      main_max,
      direction,
      ..
    } = self;
    let dir = *direction;
    lines_info.iter_mut().for_each(|line| {
      // resize the expanded widget.
      let mut flex_sum = line.flex_sum;
      if flex_sum > 0. {
        let mut offset_main = 0.;
        let mut remain_space = size.main - line.main_width + line.flex_main_width;
        (0..line.child_count)
          .map(|_| children.next().unwrap())
          .for_each(|mut ctx| {
            let prefer_rect = ctx
              .box_rect()
              .expect("relayout a expanded widget which not prepare layout");

            if offset_main > 0. {
              let mut flex_pos = FlexSize::from_point(prefer_rect.origin, dir);
              flex_pos.main += offset_main;
              ctx.update_position(flex_pos.to_point(dir));
            }

            if let Some(flex) = Self::child_flex(&ctx) {
              let expand_main = remain_space * (flex / flex_sum);
              let prefer_size = FlexSize::from_size(ctx.box_rect().unwrap().size, dir);
              // relayout the expand widget with new clamp.
              if expand_main > prefer_size.main {
                let clamp = BoxClamp {
                  max: FlexSize {
                    main: expand_main,
                    cross: line.cross_line_height,
                  }
                  .to_size(dir),
                  min: FlexSize {
                    main: expand_main,
                    cross: 0.,
                  }
                  .to_size(dir),
                };
                let real_size = FlexSize::from_size(ctx.perform_layout(clamp), dir);
                offset_main += real_size.main - prefer_size.main;
                remain_space -= real_size.main;
              } else {
                remain_space -= prefer_size.main;
              }
              flex_sum -= flex;
            }
          });
        line.main_width += offset_main;
        *main_max = main_max.max(line.main_width);
      }
    });
    self.box_size()
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
      let mut new_line = MainLineInfo::default();
      new_line.cross_pos = self.current_line.cross_bottom();
      self
        .lines_info
        .push(std::mem::replace(&mut self.current_line, new_line));
    }
  }

  fn box_size(&self) -> Size {
    let cross = self
      .lines_info
      .last()
      .map(|line| line.cross_bottom())
      .unwrap_or(0.);
    self.clamp.clamp(
      FlexSize {
        cross,
        main: self.main_max,
      }
      .to_size(self.direction),
    )
  }

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
}
