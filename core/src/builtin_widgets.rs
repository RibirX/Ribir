//! Built-in widgets is a set of minimal widgets that describes the most common
//! UI elements.
//!
//! The most of them can be used to extend other object in the
//! declare syntax, so other objects can use the builtin fields and methods like
//! self fields and methods.

mod painting_style;
use std::ops::DerefMut;
pub mod reuse_id;
pub use reuse_id::*;
pub mod widget_scope;
pub use painting_style::*;
mod text_align;
pub use text_align::*;
pub use widget_scope::*;
pub mod image_widget;
pub mod keep_alive;
pub use keep_alive::*;
pub mod backdrop_filter;
pub use backdrop_filter::*;
pub mod box_shadow;
pub use box_shadow::*;
mod theme;
use smallvec::SmallVec;
pub use theme::*;
mod cursor;
pub use cursor::*;
pub use winit::window::CursorIcon;
mod margin;
pub use margin::*;
mod foreground;
mod padding;
pub use foreground::*;
pub use padding::*;
mod scrollable;
pub use scrollable::*;
mod transform_widget;
pub use transform_widget::*;
mod visibility;
pub use visibility::*;
mod ignore_pointer;
pub use ignore_pointer::*;
mod void;
pub use void::Void;
mod unconstrained_box;
pub use unconstrained_box::*;
mod opacity;
pub use opacity::*;
mod anchor;
pub use anchor::*;
mod layout_box;
pub use layout_box::*;
pub mod align;
pub use align::*;
pub mod fitted_box;
pub use fitted_box::*;
pub mod svg;
pub use svg::*;
pub mod filter_widget;
pub use filter_widget::*;

pub mod disabled;
pub use disabled::*;

pub mod clip;
pub use clip::*;
pub mod clip_boundary;
pub use clip_boundary::*;
pub mod focus_scope;
pub use focus_scope::*;
pub mod global_anchor;
pub use global_anchor::*;
mod mix_builtin;
pub use mix_builtin::*;
pub mod container;
pub use container::*;
mod class;
pub use class::*;
mod constrained_box;
pub use constrained_box::*;
mod text_style;
pub use text_style::*;
mod smooth_layout;
pub use smooth_layout::*;

mod track_widget_id;
pub use track_widget_id::*;
mod text;
pub use text::*;
mod tooltips;
pub use tooltips::*;
mod providers;
pub use providers::*;
mod border;
pub use border::*;
mod radius;
pub use radius::*;
mod background;
pub use background::*;
pub mod location;
pub use location::*;

use crate::prelude::*;

/// A fat object that extend the `T` object with all builtin widgets ability.
///
/// A `FatObj` will create during the compose phase, and compose with the
/// builtin widgets it actually use, and drop after composed.
///
/// It's important to understand that `FatObj` is a temporary mixin object. It
/// doesn't persist in the final widget tree. Therefore, you can only clone a
/// portion of its real widget. However, if you're using the DSL macros, you
/// don't need to worry about this.
///
/// # Example
///
/// If you want to modify the margin of a `FatObj`, you need to clone the writer
/// of `Margin` widget within it.
///
/// ```rust
/// use ribir_core::{prelude::*, test_helper::*};
///
/// let w = || {
///   let mut multi = FatObj::new(MockMulti::default());
///   multi.with_margin(EdgeInsets::all(10.));
///
///   let margin = multi.margin();
///   multi.on_tap(move |_| *margin.write() = EdgeInsets::all(20.));
///   multi.into_widget()
/// };
/// ```
#[derive(Default)]
pub struct FatObj<T> {
  host: T,
  track_id: Option<Stateful<TrackWidgetId>>,
  class: Option<Stateful<Class>>,
  padding: Option<Stateful<Padding>>,
  fitted_box: Option<Stateful<FittedBox>>,
  constrained_box: Option<Stateful<ConstrainedBox>>,
  radius: Option<Stateful<RadiusWidget>>,
  border: Option<Stateful<BorderWidget>>,
  backdrop: Option<Stateful<BackdropFilter>>,
  filter: Option<Stateful<FilterWidget>>,
  box_shadow: Option<Stateful<BoxShadowWidget>>,
  background: Option<Stateful<Background>>,
  foreground: Option<Stateful<Foreground>>,
  scrollable: Option<Stateful<ScrollableWidget>>,
  layout_box: Option<Stateful<LayoutBox>>,
  mix_builtin: Option<MixBuiltin>,
  cursor: Option<Stateful<Cursor>>,
  margin: Option<Stateful<Margin>>,
  transform: Option<Stateful<TransformWidget>>,
  opacity: Option<Stateful<Opacity>>,
  visibility: Option<Stateful<Visibility>>,
  h_align: Option<Stateful<HAlignWidget>>,
  v_align: Option<Stateful<VAlignWidget>>,
  relative_anchor: Option<Stateful<RelativeAnchor>>,
  global_anchor: Option<Stateful<GlobalAnchor>>,
  painting_style: Option<Stateful<PaintingStyleWidget>>,
  text_align: Option<Stateful<TextAlignWidget>>,
  text_style: Option<Stateful<TextStyleWidget>>,
  keep_alive: Option<Stateful<KeepAlive>>,
  keep_alive_unsubscribe_handle: Option<Box<dyn Any>>,
  tooltips: Option<Stateful<Tooltips>>,
  disabled: Option<Stateful<Disabled>>,
  clip_boundary: Option<Stateful<ClipBoundary>>,
  providers: Option<SmallVec<[Provider; 1]>>,
  reuse: Option<Reuse>,
}

/// Create a function widget that uses an empty `FatObj` as the host object.
#[macro_export]
macro_rules! fat_obj {
  ($($t: tt)*) => {
    fn_widget! {
      let mut obj = FatObj::<()>::default();
      @(obj) { $($t)* }
    }
  };
}

impl<T> FatObj<T> {
  /// Create a new `FatObj` with the given host object.
  pub fn new(host: T) -> Self { FatObj::<()>::default().with_child(host) }

  /// Maps an `FatObj<T>` to `FatObj<V>` by applying a function to the host
  /// object.
  pub fn map<V>(self, f: impl FnOnce(T) -> V) -> FatObj<V> {
    FatObj {
      host: f(self.host),
      track_id: self.track_id,
      class: self.class,
      mix_builtin: self.mix_builtin,
      fitted_box: self.fitted_box,
      border: self.border,
      radius: self.radius,
      backdrop: self.backdrop,
      filter: self.filter,
      box_shadow: self.box_shadow,
      background: self.background,
      foreground: self.foreground,
      padding: self.padding,
      layout_box: self.layout_box,
      cursor: self.cursor,
      margin: self.margin,
      scrollable: self.scrollable,
      constrained_box: self.constrained_box,
      transform: self.transform,
      h_align: self.h_align,
      v_align: self.v_align,
      relative_anchor: self.relative_anchor,
      global_anchor: self.global_anchor,
      painting_style: self.painting_style,
      text_style: self.text_style,
      text_align: self.text_align,
      visibility: self.visibility,
      opacity: self.opacity,
      tooltips: self.tooltips,
      clip_boundary: self.clip_boundary,
      disabled: self.disabled,
      keep_alive: self.keep_alive,
      keep_alive_unsubscribe_handle: self.keep_alive_unsubscribe_handle,
      providers: self.providers,
      reuse: self.reuse,
    }
  }

