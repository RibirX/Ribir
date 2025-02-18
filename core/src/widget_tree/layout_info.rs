use std::collections::HashMap;

use ribir_geom::{Rect, ZERO_SIZE};

use super::{Lerp, WidgetId, WidgetTree};
use crate::prelude::{INFINITY_SIZE, Point, Size};

/// boundary limit of the render object's layout
#[derive(Debug, Clone, PartialEq, Copy, Lerp)]
pub struct BoxClamp {
  pub min: Size,
  pub max: Size,
}

impl BoxClamp {
  pub const UNLIMITED: BoxClamp = BoxClamp { min: ZERO_SIZE, max: INFINITY_SIZE };

  /// clamp use fixed width and unfixed height
  pub const fn fixed_width(width: f32) -> Self {
    BoxClamp { min: Size::new(width, 0.), max: Size::new(width, f32::INFINITY) }
  }

  /// clamp use fixed height and unfixed width
  pub const fn fixed_height(height: f32) -> Self {
    BoxClamp { min: Size::new(0., height), max: Size::new(f32::INFINITY, height) }
  }

  /// clamp use fixed size
  pub const fn fixed_size(size: Size) -> Self { BoxClamp { min: size, max: size } }

  pub const fn min_width(width: f32) -> Self {
    let mut clamp = Self::UNLIMITED;
    clamp.min.width = width;
    clamp
  }

  pub const fn min_height(height: f32) -> Self {
    let mut clamp = Self::UNLIMITED;
    clamp.min.height = height;
    clamp
  }

  pub const fn min_size(min: Size) -> Self {
    Self { min, max: Size::new(f32::INFINITY, f32::INFINITY) }
  }

  pub const fn max_size(max: Size) -> Self { Self { min: ZERO_SIZE, max } }

  pub fn with_min_size(mut self, size: Size) -> Self {
    self.min = size.min(self.max);
    self
  }

  pub fn with_max_size(mut self, size: Size) -> Self {
    self.max = size.max(self.min);
    self
  }

  pub const fn with_fixed_height(mut self, height: f32) -> Self {
    self.min.height = height;
    self.max.height = height;
    self
  }

  pub const fn with_fixed_width(mut self, width: f32) -> Self {
    self.min.width = width;
    self.max.width = width;
    self
  }

  pub fn with_max_width(mut self, width: f32) -> Self {
    self.max.width = width.max(self.min.width);
    self
  }

  pub fn with_max_height(mut self, height: f32) -> Self {
    self.max.height = height.max(self.min.height);
    self
  }

  pub fn with_min_width(mut self, width: f32) -> Self {
    self.min.width = width.min(self.max.width);
    self
  }

  pub fn with_min_height(mut self, height: f32) -> Self {
    self.min.height = height.min(self.max.height);
    self
  }
}

#[derive(Default, Debug, Clone, PartialEq, Copy)]
pub struct VisualBox {
  /// the bounds rect of the render object
  pub rect: Option<Rect>,
  /// the bounds rect of the subtree
  pub subtree: Option<Rect>,
}

impl VisualBox {
  pub fn bounds_rect(&self) -> Option<Rect> {
    match (self.rect, self.subtree) {
      (Some(rect), Some(subtree)) => Some(rect.union(&subtree)),
      (Some(rect), None) => Some(rect),
      (None, Some(subtree)) => Some(subtree),
      (None, None) => None,
    }
  }
}

/// render object's layout box, the information about layout, including box
/// size, box position, and the clamp of render object layout.
#[derive(Debug, Default, Clone)]
pub struct LayoutInfo {
  /// Box bound is the bound of the layout can be place. it will be set after
  /// render object computing its layout. It's passed by render object's parent.
  pub clamp: BoxClamp,
  /// The size of the object's layout result, indicating that the object has
  /// been laid out; otherwise, it is `None`.
  pub size: Option<Size>,
  /// The position render object to place, default is zero
  pub pos: Point,

  /// the visual box of the render object
  pub visual_box: VisualBox,
}

