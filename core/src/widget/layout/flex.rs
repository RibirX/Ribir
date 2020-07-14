use crate::prelude::*;

use crate::render::render_tree::*;

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
  pub children: Vec<BoxWidget>,
}

#[derive(Debug)]
pub struct FlexRender {
  pub reverse: bool,
  pub direction: Direction,
  pub wrap: bool,
}

impl Flex {
  pub fn from_iter(children: impl Iterator<Item = BoxWidget>) -> Self {
    Self {
      reverse: false,
      wrap: false,
      direction: Direction::Horizontal,
      children: children.collect(),
    }
  }

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

impl Default for Flex {
  fn default() -> Self {
    Self {
      reverse: false,
      wrap: false,
      direction: Direction::Horizontal,
      children: vec![],
    }
  }
}

impl_widget_for_multi_child_widget!(Flex);

impl RenderWidget for Flex {
  type RO = FlexRender;
  fn create_render_object(&self) -> Self::RO {
    FlexRender {
      reverse: self.reverse,
      direction: self.direction,
      wrap: self.wrap,
    }
  }
}

impl MultiChildWidget for Flex {
  #[inline]
  fn take_children(&mut self) -> Vec<BoxWidget> { std::mem::replace(&mut self.children, vec![]) }
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

  fn perform_layout(&mut self, limit: BoxClamp, ctx: &mut RenderCtx) -> Size {
    let boundary = FlexSize::from_size(limit.max, self.direction);
    // Store all place position of all children
    let mut geometry_pos = vec![];
    // Store the (index, flex) of all expanded child.
    let mut expended = vec![];
    // Flex sum.
    let mut flex = 0.0;
    // the max of child touch in main axis
    let mut main_max = 0.0f32;
    let mut allocated_pos = FlexSize {
      main: 0.,
      cross: 0.,
    };
    // current line's cross height.
    let mut cross_line_height = 0.;

    ctx.children().enumerate().for_each(|(idx, mut child_ctx)| {
      // expand item
      if false {
        expended.push(idx);
      } else {
        let size = child_ctx.perform_layout(BoxClamp {
          max: (boundary - allocated_pos).to_size(self.direction),
          min: Size::zero(),
        });

        let flex_size = FlexSize::from_size(size, self.direction);
        if self.wrap && allocated_pos.main + flex_size.main > boundary.main {
          main_max = main_max.max(allocated_pos.main);
          // wrap to a new line to place child.
          allocated_pos.main = 0.;
          allocated_pos.cross += cross_line_height;
          cross_line_height = flex_size.cross;
        } else {
          cross_line_height = cross_line_height.max(flex_size.cross);
          allocated_pos.main += flex_size.main;
        }
      }
      geometry_pos.push(allocated_pos);
    });
    main_max = main_max.max(allocated_pos.main);

    // todo: lay out expanded children
    // todo: support align children

    ctx
      .children()
      .zip(geometry_pos.iter())
      .for_each(|(mut ctx, size)| ctx.update_position(size.to_point(self.direction)));

    allocated_pos.main = allocated_pos.main.max(main_max);
    allocated_pos.cross += cross_line_height;
    allocated_pos.to_size(self.direction)
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  #[inline]
  fn paint<'a>(&'a self, _: &mut PaintingContext<'a>) {}
}

impl FlexRender {
  pub fn new(dir: Direction) -> Flex {
    Flex {
      direction: dir,
      children: vec![],
      reverse: false,
      wrap: false,
    }
  }
}

#[derive(Debug, Clone, Copy)]
struct FlexSize {
  main: f32,
  cross: f32,
}

impl FlexSize {
  #[inline]
  fn to_size(&self, dir: Direction) -> Size {
    match dir {
      Direction::Horizontal => Size::new(self.main, self.cross),
      Direction::Vertical => Size::new(self.cross, self.main),
    }
  }

  #[inline]
  fn to_point(&self, dir: Direction) -> Point { self.to_size(dir).to_vector().to_point() }

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

#[cfg(test)]
mod tests {
  use super::*;
}
