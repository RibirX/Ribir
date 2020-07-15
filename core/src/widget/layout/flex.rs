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
    let mut layouter = FlexLayouter::new(clamp.max, self.direction, self.wrap);
    if self.reverse {
      layouter.perform_children(ctx.reverse_children());
      layouter.update_position(ctx.reverse_children());
    } else {
      layouter.perform_children(ctx.children());
      layouter.update_position(ctx.children());
    }
    clamp.clamp(layouter.best_size())
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
  #[inline]
  fn to_size(self, dir: Direction) -> Size {
    match dir {
      Direction::Horizontal => Size::new(self.main, self.cross),
      Direction::Vertical => Size::new(self.cross, self.main),
    }
  }

  #[inline]
  fn to_point(self, dir: Direction) -> Point { self.to_size(dir).to_vector().to_point() }

  #[inline]
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
  boundary: Size,
  direction: Direction,
  /// Store all place position of all children
  geometry_pos: Vec<FlexSize>,
  /// Store the (index, flex) of all expanded child.
  expended: Vec<(usize, f32)>,
  /// Flex sum.
  flex_sum: f32,
  /// the max of child touch in main axis
  main_max: f32,
  /// the position next child to place.
  allocated_start: FlexSize,
  /// current line's cross height.
  cross_line_height: f32,
  wrap: bool,
}

impl FlexLayouter {
  fn new(boundary: Size, direction: Direction, wrap: bool) -> Self {
    Self {
      boundary,
      direction,
      wrap,
      ..Default::default()
    }
  }

  fn perform_children<'a>(&mut self, children: impl Iterator<Item = RenderCtx<'a>>) {
    let boundary = FlexSize::from_size(self.boundary, self.direction);
    let clamp = BoxClamp {
      max: self.boundary,
      min: Size::zero(),
    };

    children.enumerate().for_each(|(idx, mut child_ctx)| {
      // expand item
      if false {
        self.expended.push((idx, 1.));
        self.geometry_pos.push(self.allocated_start);
      } else {
        let size = child_ctx.perform_layout(clamp);
        let flex_size = FlexSize::from_size(size, self.direction);
        if self.wrap && self.allocated_start.main + flex_size.main > boundary.main {
          self.main_max = self.main_max.max(self.allocated_start.main);
          // wrap to a new line to place child.
          self.allocated_start.cross += self.cross_line_height;
          self.allocated_start.main = flex_size.main;
          self.cross_line_height = flex_size.cross;
        } else {
          self.cross_line_height = self.cross_line_height.max(flex_size.cross);
          self.allocated_start.main += flex_size.main;
        }
        self.geometry_pos.push(FlexSize {
          main: self.allocated_start.main - flex_size.main,
          cross: self.allocated_start.cross,
        });
      }
    });
    self.main_max = self.main_max.max(self.allocated_start.main);
  }

  fn update_position<'a>(&mut self, children: impl Iterator<Item = RenderCtx<'a>>) {
    children
      .zip(self.geometry_pos.iter())
      .for_each(|(mut ctx, size)| ctx.update_position(size.to_point(self.direction)));
  }

  fn best_size(&self) -> Size {
    FlexSize {
      main: self.main_max,
      cross: self.allocated_start.cross + self.cross_line_height,
    }
    .to_size(self.direction)
  }
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
    let (rect, _) = widget_and_its_children_box_rect(row);
    assert_eq!(rect.size, Size::new(100., 20.));
  }

  #[test]
  fn vertical_line() {
    let col = (0..10)
      .map(|_| SizedBox::empty_box(Size::new(10., 20.)).box_it())
      .collect::<Flex>()
      .with_direction(Direction::Vertical);
    let (rect, _) = widget_and_its_children_box_rect(col);
    assert_eq!(rect.size, Size::new(10., 200.));
  }

  #[test]
  fn row_wrap() {
    let size = Size::new(200., 20.);
    let row = (0..3)
      .map(|_| SizedBox::empty_box(size).box_it())
      .collect::<Flex>()
      .with_wrap(true);
    let (rect, children) = widget_and_its_children_box_rect(row);
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
    let (rect, children) = widget_and_its_children_box_rect(row);
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