  /// Splits the `FatObj` into its host object and the remaining shell.
  /// Returns a tuple containing both the extracted host and the shell object.
  pub fn into_parts(self) -> (T, FatObj<()>) {
    let mut host = None;
    let fat = self.map(|old| host = Some(old));
    (host.expect("Host value should be set"), fat)
  }

  /// Return true if the FatObj not contains any builtin widgets.
  pub fn is_empty(&self) -> bool {
    self.track_id.is_none()
      && self.mix_builtin.is_none()
      && self.fitted_box.is_none()
      && self.border.is_none()
      && self.radius.is_none()
      && self.backdrop.is_none()
      && self.filter.is_none()
      && self.background.is_none()
      && self.foreground.is_none()
      && self.padding.is_none()
      && self.layout_box.is_none()
      && self.cursor.is_none()
      && self.margin.is_none()
      && self.scrollable.is_none()
      && self.constrained_box.is_none()
      && self.transform.is_none()
      && self.h_align.is_none()
      && self.v_align.is_none()
      && self.relative_anchor.is_none()
      && self.global_anchor.is_none()
      && self.class.is_none()
      && self.painting_style.is_none()
      && self.text_style.is_none()
      && self.visibility.is_none()
      && self.opacity.is_none()
      && self.keep_alive.is_none()
      && self.tooltips.is_none()
      && self.disabled.is_none()
      && self.clip_boundary.is_none()
      && self.reuse.is_none()
  }

  /// Return the host object of the FatObj.
  ///
  /// # Panics
  ///
  /// Panics if the FatObj contains builtin widgets.
  pub fn into_inner(self) -> T {
    assert!(self.is_empty(), "Unwrap a FatObj with contains builtin widgets is not allowed.");
    self.host
  }
}

macro_rules! on_mixin {
  ($this:ident, $on_method:ident, $f:ident) => {{
    $this.mix_builtin_widget().$on_method($f);
    $this
  }};
}

// report all builtin widgets apis
impl<T> FatObj<T> {
  /// Attaches an event handler to the widget. It's triggered when any event or
  /// lifecycle change happens.
  pub fn on_event(&mut self, f: impl FnMut(&mut Event) + 'static) -> &mut Self {
    on_mixin!(self, on_event, f)
  }

  /// Attaches an event handler that runs when the widget is first mounted to
  /// the tree.
  pub fn on_mounted(&mut self, f: impl FnOnce(&mut LifecycleEvent) + 'static) -> &mut Self {
    on_mixin!(self, on_mounted, f)
  }

  /// Attaches an event handler that runs after the widget is performed layout.
  pub fn on_performed_layout(&mut self, f: impl FnMut(&mut LifecycleEvent) + 'static) -> &mut Self {
    on_mixin!(self, on_performed_layout, f)
  }

  /// Attaches an event handler that runs when the widget is disposed.
  pub fn on_disposed(&mut self, f: impl FnOnce(&mut LifecycleEvent) + 'static) -> &mut Self {
    on_mixin!(self, on_disposed, f)
  }

  /// Attaches a handler to the widget that is triggered when a pointer down
  /// occurs.
  pub fn on_pointer_down(&mut self, f: impl FnMut(&mut PointerEvent) + 'static) -> &mut Self {
    on_mixin!(self, on_pointer_down, f)
  }

  /// Attaches a handler to the widget that is triggered during the capture
  /// phase of a pointer down event. This is similar to `on_pointer_down`, but
  /// it's triggered earlier in the event flow. For more information on event
  /// capturing, see [Event capture](https://www.w3.org/TR/DOM-Level-2-Events/events.html#Events-flow-capture).
  pub fn on_pointer_down_capture(
    &mut self, f: impl FnMut(&mut PointerEvent) + 'static,
  ) -> &mut Self {
    on_mixin!(self, on_pointer_down_capture, f)
  }

  /// Attaches a handler to the widget that is triggered when a pointer up
  /// occurs.
  pub fn on_pointer_up(&mut self, f: impl FnMut(&mut PointerEvent) + 'static) -> &mut Self {
    on_mixin!(self, on_pointer_up, f)
  }

  /// Attaches a handler to the widget that is triggered during the capture
  /// phase of a pointer up event. This is similar to `on_pointer_up`, but it's
  /// triggered earlier in the event flow. For more information on event
  /// capturing, see [Event capture](https://www.w3.org/TR/DOM-Level-2-Events/events.html#Events-flow-capture).
  pub fn on_pointer_up_capture(&mut self, f: impl FnMut(&mut PointerEvent) + 'static) -> &mut Self {
    on_mixin!(self, on_pointer_up_capture, f)
  }

  /// Attaches a handler to the widget that is triggered when a pointer move
  /// occurs.
  pub fn on_pointer_move(&mut self, f: impl FnMut(&mut PointerEvent) + 'static) -> &mut Self {
    on_mixin!(self, on_pointer_move, f)
  }

  /// Attaches a handler to the widget that is triggered during the capture
  /// phase of a pointer move event. This is similar to `on_pointer_move`, but
  /// it's triggered earlier in the event flow. For more information on event
  /// capturing, see [Event capture](https://www.w3.org/TR/DOM-Level-2-Events/events.html#Events-flow-capture).
  pub fn on_pointer_move_capture(
    &mut self, f: impl FnMut(&mut PointerEvent) + 'static,
  ) -> &mut Self {
    on_mixin!(self, on_pointer_move_capture, f)
  }

  /// Attaches a handler to the widget that is triggered when a pointer event
  /// cancels.
  pub fn on_pointer_cancel(&mut self, f: impl FnMut(&mut PointerEvent) + 'static) -> &mut Self {
    on_mixin!(self, on_pointer_cancel, f)
  }

  /// Attaches a handler to the widget that is triggered when a pointer device
  /// is moved into the hit test boundaries of an widget or one of its
  /// descendants.
  pub fn on_pointer_enter(&mut self, f: impl FnMut(&mut PointerEvent) + 'static) -> &mut Self {
    on_mixin!(self, on_pointer_enter, f)
  }

  /// Attaches a handler to the widget that is triggered when a pointer device
  /// is moved out of the hit test boundaries of an widget or one of its
  /// descendants.
  pub fn on_pointer_leave(&mut self, f: impl FnMut(&mut PointerEvent) + 'static) -> &mut Self {
    on_mixin!(self, on_pointer_leave, f)
  }

  /// Attaches a handler to the widget that is triggered when a tap(click)
  /// occurs.
  pub fn on_tap(&mut self, f: impl FnMut(&mut PointerEvent) + 'static) -> &mut Self {
    on_mixin!(self, on_tap, f)
  }

  /// Attaches a handler to the widget that is triggered during the capture
  /// phase of a tap event. This is similar to `on_tap`, but it's triggered
  /// earlier in the event flow. For more information on event capturing, see
  /// [Event capture](https://www.w3.org/TR/DOM-Level-2-Events/events.html#Events-flow-capture).
  pub fn on_tap_capture(&mut self, f: impl FnMut(&mut PointerEvent) + 'static) -> &mut Self {
    on_mixin!(self, on_tap_capture, f)
  }

  /// Attaches a handler to the widget that is triggered when a double tap
  /// occurs.
  pub fn on_double_tap(&mut self, f: impl FnMut(&mut PointerEvent) + 'static) -> &mut Self {
    on_mixin!(self, on_double_tap, f)
  }