/// Store the render object's place relative to parent coordinate and the
/// clamp passed from parent.
#[derive(Default)]
pub(crate) struct LayoutStore {
  data: HashMap<WidgetId, LayoutInfo, ahash::RandomState>,
}

impl LayoutStore {
  /// Remove the layout info of the `wid`
  pub(crate) fn force_layout(&mut self, id: WidgetId) -> Option<LayoutInfo> { self.remove(id) }

  pub(crate) fn remove(&mut self, id: WidgetId) -> Option<LayoutInfo> { self.data.remove(&id) }

  pub(crate) fn layout_box_size(&self, id: WidgetId) -> Option<Size> {
    self.layout_info(id).and_then(|info| info.size)
  }

  pub(crate) fn layout_box_pos(&self, id: WidgetId) -> Option<Point> {
    self.layout_info(id).map(|info| info.pos)
  }

  pub(crate) fn layout_info(&self, id: WidgetId) -> Option<&LayoutInfo> { self.data.get(&id) }

  /// return a mutable reference of the layout info  of `id`, if it's not exist
  /// insert a default value before return
  pub(crate) fn layout_info_or_default(&mut self, id: WidgetId) -> &mut LayoutInfo {
    self.data.entry(id).or_default()
  }
}

impl WidgetTree {
  pub(crate) fn map_to_parent(&self, id: WidgetId, pos: Point) -> Point {
    self
      .store
      .layout_box_pos(id)
      .map_or(pos, |offset| {
        let pos = id
          .assert_get(self)
          .get_transform()
          .map_or(pos, |t| t.transform_point(pos));
        pos + offset.to_vector()
      })
  }

  pub(crate) fn map_from_parent(&self, id: WidgetId, pos: Point) -> Point {
    self
      .store
      .layout_box_pos(id)
      .map_or(pos, |offset| {
        let pos = pos - offset.to_vector();
        id.assert_get(self)
          .get_transform()
          .map_or(pos, |t| {
            t.inverse()
              .map_or(pos, |t| t.transform_point(pos))
          })
      })
  }

  pub(crate) fn map_to_global(&self, pos: Point, widget: WidgetId) -> Point {
    widget
      .ancestors(self)
      .fold(pos, |pos, p| self.map_to_parent(p, pos))
  }

  pub(crate) fn map_from_global(&self, pos: Point, widget: WidgetId) -> Point {
    let stack = widget.ancestors(self).collect::<Vec<_>>();
    stack
      .iter()
      .rev()
      .fold(pos, |pos, p| self.map_from_parent(*p, pos))
  }
}

impl BoxClamp {
  #[inline]
  pub fn clamp(self, size: Size) -> Size { size.clamp(self.min, self.max) }

  #[inline]
  pub fn expand(mut self) -> Self {
    self.max = INFINITY_SIZE;
    self
  }

  #[inline]
  pub fn loose(mut self) -> Self {
    self.min = ZERO_SIZE;
    self
  }
}

impl Default for BoxClamp {
  fn default() -> Self {
    Self { min: Size::new(0., 0.), max: Size::new(f32::INFINITY, f32::INFINITY) }
  }
}

impl std::ops::Deref for LayoutStore {
  type Target = HashMap<WidgetId, LayoutInfo, ahash::RandomState>;
  fn deref(&self) -> &Self::Target { &self.data }
}

