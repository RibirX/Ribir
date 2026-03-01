use std::collections::HashMap;

use ribir_geom::ZERO_SIZE;

use super::{Lerp, WidgetId, WidgetTree};
use crate::prelude::{INFINITY_SIZE, Measure, Point, RFrom, Size};

/// boundary limit of the render object's layout
#[derive(Debug, Clone, PartialEq, Copy, Lerp)]
pub struct BoxClamp {
  pub min: Size,
  pub max: Size,
}

impl BoxClamp {
  pub const UNLIMITED: BoxClamp = BoxClamp { min: ZERO_SIZE, max: INFINITY_SIZE };
  /// Expand horizontally to fill available width
  pub const EXPAND_X: BoxClamp =
    BoxClamp { min: Size::new(f32::INFINITY, 0.), max: Size::new(f32::INFINITY, f32::INFINITY) };
  /// Expand vertically to fill available height
  pub const EXPAND_Y: BoxClamp =
    BoxClamp { min: Size::new(0., f32::INFINITY), max: Size::new(f32::INFINITY, f32::INFINITY) };
  /// Expand both horizontally and vertically to fill available space
  pub const EXPAND_BOTH: BoxClamp = BoxClamp { min: INFINITY_SIZE, max: INFINITY_SIZE };

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

  pub const fn max_height(height: f32) -> Self {
    Self { min: ZERO_SIZE, max: Size::new(f32::INFINITY, height) }
  }

  pub const fn max_width(width: f32) -> Self {
    Self { min: ZERO_SIZE, max: Size::new(width, f32::INFINITY) }
  }

  pub const fn with_min_size(mut self, size: Size) -> Self {
    self.min = Size::new(size.width.min(self.max.width), size.height.min(self.max.height));
    self
  }

  pub const fn with_max_size(mut self, size: Size) -> Self {
    self.max = Size::new(size.width.max(self.min.width), size.height.max(self.min.height));
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

  pub const fn with_max_width(mut self, width: f32) -> Self {
    self.max.width = width.max(self.min.width);
    self
  }

  pub const fn with_max_height(mut self, height: f32) -> Self {
    self.max.height = height.max(self.min.height);
    self
  }

  pub const fn with_min_width(mut self, width: f32) -> Self {
    self.min.width = width.min(self.max.width);
    self
  }

  pub const fn with_min_height(mut self, height: f32) -> Self {
    self.min.height = height.min(self.max.height);
    self
  }

  /// Calculates an estimated container width during child layout phases when
  /// parent width is unknown.
  ///
  /// Provides a deterministic strategy to determine container width based on
  /// constraints and child metrics:
  ///
  /// 1. **Constraint-based width**: Uses the maximum width constraint when
  ///    finite and bounded
  /// 2. **Content-based width**: Falls back to child width clamped by minimum
  ///    width constraint
  ///
  /// # Arguments
  ///
  /// - `child_width`: The child's intrinsic width requirement before clamping
  ///
  /// # Returns
  ///
  /// Estimated layout width that respects constraints while considering child
  /// requirements.
  ///
  /// # Implementation Notes
  ///
  /// The returned width represents a layout hypothesis rather than final parent
  /// dimensions. This intermediate value enables consistent layout calculations
  /// before parent constraints are fully resolved, but may differ from the
  /// final container width determined during parent layout phases.
  pub const fn container_width(&self, child_width: f32) -> f32 {
    let min = self.min.width;
    let max = self.max.width;

    // Prefer finite maximum constraint when available, otherwise use clamped child
    // width
    if max.is_finite() { max } else { min.max(child_width) }
  }

  /// Calculates an estimated container height during child layout phases when
  /// parent height is unknown.
  ///
  /// Provides a deterministic strategy to determine container height based on
  /// constraints and child metrics:
  ///
  /// 1. **Constraint-based height**: Uses the maximum height constraint when
  ///    finite and bounded
  /// 2. **Content-based height**: Falls back to child height clamped by minimum
  ///    height constraint
  ///
  /// # Arguments
  ///
  /// - `child_height`: The child's intrinsic height requirement before clamping
  ///
  /// # Returns
  ///
  /// Estimated layout height that respects constraints while considering child
  /// requirements.
  ///
  /// # Implementation Notes
  ///
  /// The returned height represents a layout hypothesis rather than final
  /// parent dimensions. This intermediate value enables consistent layout
  /// calculations before parent constraints are fully resolved, but may
  /// differ from the final container height determined during parent layout
  /// phases.
  pub const fn container_height(&self, child_height: f32) -> f32 {
    let min = self.min.height;
    let max = self.max.height;

    // Prefer finite maximum constraint when available, otherwise use clamped child
    // height
    if max.is_finite() { max } else { min.max(child_height) }
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

#[derive(Default, PartialEq, Clone)]
pub struct Anchor {
  pub x: Option<AnchorX>,
  pub y: Option<AnchorY>,
}

impl Anchor {
  pub fn new(x: impl Into<Measure>, y: impl Into<Measure>) -> Self {
    Self { x: Some(AnchorX::left().offset(x)), y: Some(AnchorY::top().offset(y)) }
  }

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

impl BoxClamp {
  /// Restricts a size to stay within the clamp's minimum and maximum bounds
  #[inline]
  pub fn clamp(self, size: Size) -> Size { size.clamp(self.min, self.max) }

  /// Creates a constraint that allows maximum expansion
  /// (sets maximum dimensions to infinity while preserving minimums)
  #[inline]
  pub fn expand(mut self) -> Self {
    self.max = INFINITY_SIZE;
    self
  }

  /// Creates a constraint with relaxed minimum requirements
  /// (sets minimum dimensions to zero while preserving maximums)
  #[inline]
  pub fn loose(mut self) -> Self {
    self.min = ZERO_SIZE;
    self
  }

  /// Removes horizontal constraints while preserving vertical bounds
  /// (width can be any value between 0 and infinity)
  pub fn free_width(mut self) -> Self {
    self.min.width = 0.0;
    self.max.width = f32::INFINITY;
    self
  }

  /// Removes vertical constraints while preserving horizontal bounds
  /// (height can be any value between 0 and infinity)
  pub fn free_height(mut self) -> Self {
    self.min.height = 0.0;
    self.max.height = f32::INFINITY;
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
}