  /// Attaches a handler to the widget that is triggered during the capture
  /// phase of a double tap event. This is similar to `on_double_tap`, but it's
  /// triggered earlier in the event flow. For more information on event
  /// capturing, see [Event capture](https://www.w3.org/TR/DOM-Level-2-Events/events.html#Events-flow-capture).
  pub fn on_double_tap_capture(&mut self, f: impl FnMut(&mut PointerEvent) + 'static) -> &mut Self {
    on_mixin!(self, on_double_tap_capture, f)
  }

  /// Attaches a handler to the widget that is triggered when a triple tap
  /// occurs.
  pub fn on_triple_tap(&mut self, f: impl FnMut(&mut PointerEvent) + 'static) -> &mut Self {
    on_mixin!(self, on_triple_tap, f)
  }

  /// Attaches a handler to the widget that is triggered when a triple tap
  /// occurs. This is similar to `on_double_tap`, but it's triggered earlier
  /// in the event flow. For more information on event capturing, see
  /// [Event capture](https://www.w3.org/TR/DOM-Level-2-Events/events.html#Events-flow-capture).
  pub fn on_triple_tap_capture(&mut self, f: impl FnMut(&mut PointerEvent) + 'static) -> &mut Self {
    on_mixin!(self, on_triple_tap_capture, f)
  }