impl std::ops::DerefMut for LayoutStore {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.data }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{prelude::*, reset_test_env, test_helper::*};

  #[derive(Declare, Clone, SingleChild)]
  struct OffsetBox {
    pub offset: Point,
    pub size: Size,
  }

  impl Render for OffsetBox {
    fn perform_layout(&self, mut clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
      clamp.max = clamp.max.min(self.size);
      let child = ctx.assert_single_child();
      ctx.perform_child_layout(child, clamp);
      ctx.update_position(child, self.offset);
      self.size
    }

    #[inline]
    fn only_sized_by_parent(&self) -> bool { true }

    #[inline]
    fn paint(&self, _: &mut PaintingCtx) {}
  }

  #[test]
  fn fix_incorrect_relayout_root() {
    reset_test_env!();

    // Can't use layout info of dirty widget to detect if the ancestors path have
    // in relayout list. Because new widget insert by `DynWidget` not have layout
    // info, but its parent have.

    let child_box = Stateful::new(MockBox { size: Size::zero() });
    let c_child_box = child_box.clone_writer();
    let (layout_cnt, w_layout_cnt) = split_value(0);

    let w = fn_widget! {
      let child_box = child_box.clone_writer();
      @MockMulti {
        on_performed_layout: move |_| *$w_layout_cnt.write() += 1,
        @ {
          pipe!($child_box.size.is_empty())
            .map(move|b| if b {
                MockBox { size: Size::new(1., 1.) }.into_widget()
              } else {
                child_box.clone_writer().into_widget()
            })
        }
      }
    };

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();
    assert_eq!(*layout_cnt.read(), 1);
    {
      c_child_box.write().size = Size::new(2., 2.);
    }
    wnd.draw_frame();
    assert_eq!(*layout_cnt.read(), 2);
  }

  #[test]
  fn layout_list_from_root_to_leaf() {
    reset_test_env!();

    let layout_order = Stateful::new(vec![]);
    let trigger = Stateful::new(Size::zero());
    let order = layout_order.clone_writer();
    let size = trigger.clone_watcher();
    let w = fn_widget! {
      @MockBox {
        size: pipe!(*$size),
        on_performed_layout: move |_| $order.write().push(1),
        @MockBox {
          size: pipe!(*$size),
          on_performed_layout: move |_| $order.write().push(2),
          @MockBox {
            size: pipe!(*$size),
            on_performed_layout: move |_| $order.write().push(3),
          }
        }
      }
    };

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();
    assert_eq!([3, 2, 1], &**layout_order.read());
    {
      *trigger.write() = Size::new(1., 1.);
    }
    wnd.draw_frame();
    assert_eq!([3, 2, 1, 3, 2, 1], &**layout_order.read());
  }

  #[test]
  fn relayout_size() {
    reset_test_env!();

    let trigger = Stateful::new(Size::zero());
    let size = trigger.clone_watcher();
    let w = fn_widget! {
      @OffsetBox {
        size: Size::new(100., 100.),
        offset: Point::new(50., 50.),
        @MockBox {
          size: Size::new(50., 50.),
          @MockBox { size: pipe!(*$size) }
        }
      }
    };

    #[track_caller]
    fn assert_rect_by_path(wnd: &TestWindow, path: &[usize], rect: Rect) {
      let info = wnd.layout_info_by_path(path).unwrap();
      assert_eq!(info.pos, rect.origin);
      assert_eq!(info.size.unwrap(), rect.size);
    }

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();
    assert_rect_by_path(&wnd, &[0, 0], ribir_geom::rect(50., 50., 50., 50.));
    assert_rect_by_path(&wnd, &[0, 0, 0], ribir_geom::rect(0., 0., 0., 0.));

    {
      *trigger.write() = Size::new(10., 10.);
    }

    wnd.draw_frame();
    assert_rect_by_path(&wnd, &[0, 0], ribir_geom::rect(50., 50., 50., 50.));
    assert_rect_by_path(&wnd, &[0, 0, 0], ribir_geom::rect(0., 0., 10., 10.));
  }

  #[test]
  fn relayout_from_parent() {
    reset_test_env!();

    let (cnt, w_cnt) = split_value(0);
    let (size, w_size) = split_value(Size::zero());
    let w = fn_widget! {
      @MockBox {
        size: Size::new(50., 50.),
        on_performed_layout: move |_| *$w_cnt.write() += 1,
        @MockBox { size: pipe!(*$size) }
      }
    };

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();
    assert_eq!(*cnt.read(), 1);

    *w_size.write() = Size::new(10., 10.);

    wnd.draw_frame();
    assert_eq!(*cnt.read(), 2);
  }
}
