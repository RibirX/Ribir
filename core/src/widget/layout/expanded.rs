use crate::prelude::*;

/// A widget that expanded a child of `Flex`, so that the child fills the
/// available space. If multiple children are expanded, the available space is
/// divided among them according to the flex factor.
#[derive(SingleChildWidget, Clone, PartialEq, Declare)]
pub struct Expanded {
  pub flex: f32,
}

impl Expanded {
  pub fn new(flex: f32) -> Self { Self { flex } }
}

impl Render for Expanded {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let child = ctx
      .single_child()
      .expect("Expanded render should always have a single child");
    ctx.perform_child_layout(child, clamp)
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::*;

  #[test]
  fn one_line_expanded() {
    let widget = |size| {
      widget! {
        declare Row {
          Expanded {
            flex: 1.,
            SizedBox { size }
          }
          SizedBox { size }
          SizedBox { size }
          Expanded {
            flex: 2.,
            SizedBox { size }
          }
        }
      }
    };

    let size = Size::new(100., 50.);

    let (rect, children) = widget_and_its_children_box_rect(widget(size), Size::new(500., 500.));

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
    let row = Row { wrap: true, ..<_>::default() }
      .have_child(
        Expanded { flex: 1. }
          .have_child(SizedBox { size }.box_it())
          .box_it(),
      )
      .have_child(SizedBox { size }.box_it())
      .have_child(SizedBox { size }.box_it())
      .have_child(
        Expanded { flex: 2. }
          .have_child(SizedBox { size }.box_it())
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
