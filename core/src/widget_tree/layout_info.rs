use std::collections::HashMap;

pub use ribir_types::BoxClamp;

use super::{Lerp, WidgetId, WidgetTree};
use crate::prelude::{Measure, Point, RFrom, Size};

impl Lerp for BoxClamp {
  fn lerp(&self, to: &Self, factor: f32) -> Self {
    Self { min: self.min.lerp(to.min, factor), max: self.max.lerp(to.max, factor) }
  }
}

// === Positioning Types ===

#[derive(Clone, PartialEq, Debug, Default)]
enum AlignType {
  #[default]
  Start,
  Center,
  End,
  Before,
  After,
}

/// A unit to describe the position of a widget relative to its parent.
#[derive(Clone, PartialEq, Default, Debug)]
struct AnchorUnit {
  align: AlignType,
  /// Pixel offset, accumulated via offset() calls
  pixel_offset: f32,
  /// Percent offset (0.0-1.0), accumulated via offset() calls
  percent_offset: f32,
}

impl AnchorUnit {
  fn new(align: AlignType) -> Self { Self { align, pixel_offset: 0., percent_offset: 0. } }

  /// Add an offset. Pixel and percent offsets are accumulated separately.
  fn offset(mut self, offset: impl Into<Measure>) -> Self {
    match offset.into() {
      Measure::Pixel(px) => self.pixel_offset += px,
      Measure::Unit(pct) => self.percent_offset += pct,
    }
    self
  }

  fn calculate(&self, reference: f32, this: f32) -> f32 {
    let offset = self.pixel_offset + self.percent_offset * reference;
    match self.align {
      AlignType::Start => offset,
      AlignType::Center => (reference - this) / 2. + offset,
      AlignType::End => reference - this - offset,
      AlignType::Before => -this + offset,
      AlignType::After => reference + offset,
    }
  }

  fn start() -> Self { Self::new(AlignType::Start) }
  fn center() -> Self { Self::new(AlignType::Center) }
  fn end() -> Self { Self::new(AlignType::End) }
  fn before() -> Self { Self::new(AlignType::Before) }
  fn after() -> Self { Self::new(AlignType::After) }
}

impl Lerp for AnchorUnit {
  fn lerp(&self, to: &Self, factor: f32) -> Self {
    if self.align == to.align {
      AnchorUnit {
        align: self.align.clone(),
        pixel_offset: self.pixel_offset.lerp(&to.pixel_offset, factor),
        percent_offset: self
          .percent_offset
          .lerp(&to.percent_offset, factor),
      }
    } else {
      to.clone()
    }
  }
}

#[derive(Default, PartialEq, Clone, Debug)]
pub struct AnchorX(AnchorUnit);

impl AnchorX {
  pub fn new(v: impl Into<Measure>) -> Self { Self(AnchorUnit::start().offset(v)) }
  pub fn percent(v: f32) -> Self { Self(AnchorUnit::start().offset(Measure::Unit(v))) }
  /// align to the left
  pub fn left() -> Self { Self(AnchorUnit::start()) }
  /// align to the right
  pub fn right() -> Self { Self(AnchorUnit::end()) }
  /// align to the center
  pub fn center() -> Self { Self(AnchorUnit::center()) }
  /// left of the parent
  pub fn before() -> Self { Self(AnchorUnit::before()) }
  /// right of the parent
  pub fn after() -> Self { Self(AnchorUnit::after()) }
  pub fn offset(self, offset: impl Into<Measure>) -> Self { Self(self.0.offset(offset)) }
  pub fn calculate(&self, reference: f32, this: f32) -> f32 { self.0.calculate(reference, this) }
}

impl Lerp for AnchorX {
  fn lerp(&self, to: &Self, factor: f32) -> Self { AnchorX(self.0.lerp(&to.0, factor)) }
}

#[derive(Default, PartialEq, Clone, Debug)]
pub struct AnchorY(AnchorUnit);