  /// Attaches a handler to the widget that is triggered when a x-times tap
  /// occurs.
  pub fn on_x_times_tap(
    &mut self, (times, f): (usize, impl FnMut(&mut PointerEvent) + 'static),
  ) -> &mut Self {
    self
      .mix_builtin_widget()
      .on_x_times_tap((times, f));
    self
  }

  /// Attaches a handler to the widget that is triggered during the capture
  /// phase of a x-times tap event. This is similar to `on_x_times_tap`, but
  /// it's triggered earlier in the event flow. For more information on event
  /// capturing, see [Event capture](https://www.w3.org/TR/DOM-Level-2-Events/events.html#Events-flow-capture).
  pub fn on_x_times_tap_capture(
    &mut self, (times, f): (usize, impl FnMut(&mut PointerEvent) + 'static),
  ) -> &mut Self {
    self
      .mix_builtin_widget()
      .on_x_times_tap_capture((times, f));
    self
  }

  /// Attaches a handler to the widget that is triggered when the user rotates a
  /// wheel button on a pointing device (typically a mouse).
  pub fn on_wheel(&mut self, f: impl FnMut(&mut WheelEvent) + 'static) -> &mut Self {
    on_mixin!(self, on_wheel, f)
  }

  /// Attaches a handler to the widget that is triggered during the capture
  /// phase of a wheel event. This is similar to `on_wheel`, but it's triggered
  /// earlier in the event flow. For more information on event capturing, see
  /// [Event capture](https://www.w3.org/TR/DOM-Level-2-Events/events.html#Events-flow-capture).
  pub fn on_wheel_capture(&mut self, f: impl FnMut(&mut WheelEvent) + 'static) -> &mut Self {
    on_mixin!(self, on_wheel_capture, f)
  }

  /// Attaches a handler to the widget that is triggered when the input method
  /// pre-edit area is changed.
  pub fn on_ime_pre_edit(&mut self, f: impl FnMut(&mut ImePreEditEvent) + 'static) -> &mut Self {
    on_mixin!(self, on_ime_pre_edit, f)
  }

  /// Attaches a handler to the widget that is triggered during the capture
  /// phase of a ime pre-edit event. This is similar to `on_ime_pre_edit`,
  /// but it's triggered earlier in the event flow. For more information on
  /// event capturing, see [Event capture](https://www.w3.org/TR/DOM-Level-2-Events/events.html#Events-flow-capture).
  pub fn on_ime_pre_edit_capture(
    &mut self, f: impl FnMut(&mut ImePreEditEvent) + 'static,
  ) -> &mut Self {
    on_mixin!(self, on_ime_pre_edit_capture, f)
  }

  /// Attaches a handler to the widget that is triggered when the input method
  /// commits text or keyboard pressed the text key.
  pub fn on_chars(&mut self, f: impl FnMut(&mut CharsEvent) + 'static) -> &mut Self {
    on_mixin!(self, on_chars, f)
  }

  /// Attaches a handler to the widget that is triggered during the capture
  /// phase of a chars event. This is similar to `on_chars`, but it's triggered
  /// earlier in the event flow. For more information on event capturing,
  /// see [Event capture](https://www.w3.org/TR/DOM-Level-2-Events/events.html#Events-flow-capture).
  pub fn on_chars_capture(&mut self, f: impl FnMut(&mut CharsEvent) + 'static) -> &mut Self {
    on_mixin!(self, on_chars_capture, f)
  }

  /// Attaches a handler to the widget that is triggered when the keyboard key
  /// is pressed.
  pub fn on_key_down(&mut self, f: impl FnMut(&mut KeyboardEvent) + 'static) -> &mut Self {
    on_mixin!(self, on_key_down, f)
  }

  /// Attaches a handler to the widget that is triggered during the capture
  /// phase of a key down event. This is similar to `on_key_down`, but it's
  /// triggered earlier in the event flow. For more information on event
  /// capturing, see [Event capture](https://www.w3.org/TR/DOM-Level-2-Events/events.html#Events-flow-capture).
  pub fn on_key_down_capture(&mut self, f: impl FnMut(&mut KeyboardEvent) + 'static) -> &mut Self {
    on_mixin!(self, on_key_down_capture, f)
  }

  /// Attaches a handler to the widget that is triggered when the keyboard key
  /// is released.
  pub fn on_key_up(&mut self, f: impl FnMut(&mut KeyboardEvent) + 'static) -> &mut Self {
    on_mixin!(self, on_key_up, f)
  }

  /// Attaches a handler to the widget that is triggered during the capture
  /// phase of a key up event. This is similar to `on_key_up`, but it's
  /// triggered earlier in the event flow. For more information on event
  /// capturing, see [Event capture](https://www.w3.org/TR/DOM-Level-2-Events/events.html#Events-flow-capture).
  pub fn on_key_up_capture(&mut self, f: impl FnMut(&mut KeyboardEvent) + 'static) -> &mut Self {
    on_mixin!(self, on_key_up_capture, f)
  }

  /// Attaches a handler to the widget that is triggered when the widget is
  /// focused.
  pub fn on_focus(&mut self, f: impl FnMut(&mut FocusEvent) + 'static) -> &mut Self {
    on_mixin!(self, on_focus, f)
  }

  /// Attaches a handler to the widget that is triggered when the widget is lost
  /// focus.
  pub fn on_blur(&mut self, f: impl FnMut(&mut FocusEvent) + 'static) -> &mut Self {
    on_mixin!(self, on_blur, f)
  }

  /// Attaches a handler to the widget that is triggered when the widget or its
  /// descendants are focused. The main difference between this event and focus
  /// is that focusin bubbles while focus does not.
  pub fn on_focus_in(&mut self, f: impl FnMut(&mut FocusEvent) + 'static) -> &mut Self {
    on_mixin!(self, on_focus_in, f)
  }

  /// Attaches a handler to the widget that is triggered during the capture
  /// phase of a focus in event. This is similar to `on_focus_in`, but it's
  /// triggered earlier in the event flow. For more information on event
  /// capturing, see [Event capture](https://www.w3.org/TR/DOM-Level-2-Events/events.html#Events-flow-capture).
  pub fn on_focus_in_capture(&mut self, f: impl FnMut(&mut FocusEvent) + 'static) -> &mut Self {
    on_mixin!(self, on_focus_in_capture, f)
  }

  /// Attaches a handler to the widget that is triggered when the widget or its
  /// descendants are lost focus. The main difference between this event and
  /// focusout is that focusout bubbles while blur does not.
  pub fn on_focus_out(&mut self, f: impl FnMut(&mut FocusEvent) + 'static) -> &mut Self {
    on_mixin!(self, on_focus_out, f)
  }

  /// Attaches a handler to the specific custom event that is bubbled from the
  /// descendants.
  pub fn on_custom_concrete_event<E: 'static>(
    &mut self, f: impl FnMut(&mut CustomEvent<E>) + 'static,
  ) -> &mut Self {
    on_mixin!(self, on_custom_concrete_event, f)
  }

  /// Attaches a handler to raw custom event that is bubbled from the
  /// descendants.
  pub fn on_custom_event(&mut self, f: impl FnMut(&mut RawCustomEvent) + 'static) -> &mut Self {
    on_mixin!(self, on_custom_event, f)
  }

  /// Attaches a handler to the widget that is triggered during the capture
  /// phase of a focus out event. This is similar to `on_focus_out`, but it's
  /// triggered earlier in the event flow. For more information on event
  /// capturing, see [Event capture](https://www.w3.org/TR/DOM-Level-2-Events/events.html#Events-flow-capture).
  pub fn on_focus_out_capture(&mut self, f: impl FnMut(&mut FocusEvent) + 'static) -> &mut Self {
    on_mixin!(self, on_focus_out_capture, f)
  }

  /// Initializes the widget with a tab index. The tab index is used to
  /// allow or prevent widgets from being sequentially focusable(usually with
  /// the Tab key, hence the name) and determine their relative ordering for
  /// sequential focus navigation. It accepts an integer as a value, with
  /// different results depending on the integer's value:
  /// - A negative value (usually -1) means that the widget is not reachable via
  ///   sequential keyboard navigation, but could be focused with API or
  ///   visually by clicking with the mouse.
  /// - Zero means that the element should be focusable in sequential keyboard
  ///   navigation, after any positive tab_index values and its order is defined
  ///   by the tree's source order.
  /// - A positive value means the element should be focusable in sequential
  ///   keyboard navigation, with its order defined by the value of the number.
  ///   That is, tab_index=4 is focused before tab_index=5 and tab_index=0, but
  ///   after tab_index=3. If multiple elements share the same positive
  ///   tab_index value, their order relative to each other follows their
  ///   position in the tree source. The maximum value for tab_index is 32767.
  ///   If not specified, it takes the default value 0.
  pub fn with_tab_index<K: ?Sized>(&mut self, tab_idx: impl RInto<PipeValue<i16>, K>) -> &mut Self {
    let focus = &self.focus_handle().flags;
    self
      .mix_builtin_widget()
      .init_sub_widget(tab_idx, focus, |m, v| m.set_tab_index(v));
    self
  }

  /// Initializes the `Class` that should be applied to the widget.
  pub fn with_class<K: ?Sized>(
    &mut self, cls: impl RInto<PipeValue<Option<ClassName>>, K>,
  ) -> &mut Self {
    init_sub_widget!(self, class, class, cls)
  }

  /// Initializes whether the `widget` should automatically get focus when the
  /// window loads.
  ///
  /// Only one widget should have this attribute specified.  If there are
  /// several, the widget nearest the root, get the initial focus.
  pub fn with_auto_focus<K: ?Sized>(&mut self, v: impl RInto<PipeValue<bool>, K>) -> &mut Self {
    let focus = &self.focus_handle().flags;
    self
      .mix_builtin_widget()
      .init_sub_widget(v, focus, |m, v| m.set_auto_focus(v));
    self
  }

  /// Initializes how its child should be scale to fit its box.
  pub fn with_box_fit<K: ?Sized>(&mut self, v: impl RInto<PipeValue<BoxFit>, K>) -> &mut Self {
    init_sub_widget!(self, fitted_box, box_fit, v)
  }

  /// Provide a painting style to this widget.
  pub fn with_painting_style<K: ?Sized>(
    &mut self, v: impl RInto<PipeValue<PaintingStyle>, K>,
  ) -> &mut Self {
    init_sub_widget!(self, painting_style, painting_style, v)
  }

  pub fn with_text_align<K: ?Sized>(
    &mut self, v: impl RInto<PipeValue<TextAlign>, K>,
  ) -> &mut Self {
    init_sub_widget!(self, text_align, text_align, v)
  }

  /// Initializes the text style of this widget.
  pub fn with_text_style<K: ?Sized>(
    &mut self, v: impl RInto<PipeValue<TextStyle>, K>,
  ) -> &mut Self {
    let mix = self
      .mix_builtin
      .get_or_insert_with(MixBuiltin::default);
    let text_style = self
      .text_style
      .get_or_insert_with(|| Stateful::new(TextStyleWidget::inherit_widget()));

    mix.init_sub_widget(v, text_style, move |widget, v| widget.text_style = v);
    self
  }

  /// Initializes the font size of this widget.
  pub fn with_font_size<K: ?Sized>(&mut self, v: impl RInto<PipeValue<f32>, K>) -> &mut Self {
    init_text_style!(self, font_size, v)
  }

  /// Initializes the font face of this widget.
  pub fn with_font_face<K: ?Sized>(&mut self, v: impl RInto<PipeValue<FontFace>, K>) -> &mut Self {
    init_text_style!(self, font_face, v)
  }

  /// Initializes the letter space of this widget.
  pub fn with_letter_spacing<K: ?Sized>(&mut self, v: impl RInto<PipeValue<f32>, K>) -> &mut Self {
    init_text_style!(self, letter_space, v)
  }

  /// Initializes the text line height of this widget.
  pub fn with_text_line_height<K: ?Sized>(
    &mut self, v: impl RInto<PipeValue<f32>, K>,
  ) -> &mut Self {
    init_text_style!(self, line_height, v)
  }

  /// Initializes the text overflow of this widget.
  pub fn with_text_overflow<K: ?Sized>(
    &mut self, v: impl RInto<PipeValue<TextOverflow>, K>,
  ) -> &mut Self {
    init_text_style!(self, overflow, v)
  }

  /// Initializes the backdrop filter of the widget.
  pub fn with_backdrop_filter<K: ?Sized>(
    &mut self, v: impl RInto<PipeValue<Filter>, K>,
  ) -> &mut Self {
    init_sub_widget!(self, backdrop, filter, v)
  }

  /// Initializes the filter of the widget.
  pub fn with_filter<K: ?Sized>(&mut self, v: impl RInto<PipeValue<Filter>, K>) -> &mut Self {
    init_sub_widget!(self, filter, filter, v)
  }

  /// Initializes the box shadow of the widget.
  pub fn with_box_shadow<K: ?Sized>(
    &mut self, v: impl RInto<PipeValue<BoxShadow>, K>,
  ) -> &mut Self {
    init_sub_widget!(self, box_shadow, box_shadow, v)
  }

  /// Initializes the background of the widget.
  pub fn with_background<K: ?Sized>(&mut self, v: impl RInto<PipeValue<Brush>, K>) -> &mut Self {
    init_sub_widget!(self, background, background, v)
  }

  /// Initializes the foreground of the widget.
  pub fn with_foreground<K: ?Sized>(&mut self, v: impl RInto<PipeValue<Brush>, K>) -> &mut Self {
    init_sub_widget!(self, foreground, foreground, v)
  }

  /// Initializes the border of the widget.
  pub fn with_border<K: ?Sized>(&mut self, v: impl RInto<PipeValue<Border>, K>) -> &mut Self {
    init_sub_widget!(self, border, border, v)
  }

  /// Initializes the border radius of the widget.
  pub fn with_radius<K: ?Sized>(&mut self, v: impl RInto<PipeValue<Radius>, K>) -> &mut Self {
    init_sub_widget!(self, radius, radius, v)
  }

  /// Initializes the extra space within the widget.
  pub fn with_padding<K: ?Sized>(&mut self, v: impl RInto<PipeValue<EdgeInsets>, K>) -> &mut Self {
    init_sub_widget!(self, padding, padding, v)
  }

  /// Initializes the cursor of the widget.
  pub fn with_cursor<K: ?Sized>(&mut self, v: impl RInto<PipeValue<CursorIcon>, K>) -> &mut Self {
    init_sub_widget!(self, cursor, cursor, v)
  }

  /// Initializes the space around the widget.
  pub fn with_margin<K: ?Sized>(&mut self, v: impl RInto<PipeValue<EdgeInsets>, K>) -> &mut Self {
    init_sub_widget!(self, margin, margin, v)
  }

  /// Initializes the constraints clamp of the widget.
  pub fn with_clamp<K: ?Sized>(&mut self, v: impl RInto<PipeValue<BoxClamp>, K>) -> &mut Self {
    init_sub_widget!(self, constrained_box, clamp, v)
  }

  /// Initializes how user can scroll the widget.
  pub fn with_scrollable<K: ?Sized>(
    &mut self, v: impl RInto<PipeValue<Scrollable>, K>,
  ) -> &mut Self {
    init_sub_widget!(self, scrollable, scrollable, v)
  }

  /// Initializes the transformation of the widget.
  pub fn with_transform<K: ?Sized>(&mut self, v: impl RInto<PipeValue<Transform>, K>) -> &mut Self {
    init_sub_widget!(self, transform, transform, v)
  }

  /// Initializes how the widget should be aligned horizontally.
  pub fn with_h_align<K: ?Sized>(&mut self, v: impl RInto<PipeValue<HAlign>, K>) -> &mut Self {
    init_sub_widget!(self, h_align, h_align, v)
  }

  /// Initializes how the widget should be aligned vertically.
  pub fn with_v_align<K: ?Sized>(&mut self, v: impl RInto<PipeValue<VAlign>, K>) -> &mut Self {
    init_sub_widget!(self, v_align, v_align, v)
  }

  /// Initializes the relative anchor to the parent of the widget.
  pub fn with_anchor<K: ?Sized>(&mut self, v: impl RInto<PipeValue<Anchor>, K>) -> &mut Self {
    init_sub_widget!(self, relative_anchor, anchor, v)
  }

  /// Initializes the horizontal global anchor of the widget.
  pub fn with_global_anchor_x<K: ?Sized>(
    &mut self, v: impl RInto<PipeValue<Option<GlobalAnchorX>>, K>,
  ) -> &mut Self {
    init_sub_widget!(self, global_anchor, global_anchor_x, v)
  }

  /// Initializes the vertical global anchor of the widget.
  pub fn with_global_anchor_y<K: ?Sized>(
    &mut self, v: impl RInto<PipeValue<Option<GlobalAnchorY>>, K>,
  ) -> &mut Self {
    init_sub_widget!(self, global_anchor, global_anchor_y, v)
  }

  /// Initializes the visibility of the widget.
  pub fn with_visible<K: ?Sized>(&mut self, v: impl RInto<PipeValue<bool>, K>) -> &mut Self {
    init_sub_widget!(self, visibility, visible, v)
  }

  /// Initializes the opacity of the widget.
  pub fn with_opacity<K: ?Sized>(&mut self, v: impl RInto<PipeValue<f32>, K>) -> &mut Self {
    init_sub_widget!(self, opacity, opacity, v)
  }

  /// Initializes the tooltips of the widget.
  pub fn with_tooltips<K: ?Sized>(
    &mut self, v: impl RInto<PipeValue<CowArc<str>>, K>,
  ) -> &mut Self {
    init_sub_widget!(self, tooltips, tooltips, v)
  }

  /// Initializes the disabled state of the widget.
  pub fn with_disabled<K: ?Sized>(&mut self, v: impl RInto<PipeValue<bool>, K>) -> &mut Self {
    init_sub_widget!(self, disabled, disabled, v)
  }

  /// Initializes the clip_boundary of the widget.
  pub fn with_clip_boundary<K: ?Sized>(&mut self, v: impl RInto<PipeValue<bool>, K>) -> &mut Self {
    init_sub_widget!(self, clip_boundary, clip_boundary, v)
  }

  /// Initializes the `keep_alive` value of the `KeepAlive` widget.
  pub fn with_keep_alive<K: ?Sized>(&mut self, v: impl RInto<PipeValue<bool>, K>) -> &mut Self {
    let (v, o) = v.r_into().unzip();
    let d = sub_widget!(self, keep_alive);
    d.write().keep_alive = v;
    if let Some(o) = o {
      let c_delay = d.clone_writer();

      // KeepAliveWidget may continue to exist after `on_disposed` is fired. It needs
      // to accept value changes to determine when to drop. So instead of
      // unsubscribing in `on_disposed`, we unsubscribe when the widget node is
      // dropped.
      let u = o
        .subscribe(move |v| c_delay.write().keep_alive = v)
        .unsubscribe_when_dropped();
      self.keep_alive_unsubscribe_handle = Some(Box::new(u));
    }
    self
  }

  /// Initializes the providers of the widget.
  pub fn with_providers(&mut self, providers: impl Into<SmallVec<[Provider; 1]>>) -> &mut Self {
    if let Some(vec) = self.providers.as_mut() {
      vec.extend(providers.into());
    } else {
      self.providers = Some(providers.into());
    }
    self
  }

  pub fn with_reuse_id(&mut self, reuse_id: impl Into<ReuseId>) -> &mut Self {
    assert!(self.reuse.is_none());
    self.reuse = Some(Reuse { reuse_id: reuse_id.into() });
    self
  }
}

impl<T> FatObj<T> {
  /// Creates and returns a state writer for managing the widget's CSS class
  /// name.
  ///
  /// The returned writer allows:
  /// - Modifying the class name dynamically
  /// - Observing changes to the class name
  /// - Composing with other reactive operations
  pub fn class(&mut self) -> impl StateWriter<Value = Option<ClassName>> {
    let class = sub_widget!(self, class);
    part_writer!(&mut class.class)
  }

