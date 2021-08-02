use crate::prelude::*;

/// A widget that expanded a child of `Flex`, so that the child fills the
/// available space. If multiple children are expanded, the available space is
/// divided among them according to the flex factor.
#[stateful]
#[derive(Widget, SingleChildWidget)]
pub struct Expanded {
  #[state]
  pub flex: f32,
}

impl Expanded {
  pub fn new(flex: f32) -> Self { Self { flex } }
}

impl RenderWidget for Expanded {
  type RO = ExpandedState;
  #[inline]
  fn create_render_object(&self) -> Self::RO { ExpandedState { flex: self.flex } }
}

impl RenderObject for ExpandedState {
  type States = ExpandedState;

  #[inline]
  fn update(&mut self, states: Self::States, _: &mut UpdateCtx) { self.flex = states.flex; }

  #[inline]
  fn perform_layout(&mut self, clamp: BoxClamp, ctx: &mut RenderCtx) -> Size {
    debug_assert_eq!(ctx.children().count(), 1);

    ctx
      .children()
      .next()
      .expect("Expanded render should always have a single child")
      .perform_layout(clamp)
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  #[inline]
  fn paint<'a>(&'a self, _: &mut PaintingContext<'a>) {
    // nothing to draw.
  }

  #[inline]
  fn get_states(&self) -> &Self::States { self }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::*;

  #[test]
  fn one_line_expanded() {
    let size = Size::new(100., 50.);
    let row = Row::default()
      .push(
        Expanded::new(1.)
          .with_child(SizedBox::from_size(size).box_it())
          .box_it(),
      )
      .push(SizedBox::from_size(size).box_it())
      .push(SizedBox::from_size(size).box_it())
      .push(
        Expanded::new(2.)
          .with_child(SizedBox::from_size(size).box_it())
          .box_it(),
      )
      .box_it();

    let (rect, children) = widget_and_its_children_box_rect(row, Size::new(500., 500.));

    assert_eq!(rect, Rect::from_size(Size::new(500., 50.)));
    assert_eq!(
      children,
      vec![
        Rect::from_size(size),
        Rect::new(Point::new(100., 0.), size),
        Rect::new(Point::new(200., 0.), size),
        Rect::new(Point::new(300., 0.), Size::new(200., 50.))
      ]
    )
  }

  #[test]
  fn wrap_expanded() {
    let size = Size::new(100., 50.);
    let row = Row::default()
      .with_wrap(true)
      .push(
        Expanded::new(1.)
          .with_child(SizedBox::from_size(size).box_it())
          .box_it(),
      )
      .push(SizedBox::from_size(size).box_it())
      .push(SizedBox::from_size(size).box_it())
      .push(
        Expanded::new(2.)
          .with_child(SizedBox::from_size(size).box_it())
          .box_it(),
      );

    let (rect, children) = widget_and_its_children_box_rect(row.box_it(), Size::new(350., 500.));

    assert_eq!(rect, Rect::from_size(Size::new(350., 100.)));
    assert_eq!(
      children,
      vec![
        Rect::from_size(Size::new(150., 50.)),
        Rect::new(Point::new(150., 0.), size),
        Rect::new(Point::new(250., 0.), size),
        Rect::new(Point::new(0., 50.), Size::new(350., 50.))
      ]
    )
  }
}