impl AnchorY {
  pub fn new(v: impl Into<Measure>) -> Self { Self(AnchorUnit::start().offset(v)) }
  pub fn percent(v: f32) -> Self { Self(AnchorUnit::start().offset(Measure::Unit(v))) }
  /// align to the top
  pub fn top() -> Self { Self(AnchorUnit::start()) }
  /// align to the bottom
  pub fn bottom() -> Self { Self(AnchorUnit::end()) }
  /// align to the center
  pub fn center() -> Self { Self(AnchorUnit::center()) }
  /// above the parent
  pub fn above() -> Self { Self(AnchorUnit::before()) }
  /// below the parent
  pub fn under() -> Self { Self(AnchorUnit::after()) }
  pub fn offset(self, offset: impl Into<Measure>) -> Self { Self(self.0.offset(offset)) }
  pub fn calculate(&self, reference: f32, this: f32) -> f32 { self.0.calculate(reference, this) }
}

impl Lerp for AnchorY {
  fn lerp(&self, to: &Self, factor: f32) -> Self { AnchorY(self.0.lerp(&to.0, factor)) }
}

#[derive(Default, PartialEq, Clone)]
pub struct Anchor {
  pub x: Option<AnchorX>,
  pub y: Option<AnchorY>,
}

impl Anchor {
  pub fn new(x: impl Into<Measure>, y: impl Into<Measure>) -> Self {
    Self { x: Some(AnchorX::left().offset(x)), y: Some(AnchorY::top().offset(y)) }
  }

  pub fn center() -> Self { Self { x: Some(AnchorX::center()), y: Some(AnchorY::center()) } }

  pub fn left(x: impl Into<Measure>) -> Self {
    Self { x: Some(AnchorX::left().offset(x)), y: None }
  }

  pub fn right(x: impl Into<Measure>) -> Self {
    Self { x: Some(AnchorX::right().offset(x)), y: None }
  }

  pub fn top(y: impl Into<Measure>) -> Self { Self { x: None, y: Some(AnchorY::top().offset(y)) } }

  pub fn bottom(y: impl Into<Measure>) -> Self {
    Self { x: None, y: Some(AnchorY::bottom().offset(y)) }
  }

  pub fn left_top(x: impl Into<Measure>, y: impl Into<Measure>) -> Self {
    Self { x: Some(AnchorX::left().offset(x)), y: Some(AnchorY::top().offset(y)) }
  }

  pub fn right_top(x: impl Into<Measure>, y: impl Into<Measure>) -> Self {
    Self { x: Some(AnchorX::right().offset(x)), y: Some(AnchorY::top().offset(y)) }
  }

  pub fn left_bottom(x: impl Into<Measure>, y: impl Into<Measure>) -> Self {
    Self { x: Some(AnchorX::left().offset(x)), y: Some(AnchorY::bottom().offset(y)) }
  }

  pub fn right_bottom(x: impl Into<Measure>, y: impl Into<Measure>) -> Self {
    Self { x: Some(AnchorX::right().offset(x)), y: Some(AnchorY::bottom().offset(y)) }
  }

  pub fn from_point(p: Point) -> Self {
    Self { x: Some(AnchorX::new(p.x)), y: Some(AnchorY::new(p.y)) }
  }

  pub fn calculate(&self, reference: Size, this: Size) -> Point {
    let x = self
      .x
      .as_ref()
      .map(|x| x.calculate(reference.width, this.width))
      .unwrap_or(0.);
    let y = self
      .y
      .as_ref()
      .map(|y| y.calculate(reference.height, this.height))
      .unwrap_or(0.);
    Point::new(x, y)
  }
}

impl Lerp for Anchor {
  fn lerp(&self, to: &Self, factor: f32) -> Self {
    Anchor { x: self.x.lerp(&to.x, factor), y: self.y.lerp(&to.y, factor) }
  }
}