  /// Returns a watcher that tracks whether the widget is currently being
  /// hovered over by a pointer.
  ///
  /// The watcher provides reactive boolean values indicating hover state
  /// changes.
  pub fn is_hovered(&mut self) -> impl StateWatcher<Value = bool> {
    self.mix_builtin_widget().trace_hover();
    self.mix_flags_watcher(|mix| PartRef::from_value(mix.is_hovered()))
  }

  /// Returns a watcher that tracks whether a pointer device is currently
  /// pressed on this widget.
  ///
  /// Useful for implementing press/release interactions and visual feedback.
  pub fn is_pointer_pressed(&mut self) -> impl StateWatcher<Value = bool> {
    self.mix_builtin_widget().trace_pointer_pressed();
    self.mix_flags_watcher(|mix| PartRef::from_value(mix.is_pointer_pressed()))
  }

  /// Returns a watcher that tracks the auto-focus state of the widget.
  ///
  /// The watcher will update whenever the auto-focus state changes.
  ///
  /// To change the auto-focus state, use [`FocusHandle::set_auto_focus`].
  pub fn is_auto_focus(&mut self) -> impl StateWatcher<Value = bool> {
    self
      .focus_handle()
      .flags
      .part_watcher(|focus| PartRef::from_value(focus.auto_focus()))
  }

