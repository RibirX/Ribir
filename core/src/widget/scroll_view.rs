use std::{cell::RefCell, rc::Rc};

use crate::prelude::*;

use super::scrollbar::ScrollBar;

#[derive(Clone, Copy)]
pub enum ScrollMode {
  Auto,
  Always,
  Hidden,
}

#[derive(Clone)]
struct ViewLayout {
  view: f32,
  content: f32,
}

#[derive(Clone)]
pub struct ScrollInfo {
  layout: Rc<RefCell<ViewLayout>>,
  offset: f32,
  mode: Option<ScrollMode>,
  direction: Direction,
}

impl ScrollInfo {
  fn new(direction: Direction, mode: Option<ScrollMode>) -> Self {
    ScrollInfo {
      layout: Rc::new(RefCell::new(ViewLayout { view: 0., content: 0. })),
      offset: 0.,
      mode,
      direction,
    }
  }

  fn update_view_silent(&self, view: f32) { self.layout.borrow_mut().view = view; }

  fn update_content_silent(&self, content: f32) { self.layout.borrow_mut().content = content; }

  fn update_offset(&mut self, offset: f32) { self.offset = offset; }

  pub fn direction(&self) -> Direction { self.direction }

  pub fn scrollbar_offset(&self) -> f32 {
    let info = self.layout.borrow();
    info.view * self.offset / info.content
  }

  pub fn scrollbar_size(&self) -> f32 {
    let info = self.layout.borrow();
    info.view * info.view / info.content
  }

  pub fn is_show(&self) -> bool {
    let info = self.layout.borrow();
    let ratio = info.view / info.content;
    match self.mode {
      Some(ScrollMode::Auto) => ratio < 1.,
      Some(ScrollMode::Always) => true,
      Some(ScrollMode::Hidden) => false,
      None => false,
    }
  }
}

#[derive(MultiChildWidget, Declare, Clone)]
struct ViewContainer {
  horizontal: ScrollInfo,
  vertical: ScrollInfo,
}

impl Default for ViewContainer {
  fn default() -> ViewContainer {
    ViewContainer {
      horizontal: ScrollInfo::new(Direction::Horizontal, Some(ScrollMode::Auto)),
      vertical: ScrollInfo::new(Direction::Vertical, Some(ScrollMode::Auto)),
    }
  }
}

#[derive(Clone, Copy, PartialEq)]
enum ElemType {
  Content,
  VScrollBar,
  HScrollBar,
}

impl ViewContainer {
  fn content_box(&self, ctx: &mut LayoutCtx) -> Option<Rect> {
    let tree = ctx.widget_tree();
    let scroll_box = ctx.id().first_child(tree).unwrap();
    let content = scroll_box.single_child(tree).unwrap();

    ctx.layout_store().layout_box_rect(content)
  }

  fn update_scroll_layout(&self, clamp: &BoxClamp, ctx: &mut LayoutCtx) {
    let rc = self.content_box(ctx).unwrap();
    let view = clamp.clamp(rc.size);
    self.horizontal.update_content_silent(rc.width());
    self.horizontal.update_view_silent(view.width);

    self.vertical.update_content_silent(rc.height());
    self.vertical.update_view_silent(view.height);
  }

  #[inline]
  fn iter_children<'a, 'b>(
    &self,
    ctx: &'b mut LayoutCtx<'a>,
  ) -> (
    &'b mut LayoutCtx<'a>,
    impl Iterator<Item = (WidgetId, ElemType)> + 'b,
  ) {
    let (new_ctx, children) = ctx.split_children();
    let types = [
      ElemType::Content,
      ElemType::VScrollBar,
      ElemType::HScrollBar,
    ];
    (new_ctx, children.zip(types.into_iter()))
  }

  fn calc_position(&self, elem_type: ElemType, child_size: Size, content_size: Size) -> Point {
    match elem_type {
      ElemType::Content => Point::zero(),
      ElemType::VScrollBar => Point::new(content_size.width - child_size.width, 0.),
      ElemType::HScrollBar => Point::new(0., content_size.height - child_size.height),
    }
  }
}

impl Render for ViewContainer {
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let (new_ctx, it) = self.iter_children(ctx);

    let mut self_size = clamp.min.clone();
    for (wid, elem_type) in it {
      let child_size = new_ctx.perform_child_layout(wid, clamp);
      self_size = self_size.max(child_size);
      new_ctx.update_position(wid, self.calc_position(elem_type, child_size, self_size));

      if elem_type == ElemType::Content {
        self.update_scroll_layout(&clamp, new_ctx);
      }
    }

    self_size
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}
}

pub struct ScrollView {}
impl ScrollView {
  pub fn build(content: BoxedWidget, ctx: &mut BuildCtx) -> BoxedWidget
  where
    Self: Sized,
  {
    let container = ViewContainer::default();
    let mut scroll = ScrollableBoth { pos: Point::zero() }.into_stateful();

    let v_bar = ScrollBar::new(container.vertical.clone(), ctx).into_stateful();
    let mut v_bar_ref = unsafe { v_bar.state_ref() };

    let h_bar = ScrollBar::new(container.horizontal.clone(), ctx).into_stateful();
    let mut h_bar_ref = unsafe { h_bar.state_ref() };

    scroll
      .state_change(|w| w.offset_x())
      .subscribe(move |offset| {
        h_bar_ref.info.update_offset(offset.after);
      });

    scroll
      .state_change(|w| w.offset_y())
      .subscribe(move |offset| {
        v_bar_ref.info.update_offset(offset.after);
      });

    container
      .have_child([
        scroll.have_child(content).box_it(),
        v_bar.box_it(),
        h_bar.box_it(),
      ])
      .box_it()
  }
}

#[cfg(test)]
mod tests {
  use std::{cell::RefCell, rc::Rc};

  use crate::{
    prelude::Direction,
    widget::scroll_view::{ScrollInfo, ScrollMode, ViewLayout},
  };
  #[test]
  fn show_scrollbar() {
    assert!(
      ScrollInfo {
        layout: Rc::new(RefCell::new(ViewLayout { view: 30., content: 60. })),
        offset: 0.,
        mode: Some(ScrollMode::Auto),
        direction: Direction::Vertical,
      }
      .is_show()
    );

    assert!(
      !ScrollInfo {
        layout: Rc::new(RefCell::new(ViewLayout { view: 60., content: 30. })),
        offset: 0.,
        mode: Some(ScrollMode::Auto),
        direction: Direction::Vertical,
      }
      .is_show()
    );

    assert!(
      !ScrollInfo {
        layout: Rc::new(RefCell::new(ViewLayout { view: 30., content: 60. })),
        offset: 0.,
        mode: None,
        direction: Direction::Vertical,
      }
      .is_show()
    );

    assert!(
      ScrollInfo {
        layout: Rc::new(RefCell::new(ViewLayout { view: 60., content: 30. })),
        offset: 0.,
        mode: Some(ScrollMode::Always),
        direction: Direction::Vertical,
      }
      .is_show()
    );

    assert!(
      !ScrollInfo {
        layout: Rc::new(RefCell::new(ViewLayout { view: 30., content: 60. })),
        offset: 0.,
        mode: Some(ScrollMode::Hidden),
        direction: Direction::Vertical,
      }
      .is_show()
    );
  }
}