impl From<f32> for AnchorX {
  fn from(v: f32) -> Self { AnchorX::new(v) }
}

impl From<Measure> for AnchorX {
  fn from(m: Measure) -> Self { AnchorX::default().offset(m) }
}

impl From<f32> for AnchorY {
  fn from(v: f32) -> Self { AnchorY::new(v) }
}

impl From<Measure> for AnchorY {
  fn from(m: Measure) -> Self { AnchorY::default().offset(m) }
}

// RFrom impls for Option<AnchorX> and Option<AnchorY> - allows f32 and Measure
// to be used directly with with_x()/with_y().
// We use RFrom instead of From to bypass Rust's orphan rules (since RFrom is
// local). We implement for specific types (f32, Measure) rather than generic T:
// Into<Anchor*> to avoid ambiguity with IntoKind when AnchorX/AnchorY is passed
// directly.
pub struct OptionAnchorKind;

impl RFrom<f32, OptionAnchorKind> for Option<AnchorX> {
  fn r_from(v: f32) -> Self { Some(AnchorX::from(v)) }
}

impl RFrom<Measure, OptionAnchorKind> for Option<AnchorX> {
  fn r_from(v: Measure) -> Self { Some(AnchorX::from(v)) }
}

impl RFrom<f32, OptionAnchorKind> for Option<AnchorY> {
  fn r_from(v: f32) -> Self { Some(AnchorY::from(v)) }
}

impl RFrom<Measure, OptionAnchorKind> for Option<AnchorY> {
  fn r_from(v: Measure) -> Self { Some(AnchorY::from(v)) }
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
  /// The position of the widget relative to its parent.
  pub pos: Point,
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

  /// Calculate the position of widget `id` given the parent size.
  /// This performs lazy position calculation based on stored AnchorX/AnchorY
  /// rules.
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
    fn measure(&self, mut clamp: BoxClamp, ctx: &mut MeasureCtx) -> Size {
      clamp.max = clamp.max.min(self.size);
      let child = ctx.assert_single_child();
      ctx.layout_child(child, clamp);
      self.size
    }