  /// Returns a watcher that tracks whether the widget currently has input
  /// focus.
  ///
  /// This is essential for implementing focus-based behaviors and accessibility
  /// features.
  pub fn is_focused(&mut self) -> impl StateWatcher<Value = bool> {
    self
      .focus_handle()
      .flags
      .part_watcher(|focus| PartRef::from_value(focus.is_focused()))
  }

  /// Returns a watcher that tracks the reason why the widget currently has
  /// input focus.
  pub fn focus_changed_reason(&mut self) -> impl StateWatcher<Value = FocusReason> {
    self
      .focus_handle()
      .flags
      .part_watcher(|focus| PartRef::from_value(focus.focus_changed_reason()))
  }

  // Layout-related property watchers

  /// Returns a watcher for tracking changes to the widget's layout rectangle
  /// (position and size).
  pub fn layout_rect(&mut self) -> impl StateWatcher<Value = Rect> {
    self.layout_box_watcher(|layout| PartRef::from_value(layout.layout_rect()))
  }

  /// Returns a watcher for tracking changes to the widget's layout position
  /// (x,y coordinates).
  pub fn layout_pos(&mut self) -> impl StateWatcher<Value = Point> {
    self.layout_box_watcher(|layout| PartRef::from_value(layout.layout_pos()))
  }

  /// Returns a watcher for tracking changes to the widget's dimensions (width
  /// and height).
  pub fn layout_size(&mut self) -> impl StateWatcher<Value = Size> {
    self.layout_box_watcher(|layout| PartRef::from_value(layout.layout_size()))
  }

  // Individual layout dimension watchers

  /// Returns a watcher specifically for tracking the left position of the
  /// widget's layout.
  pub fn layout_left(&mut self) -> impl StateWatcher<Value = f32> {
    self.layout_box_watcher(|layout| PartRef::from_value(layout.layout_left()))
  }

  /// Returns a watcher specifically for tracking the top position of the
  /// widget's layout.
  pub fn layout_top(&mut self) -> impl StateWatcher<Value = f32> {
    self.layout_box_watcher(|layout| PartRef::from_value(layout.layout_top()))
  }

  /// Returns a watcher specifically for tracking the width of the widget's
  /// layout.
  pub fn layout_width(&mut self) -> impl StateWatcher<Value = f32> {
    self.layout_box_watcher(|layout| PartRef::from_value(layout.layout_width()))
  }

  /// Returns a watcher specifically for tracking the height of the widget's
  /// layout.
  pub fn layout_height(&mut self) -> impl StateWatcher<Value = f32> {
    self.layout_box_watcher(|layout| PartRef::from_value(layout.layout_height()))
  }

  // Style property writers

  /// Creates and returns a state writer for controlling how child content fits
  /// within this widget's bounds.
  pub fn box_fit(&mut self) -> impl StateWriter<Value = BoxFit> {
    let fitted_box = sub_widget!(self, fitted_box);
    part_writer!(&mut fitted_box.box_fit)
  }

  /// Creates and returns a state writer for managing the widget's backdrop
  /// filters.
  pub fn backdrop_filter(&mut self) -> impl StateWriter<Value = Filter> {
    let backdrop = sub_widget!(self, backdrop);
    part_writer!(&mut backdrop.filter)
  }

  /// Creates and returns a state writer for managing the widget's box shadow.
  pub fn box_shadow(&mut self) -> impl StateWriter<Value = BoxShadow> {
    let box_shadow = sub_widget!(self, box_shadow);
    part_writer!(&mut box_shadow.box_shadow)
  }

  /// Creates and returns a state writer for managing the widget's background
  /// color/brush.
  pub fn background(&mut self) -> impl StateWriter<Value = Brush> {
    let background = sub_widget!(self, background);
    part_writer!(&mut background.background)
  }

  /// Returns a state writer for modifying the widget's foreground brush.
  /// This controls the color/texture of text, icons, and other foreground
  /// elements.
  pub fn foreground(&mut self) -> impl StateWriter<Value = Brush> {
    let foreground = sub_widget!(self, foreground);
    part_writer!(&mut foreground.foreground)
  }

  /// Returns a state writer for modifying corner radius values.
  /// Controls the rounding of all four corners of the widget's bounding box.
  pub fn radius(&mut self) -> impl StateWriter<Value = Radius> {
    let radius = sub_widget!(self, radius);
    part_writer!(&mut radius.radius)
  }

  /// Returns a state writer for modifying border properties.
  /// Controls the width, color, and style of the widget's border.
  pub fn border(&mut self) -> impl StateWriter<Value = Border> {
    let border = sub_widget!(self, border);
    part_writer!(&mut border.border)
  }

  /// Returns a state writer for modifying the painting style.
  /// Determines whether to fill shapes (PaintingStyle::Fill) or stroke outlines
  /// (PaintingStyle::Stroke).
  pub fn painting_style(&mut self) -> impl StateWriter<Value = PaintingStyle> {
    let painting_style = sub_widget!(self, painting_style);
    part_writer!(&mut painting_style.painting_style)
  }

  /// Returns a state writer for modifying text alignment.
  /// Controls horizontal and vertical positioning of text within its container.
  pub fn text_align(&mut self) -> impl StateWriter<Value = TextAlign> {
    let text_align = sub_widget!(self, text_align);
    part_writer!(&mut text_align.text_align)
  }

  /// Returns a state writer for modifying the complete text style.
  /// Provides comprehensive control over font properties, colors, and text
  /// rendering.
  pub fn text_style(&mut self) -> impl StateWriter<Value = TextStyle> {
    let text_style = self.text_style_widget();
    part_writer!(&mut text_style.text_style)
  }

  /// Returns a state writer specifically for modifying font size.
  pub fn font_size(&mut self) -> impl StateWriter<Value = f32> {
    let style = self.text_style_widget();
    part_writer!(&mut style.text_style.font_size)
  }

  /// Returns a state writer specifically for modifying font face/family.
  pub fn font_face(&mut self) -> impl StateWriter<Value = FontFace> {
    let style = self.text_style_widget();
    part_writer!(&mut style.text_style.font_face)
  }

  /// Returns a state writer for modifying letter spacing (tracking).
  /// Adjusts horizontal space between characters (positive values increase
  /// spacing).
  pub fn letter_space(&mut self) -> impl StateWriter<Value = f32> {
    let style = self.text_style_widget();
    part_writer!(&mut style.text_style.letter_space)
  }

  /// Returns a state writer for modifying line height (leading).
  /// Controls vertical spacing between lines of text (multiplier relative to
  /// font size).
  pub fn text_line_height(&mut self) -> impl StateWriter<Value = f32> {
    let style = self.text_style_widget();
    part_writer!(&mut style.text_style.line_height)
  }

  /// Returns a state writer for configuring text overflow behavior.
  /// Determines how text is handled when it exceeds available space (clip,
  /// ellipsis, etc.).
  pub fn text_overflow(&mut self) -> impl StateWriter<Value = TextOverflow> {
    let style = self.text_style_widget();
    part_writer!(&mut style.text_style.overflow)
  }

  /// Returns a state writer for modifying interior padding.
  /// Controls space between the widget's border and its content.
  pub fn padding(&mut self) -> impl StateWriter<Value = EdgeInsets> {
    let padding = sub_widget!(self, padding);
    part_writer!(&mut padding.padding)
  }

  /// Returns a state writer for modifying exterior margins.
  /// Controls space between this widget and adjacent elements.
  pub fn margin(&mut self) -> impl StateWriter<Value = EdgeInsets> {
    let margin = sub_widget!(self, margin);
    part_writer!(&mut margin.margin)
  }

  /// Returns a state writer for modifying horizontal anchoring in global space.
  /// Positions widget relative to the screen's horizontal edges
  /// (left/center/right).
  pub fn global_anchor_x(&mut self) -> impl StateWriter<Value = Option<GlobalAnchorX>> {
    let anchor = sub_widget!(self, global_anchor);
    part_writer!(&mut anchor.global_anchor_x)
  }

  /// Returns a state writer for modifying vertical anchoring in global space.
  /// Positions widget relative to the screen's vertical edges
  /// (top/center/bottom).
  pub fn global_anchor_y(&mut self) -> impl StateWriter<Value = Option<GlobalAnchorY>> {
    let anchor = sub_widget!(self, global_anchor);
    part_writer!(&mut anchor.global_anchor_y)
  }

  /// Returns a state writer for modifying the cursor icon.
  /// Changes the mouse cursor when hovering over the widget (pointer, text,
  /// etc.).
  pub fn cursor(&mut self) -> impl StateWriter<Value = CursorIcon> {
    let cursor = sub_widget!(self, cursor);
    part_writer!(&mut cursor.cursor)
  }

  /// Returns a state writer for enabling/disabling scroll behavior.
  /// Controls whether the widget responds to scroll gestures and shows scroll
  /// indicators.
  pub fn scrollable(&mut self) -> impl StateWriter<Value = Scrollable> {
    let scrollable = sub_widget!(self, scrollable);
    part_writer!(&mut scrollable.scrollable)
  }

  /// Returns a state writer for modifying size constraints.
  /// Sets minimum/maximum size boundaries for the widget's layout.
  pub fn clamp(&mut self) -> impl StateWriter<Value = BoxClamp> {
    let constrained_box = sub_widget!(self, constrained_box);
    part_writer!(&mut constrained_box.clamp)
  }

  /// Returns a state writer for applying visual transformations.
  /// Applies matrix transformations (translation, rotation, scaling) to the
  /// widget.
  pub fn transform(&mut self) -> impl StateWriter<Value = Transform> {
    let transform = sub_widget!(self, transform);
    part_writer!(&mut transform.transform)
  }

  /// Returns a state writer for modifying horizontal alignment.
  /// Controls positioning within available horizontal space (left, center,
  /// right).
  pub fn h_align(&mut self) -> impl StateWriter<Value = HAlign> {
    let h_align = sub_widget!(self, h_align);
    part_writer!(&mut h_align.h_align)
  }

  /// Returns a state writer for modifying vertical alignment.
  /// Controls positioning within available vertical space (top, center,
  /// bottom).
  pub fn v_align(&mut self) -> impl StateWriter<Value = VAlign> {
    let v_align = sub_widget!(self, v_align);
    part_writer!(&mut v_align.v_align)
  }

  /// Returns a state writer for modifying relative anchoring.
  /// Positions widget relative to its parent using normalized coordinates
  /// (0-1).
  pub fn anchor(&mut self) -> impl StateWriter<Value = Anchor> {
    let anchor = sub_widget!(self, relative_anchor);
    part_writer!(&mut anchor.anchor)
  }

  /// Returns a state writer for modifying visibility state.
  /// Controls whether the widget is rendered (true = visible, false = hidden).
  pub fn visible(&mut self) -> impl StateWriter<Value = bool> {
    let visibility = sub_widget!(self, visibility);
    part_writer!(&mut visibility.visible)
  }

  /// Returns a state writer for modifying opacity.
  /// Controls transparency level (0.0 = fully transparent, 1.0 = fully opaque).
  pub fn opacity(&mut self) -> impl StateWriter<Value = f32> {
    let opacity = sub_widget!(self, opacity);
    part_writer!(&mut opacity.opacity)
  }

  /// Returns a state writer for modifying keep-alive behavior.
  /// When true, preserves widget state even when not visible in the UI.
  pub fn keep_alive(&mut self) -> impl StateWriter<Value = bool> {
    let keep_alive = sub_widget!(self, keep_alive);
    part_writer!(&mut keep_alive.keep_alive)
  }

  /// Returns a state writer for modifying tooltip content.
  /// Controls the text displayed when hovering over the widget.
  pub fn tooltips(&mut self) -> impl StateWriter<Value = CowArc<str>> {
    let tooltips = sub_widget!(self, tooltips);
    part_writer!(&mut tooltips.tooltips)
  }

  /// Returns the widget's unique tracking identifier.
  /// Used for performance monitoring and debugging purposes.
  pub fn track_id(&mut self) -> TrackId { sub_widget!(self, track_id).read().track_id() }

  /// Returns a state writer for modifying clipping behavior.
  /// When true, children are clipped to this widget's bounds.
  pub fn clip_boundary(&mut self) -> impl StateWriter<Value = bool> {
    let widget = sub_widget!(self, clip_boundary);
    part_writer!(&mut widget.clip_boundary)
  }

  /// Returns a state writer for modifying disabled state.
  /// When true, widget ignores all user interaction events.
  pub fn disabled(&mut self) -> impl StateWriter<Value = bool> {
    let widget = sub_widget!(self, disabled);
    part_writer!(&mut widget.disabled)
  }

  /// Helper method to reduce code duplication for focus-related state watchers
  fn mix_flags_watcher<R: 'static>(
    &mut self, mapper: fn(&MixFlags) -> PartRef<R>,
  ) -> impl StateWatcher<Value = R> {
    self
      .mix_builtin_widget()
      .mix_flags()
      .part_watcher(mapper)
  }

  fn layout_box_watcher<R: 'static>(
    &mut self, mapper: fn(&LayoutBox) -> PartRef<R>,
  ) -> impl StateWatcher<Value = R> {
    sub_widget!(self, layout_box).part_watcher(mapper)
  }
}

impl<T> FatObj<T> {
  /// Take the scrollable widget from this widget, and return it if it exists.
  pub fn take_scrollable_widget(&mut self) -> Option<Stateful<ScrollableWidget>> {
    self.scrollable.take()
  }

  /// Returns `true` if the widget has a class.
  pub fn has_class(&self) -> bool { self.class.is_some() }
}

// builtin widgets accessors
impl<T> FatObj<T> {
  /// Return the focus widget that can track and manage the focus states
  pub fn focus_handle(&mut self) -> FocusHandle {
    let wnd = BuildCtx::get().window().id();
    let host = self.track_id();
    self.mix_builtin_widget().focus_handle(wnd, host)
  }

  /// Returns the `Stateful<ScrollableWidget>` widget from the FatObj. If it
  /// doesn't exist, a new one will be created.
  pub fn scrollable_widget(&mut self) -> &Stateful<ScrollableWidget> {
    sub_widget!(self, scrollable)
  }

  /// Returns the `Stateful<RelativeAnchor>` widget from the FatObj. If it
  /// doesn't exist, a new one will be created.
  pub fn relative_anchor_widget(&mut self) -> &Stateful<RelativeAnchor> {
    sub_widget!(self, relative_anchor)
  }

  /// Returns the `Stateful<GlobalAnchor>` widget from the FatObj. If it doesn't
  /// exist, a new one will be created.
  pub fn global_anchor_widget(&mut self) -> &Stateful<GlobalAnchor> {
    sub_widget!(self, global_anchor)
  }