    fn place_children(&self, _size: Size, ctx: &mut PlaceCtx) {
      let child = ctx.assert_single_child();
      ctx.update_position(child, self.offset);
    }

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
        on_performed_layout: move |_| *$write(w_layout_cnt) += 1,
        @ {
          pipe!($read(child_box).size.is_empty())
            .map(move|b| {
              let child_box = child_box.clone_writer();
              fn_widget! {
                if b {
                  MockBox { size: Size::new(1., 1.) }.into_widget()
                } else {
                  child_box.into_widget()
                }
              }
            })
        }
      }
    };

    let wnd = TestWindow::from_widget(w);
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
        size: pipe!(*$read(size)),
        on_performed_layout: move |_| $write(order).push(1),
        @MockBox {
          size: pipe!(*$read(size)),
          on_performed_layout: move |_| $write(order).push(2),
          @MockBox {
            size: pipe!(*$read(size)),
            on_performed_layout: move |_| $write(order).push(3),
          }
        }
      }
    };

    let wnd = TestWindow::from_widget(w);
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
          @MockBox { size: pipe!(*$read(size)) }
        }
      }
    };

    #[track_caller]
    fn assert_rect_by_path(wnd: &TestWindow, path: &[usize], rect: Rect) {
      let id = wnd.widget_id_by_path(path);
      let pos = wnd.widget_pos(id).unwrap();
      assert_eq!(pos, rect.origin);
      let info = wnd.layout_info_by_path(path).unwrap();
      assert_eq!(info.size.unwrap(), rect.size);
    }

    let wnd = TestWindow::from_widget(w);
    wnd.draw_frame();
    assert_rect_by_path(&wnd, &[0, 0], ribir_types::rect(50., 50., 50., 50.));
    assert_rect_by_path(&wnd, &[0, 0, 0], ribir_types::rect(0., 0., 0., 0.));

    {
      *trigger.write() = Size::new(10., 10.);
    }

    wnd.draw_frame();
    assert_rect_by_path(&wnd, &[0, 0], ribir_types::rect(50., 50., 50., 50.));
    assert_rect_by_path(&wnd, &[0, 0, 0], ribir_types::rect(0., 0., 10., 10.));
  }

  #[test]
  fn relayout_from_parent() {
    reset_test_env!();

    let (cnt, w_cnt) = split_value(0);
    let (size, w_size) = split_value(Size::zero());
    let w = fn_widget! {
      @MockBox {
        size: Size::new(50., 50.),
        on_performed_layout: move |_| *$write(w_cnt) += 1,
        @MockBox { size: pipe!(*$read(size)) }
      }
    };

    let wnd = TestWindow::from_widget(w);
    wnd.draw_frame();
    assert_eq!(*cnt.read(), 1);

    *w_size.write() = Size::new(10., 10.);

    wnd.draw_frame();
    assert_eq!(*cnt.read(), 2);
  }

  #[test]
  fn anchor_unit_lerp_same_align() {
    // Same align type: lerp pixel and percent offsets independently
    let from = AnchorUnit { align: AlignType::Start, pixel_offset: 10., percent_offset: 0.2 };
    let to = AnchorUnit { align: AlignType::Start, pixel_offset: 50., percent_offset: 0.8 };
    let mid = from.lerp(&to, 0.5);
    assert_eq!(mid.align, AlignType::Start);
    assert!((mid.pixel_offset - 30.).abs() < f32::EPSILON);
    assert!((mid.percent_offset - 0.5).abs() < f32::EPSILON);
  }

  #[test]
  fn anchor_unit_lerp_different_align_snaps() {
    // Different align type: snap to target
    let from = AnchorUnit { align: AlignType::Start, pixel_offset: 10., percent_offset: 0. };
    let to = AnchorUnit { align: AlignType::End, pixel_offset: 20., percent_offset: 0. };
    let result = from.lerp(&to, 0.5);
    assert_eq!(result, to);
  }

  #[test]
  fn anchor_x_lerp() {
    let from = AnchorX::left().offset(10.);
    let to = AnchorX::left().offset(50.);
    let mid = from.lerp(&to, 0.5);
    // calculate with reference=0, this=0 to extract pixel offset
    assert!((mid.calculate(0., 0.) - 30.).abs() < f32::EPSILON);
  }

  #[test]
  fn anchor_y_lerp() {
    let from = AnchorY::top().offset(0.);
    let to = AnchorY::top().offset(100.);
    let mid = from.lerp(&to, 0.25);
    assert!((mid.calculate(0., 0.) - 25.).abs() < f32::EPSILON);
  }

  #[test]
  fn anchor_lerp_both_axes() {
    let from = Anchor::left_top(10., 20.);
    let to = Anchor::left_top(50., 80.);
    let mid = from.lerp(&to, 0.5);
    let pos = mid.calculate(Size::zero(), Size::zero());
    assert!((pos.x - 30.).abs() < f32::EPSILON);
    assert!((pos.y - 50.).abs() < f32::EPSILON);
  }

  #[test]
  fn anchor_lerp_none_to_some() {
    // None lerps from default (Start + 0px)
    let from = Anchor { x: None, y: None };
    let to = Anchor::left_top(40., 60.);
    let mid = from.lerp(&to, 0.5);
    let pos = mid.calculate(Size::zero(), Size::zero());
    assert!((pos.x - 20.).abs() < f32::EPSILON);
    assert!((pos.y - 30.).abs() < f32::EPSILON);
  }

  #[test]
  fn anchor_lerp_cross_align_snaps() {
    let from = Anchor::left(10.);
    let to = Anchor::right(20.);
    let mid = from.lerp(&to, 0.3);
    // x should snap to `right(20.)` since align types differ
    assert_eq!(mid.x, to.x);
  }
}