  fn mix_builtin_widget(&mut self) -> &MixBuiltin {
    self
      .mix_builtin
      .get_or_insert_with(MixBuiltin::default)
  }

  /// Returns the `Stateful<TextStyleWidget>` widget from the FatObj. If it
  /// doesn't exist, a new one will be created.
  fn text_style_widget(&mut self) -> &Stateful<TextStyleWidget> {
    self
      .text_style
      .get_or_insert_with(|| Stateful::new(TextStyleWidget::inherit_widget()))
  }

  /// Returns the `Stateful<Tooltips>` widget from the FatObj. If it doesn't
  /// exist, a new one is created.
  pub fn tooltips_widget(&mut self) -> &Stateful<Tooltips> {
    self
      .tooltips
      .get_or_insert_with(|| Stateful::new(<_>::default()))
  }
}

macro_rules! sub_widget {
  ($this:expr, $path:ident) => {
    $this
      .$path
      .get_or_insert_with(|| Stateful::new(<_>::default()))
  };
}
use sub_widget;

macro_rules! init_sub_widget {
  ($this:expr, $sub_widget_path:ident, $field:ident, $init_value:ident) => {{
    let mix = $this
      .mix_builtin
      .get_or_insert_with(MixBuiltin::default);
    let sub_widget = $this
      .$sub_widget_path
      .get_or_insert_with(|| Stateful::new(<_>::default()));
    mix.init_sub_widget($init_value, sub_widget, move |widget, v| widget.$field = v);
    $this
  }};
}

macro_rules! init_text_style {
  ($this:expr, $field:ident, $init_value:ident) => {{
    let mix = $this
      .mix_builtin
      .get_or_insert_with(MixBuiltin::default);
    let text_style = $this
      .text_style
      .get_or_insert_with(|| Stateful::new(TextStyleWidget::inherit_widget()));
    mix.init_sub_widget($init_value, text_style, move |w, v| w.text_style.$field = v);
    $this
  }};
}
use init_sub_widget;
use init_text_style;

impl MixBuiltin {
  fn init_sub_widget<V: 'static, B: 'static, K: ?Sized>(
    &self, init: impl RInto<PipeValue<V>, K>, sub_widget: &impl StateWriter<Value = B>,
    set_value: fn(&mut B, V),
  ) {
    let (v, o) = init.r_into().unzip();
    set_value(&mut *sub_widget.silent(), v);
    if let Some(o) = o {
      let sub_widget = sub_widget.clone_writer();
      let u = o.subscribe(move |v| set_value(&mut *sub_widget.write(), v));
      self.on_disposed(move |_| u.unsubscribe());
    }
  }
}

impl<'a> FatObj<Widget<'a>> {
  pub(crate) fn compose(mut self) -> Widget<'a> {
    macro_rules! compose_builtin_widgets {
      ($host: ident + [$($field: ident),*]) => {
        $(
          if let Some($field) = self.$field {
            $host = $field.with_child($host).into_widget();
          }
        )*
      };
    }
    macro_rules! consume_providers_widget {
      ($host: ident, + [$($field: ident: $w_ty: ty),*]) => {
        $(
          if let Some($field) = self.$field {
            self
            .providers
            .get_or_insert_default()
            .push(<$w_ty>::into_provider($field));
          }
        )*
      };
    }
    let mut host = self.host;
    consume_providers_widget!(host, + [
      painting_style: PaintingStyleWidget,
      text_style: TextStyleWidget,
      text_align: TextAlignWidget
    ]);

    compose_builtin_widgets!(
      host
        + [
          track_id,
          backdrop,
          padding,
          foreground,
          border,
          background,
          filter,
          clip_boundary,
          box_shadow,
          fitted_box,
          radius,
          scrollable,
          layout_box
        ]
    );
    if let Some(providers) = self.providers {
      host = Providers::new(providers).with_child(host);
    }

    compose_builtin_widgets!(
      host
        + [
          class,
          constrained_box,
          tooltips,
          margin,
          cursor,
          mix_builtin,
          transform,
          opacity,
          visibility,
          disabled,
          h_align,
          v_align,
          relative_anchor,
          global_anchor,
          keep_alive,
          reuse
        ]
    );

    if let Some(h) = self.keep_alive_unsubscribe_handle {
      host = host.attach_anonymous_data(h);
    }
    host
  }
}

impl FatObj<()> {
  #[inline]
  pub fn with_child<C>(self, child: C) -> FatObj<C> { self.map(move |_| child) }
}

impl<T> std::ops::Deref for FatObj<T> {
  type Target = T;
  #[inline]
  fn deref(&self) -> &Self::Target { &self.host }
}

impl<T> std::ops::DerefMut for FatObj<T> {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.host }
}

/// DeclarerWithSubscription, a declarer with subscriptions
///
/// Used to wraps a declarer to make it the widget auto unsubscribe when
/// disposed. Normally you should not use this directly, most Widget types
/// derive with Declare attribute has support builtin widgets has the ability
/// of unsubscribing when disposed.
pub struct DeclarerWithSubscription<T> {
  inner: T,
  subscribes: SmallVec<[BoxedSubscription; 1]>,
}

impl<T> DeclarerWithSubscription<T> {
  pub fn new(host: T, subscribes: SmallVec<[BoxedSubscription; 1]>) -> Self {
    Self { inner: host, subscribes }
  }

  fn map<M>(self, f: impl FnOnce(T) -> M) -> DeclarerWithSubscription<M> {
    DeclarerWithSubscription { inner: f(self.inner), subscribes: self.subscribes }
  }
}

impl<T> Deref for DeclarerWithSubscription<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target { &self.inner }
}

impl<T> DerefMut for DeclarerWithSubscription<T> {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.inner }
}

impl<'w, T, K> RFrom<DeclarerWithSubscription<T>, OtherWidget<DeclarerWithSubscription<K>>>
  for Widget<'w>
where
  T: IntoWidget<'w, K> + 'w,
{
  fn r_from(value: DeclarerWithSubscription<T>) -> Self {
    let DeclarerWithSubscription { inner, subscribes } = value;

    if subscribes.is_empty() {
      inner.into_widget()
    } else {
      let mut w = FatObj::new(inner.into_widget());
      w.on_disposed(move |_| {
        subscribes
          .into_iter()
          .for_each(|u| u.unsubscribe());
      });
      w.into_widget()
    }
  }
}

impl<T: SingleChild> SingleChild for DeclarerWithSubscription<T> {}

impl<P> MultiChild for DeclarerWithSubscription<P> where P: MultiChild {}

impl<P: Parent> Parent for DeclarerWithSubscription<P> {
  fn with_children<'w>(self, children: Vec<Widget<'w>>) -> Widget<'w>
  where
    Self: 'w,
  {
    self
      .map(|host| host.with_children(children))
      .into_widget()
  }
}

impl<C, K: ?Sized, P> ComposeWithChild<C, K> for DeclarerWithSubscription<P>
where
  P: ComposeWithChild<C, K>,
{
  type Target = DeclarerWithSubscription<P::Target>;
  fn with_child(self, child: C) -> Self::Target { self.map(|host| host.with_child(child)) }
}

impl Declare for FatObj<()> {
  type Builder = Self;
  fn declarer() -> Self::Builder { FatObj::default() }
}

impl ObjDeclarer for FatObj<()> {
  type Target = Self;

  fn finish(self) -> Self::Target { self }
}
