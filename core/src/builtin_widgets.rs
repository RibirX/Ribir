//! Built-in widgets is a set of minimal widgets that describes the most common
//! UI elements.
//!
//! The most of them can be used to extend other object in the
//! declare syntax, so other objects can use the builtin fields and methods like
//! self fields and methods.

pub mod key;
mod painting_style;
use std::ops::DerefMut;

pub use key::{Key, KeyWidget};
pub use painting_style::*;
pub mod image_widget;
pub mod keep_alive;
pub use keep_alive::*;
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
mod box_decoration;
pub use box_decoration::*;
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

pub mod clip;
pub use clip::*;
pub mod focus_node;
pub use focus_node::*;
pub mod focus_scope;
pub use focus_scope::*;
pub mod global_anchor;
pub use global_anchor::*;
mod mix_builtin;
pub use mix_builtin::*;
pub mod container;
pub use container::*;
mod provider;
pub use provider::*;
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
/// let w = |ctx: &BuildCtx| {
///   let mut multi = FatObj::new(MockMulti::default()).margin(EdgeInsets::all(10.));
///
///   let w = multi.get_margin_widget().clone_writer();
///   multi
///     .on_tap(move |_| w.write().margin = EdgeInsets::all(20.))
///     .into_widget()
/// };
/// ```
#[derive(Default)]
pub struct FatObj<T> {
  host: T,
  track_id: Option<State<TrackWidgetId>>,
  class: Option<State<Class>>,
  padding: Option<State<Padding>>,
  fitted_box: Option<State<FittedBox>>,
  constrained_box: Option<State<ConstrainedBox>>,
  box_decoration: Option<State<BoxDecoration>>,
  foreground: Option<State<Foreground>>,
  scrollable: Option<State<ScrollableWidget>>,
  layout_box: Option<State<LayoutBox>>,
  mix_builtin: Option<MixBuiltin>,
  request_focus: Option<State<RequestFocus>>,
  cursor: Option<State<Cursor>>,
  margin: Option<State<Margin>>,
  transform: Option<State<TransformWidget>>,
  opacity: Option<State<Opacity>>,
  visibility: Option<State<Visibility>>,
  h_align: Option<State<HAlignWidget>>,
  v_align: Option<State<VAlignWidget>>,
  relative_anchor: Option<State<RelativeAnchor>>,
  global_anchor: Option<State<GlobalAnchor>>,
  painting_style: Option<State<PaintingStyleWidget>>,
  text_style: Option<State<TextStyleWidget>>,
  keep_alive: Option<State<KeepAlive>>,
  tooltips: Option<State<Tooltips>>,
  keep_alive_unsubscribe_handle: Option<Box<dyn Any>>,
}

/// Create a function widget that uses an empty `FatObj` as the host object.
#[macro_export]
macro_rules! fat_obj {
  ($($t: tt)*) => {
    fn_widget! {
      let obj = FatObj::<()>::default();
      @ $obj { $($t)* }
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
      request_focus: self.request_focus,
      fitted_box: self.fitted_box,
      box_decoration: self.box_decoration,
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
      visibility: self.visibility,
      opacity: self.opacity,
      tooltips: self.tooltips,
      keep_alive: self.keep_alive,
      keep_alive_unsubscribe_handle: self.keep_alive_unsubscribe_handle,
    }
  }

  /// Return true if the FatObj not contains any builtin widgets.
  pub fn is_empty(&self) -> bool {
    self.track_id.is_none()
      && self.mix_builtin.is_none()
      && self.request_focus.is_none()
      && self.fitted_box.is_none()
      && self.box_decoration.is_none()
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

// builtin widgets accessors
impl<T> FatObj<T> {
  /// Returns the `State<TrackWidgetId>` widget from the FatObj. If it doesn't
  /// exist, a new one is created.
  pub fn get_track_id_widget(&mut self) -> &State<TrackWidgetId> {
    self
      .track_id
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  /// Returns the `State<Class>` widget from the FatObj. If it doesn't exist, a
  /// new one is created.
  pub fn get_class_widget(&mut self) -> &State<Class> {
    self
      .class
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  pub fn get_mix_builtin_widget(&mut self) -> &MixBuiltin {
    self
      .mix_builtin
      .get_or_insert_with(MixBuiltin::default)
  }

  /// Returns the `State<RequestFocus>` widget from the FatObj. If it doesn't
  /// exist, a new one will be created.
  pub fn get_request_focus_widget(&mut self) -> &State<RequestFocus> {
    self
      .request_focus
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  /// Return the `State<MixFlags>` from the `MixBuiltin`. If the `MixBuiltin`
  /// does not exist in the `FatObj`, a new one will be created..
  pub fn get_mix_flags_widget(&mut self) -> &State<MixFlags> {
    self.get_mix_builtin_widget().mix_flags()
  }

  /// Begin tracing the focus status of this widget.
  pub fn trace_focus(&mut self) -> &mut Self {
    self.get_mix_builtin_widget().trace_focus();
    self
  }

  /// Begin tracing the hover status of this widget.
  pub fn trace_hover(&mut self) -> &mut Self {
    self.get_mix_builtin_widget().trace_hover();
    self
  }

  /// Begin tracing if the pointer pressed on this widget
  pub fn trace_pointer_pressed(&mut self) -> &mut Self {
    self
      .get_mix_builtin_widget()
      .trace_pointer_pressed();
    self
  }

  /// Returns the `State<FittedBox>` widget from the FatObj. If it doesn't
  /// exist, a new one will be created.
  pub fn get_fitted_box_widget(&mut self) -> &State<FittedBox> {
    self
      .fitted_box
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  /// Returns the `State<BoxDecoration>` widget from the FatObj. If it doesn't
  /// exist, a new one will be created.
  pub fn get_box_decoration_widget(&mut self) -> &State<BoxDecoration> {
    self
      .box_decoration
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  /// Returns the `State<Foreground>` widget from the FatObj. If it does not
  /// exist, a new one will be created.
  pub fn get_foreground_widget(&mut self) -> &State<Foreground> {
    self
      .foreground
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  /// Returns the `State<Padding>` widget from the FatObj. If it doesn't exist,
  /// a new one will be created.
  pub fn get_padding_widget(&mut self) -> &State<Padding> {
    self
      .padding
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  /// Returns the `State<LayoutBox>` widget from the FatObj. If it doesn't
  /// exist, a new one will be created.
  pub fn get_layout_box_widget(&mut self) -> &State<LayoutBox> {
    self
      .layout_box
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  /// Returns the `State<Cursor>` widget from the FatObj. If it doesn't exist, a
  /// new one is created.
  pub fn get_cursor_widget(&mut self) -> &State<Cursor> {
    self
      .cursor
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  /// Returns the `State<Margin>` widget from the FatObj. If it doesn't exist, a
  /// new one is created.
  pub fn get_margin_widget(&mut self) -> &State<Margin> {
    self
      .margin
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  pub fn get_constrained_box_widget(&mut self) -> &State<ConstrainedBox> {
    self
      .constrained_box
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  /// Returns the `State<ScrollableWidget>` widget from the FatObj. If it
  /// doesn't exist, a new one will be created.
  pub fn get_scrollable_widget(&mut self) -> &State<ScrollableWidget> {
    self
      .scrollable
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  /// Returns the `State<TransformWidget>` widget from the FatObj. If it doesn't
  /// exist, a new one will be created.
  pub fn get_transform_widget(&mut self) -> &State<TransformWidget> {
    self
      .transform
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  /// Returns the `State<HAlignWidget>` widget from the FatObj. If it doesn't
  /// exist, a new one will be created.
  pub fn get_h_align_widget(&mut self) -> &State<HAlignWidget> {
    self
      .h_align
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  /// Returns the `State<VAlignWidget>` widget from the FatObj. If it doesn't
  /// exist, a new one will be created.
  pub fn get_v_align_widget(&mut self) -> &State<VAlignWidget> {
    self
      .v_align
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  /// Returns the `State<RelativeAnchor>` widget from the FatObj. If it doesn't
  /// exist, a new one will be created.
  pub fn get_relative_anchor_widget(&mut self) -> &State<RelativeAnchor> {
    self
      .relative_anchor
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  /// Returns the `State<GlobalAnchor>` widget from the FatObj. If it doesn't
  /// exist, a new one will be created.
  pub fn get_global_anchor_widget(&mut self) -> &State<GlobalAnchor> {
    self
      .global_anchor
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  /// Returns the `State<PaintingStyleWidget>` widget from the FatObj. If it
  /// doesn't exist, a new one will be created.
  pub fn get_painting_style_widget(&mut self) -> &State<PaintingStyleWidget> {
    self
      .painting_style
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  /// Returns the `State<TextStyleWidget>` widget from the FatObj. If it
  /// doesn't exist, a new one will be created.
  pub fn get_text_style_widget(&mut self) -> &State<TextStyleWidget> {
    self.text_style.get_or_insert_with(|| {
      State::value(TextStyleWidget { text_style: BuildCtx::get().text_style().clone() })
    })
  }

  /// Returns the `State<Visibility>` widget from the FatObj. If it doesn't
  /// exist, a new one will be created.
  pub fn get_visibility_widget(&mut self) -> &State<Visibility> {
    self
      .visibility
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  /// Returns the `State<Opacity>` widget from the FatObj. If it doesn't exist,
  /// a new one will be created.
  pub fn get_opacity_widget(&mut self) -> &State<Opacity> {
    self
      .opacity
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  /// Returns the `State<KeepAlive>` widget from the FatObj. If it doesn't
  /// exist, a new one will be created.
  pub fn get_keep_alive_widget(&mut self) -> &State<KeepAlive> {
    self
      .keep_alive
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  /// Returns the `State<Tooltips>` widget from the FatObj. If it doesn't
  /// exist, a new one is created.
  pub fn get_tooltips_widget(&mut self) -> &State<Tooltips> {
    self
      .tooltips
      .get_or_insert_with(|| State::value(<_>::default()))
  }
}

macro_rules! on_mixin {
  ($this:ident, $on_method:ident, $f:ident) => {{
    $this.get_mix_builtin_widget().$on_method($f);
    $this
  }};
}

// report all builtin widgets apis
impl<T> FatObj<T> {
  /// Attaches an event handler to the widget. It's triggered when any event or
  /// lifecycle change happens.
  pub fn on_event(mut self, f: impl FnMut(&mut Event) + 'static) -> Self {
    on_mixin!(self, on_event, f)
  }

  /// Attaches an event handler that runs when the widget is first mounted to
  /// the tree.
  pub fn on_mounted(mut self, f: impl FnOnce(&mut LifecycleEvent) + 'static) -> Self {
    on_mixin!(self, on_mounted, f)
  }

  /// Attaches an event handler that runs after the widget is performed layout.
  pub fn on_performed_layout(mut self, f: impl FnMut(&mut LifecycleEvent) + 'static) -> Self {
    on_mixin!(self, on_performed_layout, f)
  }

  /// Attaches an event handler that runs when the widget is disposed.
  pub fn on_disposed(mut self, f: impl FnOnce(&mut LifecycleEvent) + 'static) -> Self {
    on_mixin!(self, on_disposed, f)
  }

  /// Attaches a handler to the widget that is triggered when a pointer down
  /// occurs.
  pub fn on_pointer_down(mut self, f: impl FnMut(&mut PointerEvent) + 'static) -> Self {
    on_mixin!(self, on_pointer_down, f)
  }

  /// Attaches a handler to the widget that is triggered during the capture
  /// phase of a pointer down event. This is similar to `on_pointer_down`, but
  /// it's triggered earlier in the event flow. For more information on event
  /// capturing, see [Event capture](https://www.w3.org/TR/DOM-Level-2-Events/events.html#Events-flow-capture).
  pub fn on_pointer_down_capture(mut self, f: impl FnMut(&mut PointerEvent) + 'static) -> Self {
    on_mixin!(self, on_pointer_down_capture, f)
  }

  /// Attaches a handler to the widget that is triggered when a pointer up
  /// occurs.
  pub fn on_pointer_up(mut self, f: impl FnMut(&mut PointerEvent) + 'static) -> Self {
    on_mixin!(self, on_pointer_up, f)
  }

  /// Attaches a handler to the widget that is triggered during the capture
  /// phase of a pointer up event. This is similar to `on_pointer_up`, but it's
  /// triggered earlier in the event flow. For more information on event
  /// capturing, see [Event capture](https://www.w3.org/TR/DOM-Level-2-Events/events.html#Events-flow-capture).
  pub fn on_pointer_up_capture(mut self, f: impl FnMut(&mut PointerEvent) + 'static) -> Self {
    on_mixin!(self, on_pointer_up_capture, f)
  }

  /// Attaches a handler to the widget that is triggered when a pointer move
  /// occurs.
  pub fn on_pointer_move(mut self, f: impl FnMut(&mut PointerEvent) + 'static) -> Self {
    on_mixin!(self, on_pointer_move, f)
  }

  /// Attaches a handler to the widget that is triggered during the capture
  /// phase of a pointer move event. This is similar to `on_pointer_move`, but
  /// it's triggered earlier in the event flow. For more information on event
  /// capturing, see [Event capture](https://www.w3.org/TR/DOM-Level-2-Events/events.html#Events-flow-capture).
  pub fn on_pointer_move_capture(mut self, f: impl FnMut(&mut PointerEvent) + 'static) -> Self {
    on_mixin!(self, on_pointer_move_capture, f)
  }

  /// Attaches a handler to the widget that is triggered when a pointer event
  /// cancels.
  pub fn on_pointer_cancel(mut self, f: impl FnMut(&mut PointerEvent) + 'static) -> Self {
    on_mixin!(self, on_pointer_cancel, f)
  }

  /// Attaches a handler to the widget that is triggered when a pointer device
  /// is moved into the hit test boundaries of an widget or one of its
  /// descendants.
  pub fn on_pointer_enter(mut self, f: impl FnMut(&mut PointerEvent) + 'static) -> Self {
    on_mixin!(self, on_pointer_enter, f)
  }

  /// Attaches a handler to the widget that is triggered when a pointer device
  /// is moved out of the hit test boundaries of an widget or one of its
  /// descendants.
  pub fn on_pointer_leave(mut self, f: impl FnMut(&mut PointerEvent) + 'static) -> Self {
    on_mixin!(self, on_pointer_leave, f)
  }

  /// Attaches a handler to the widget that is triggered when a tap(click)
  /// occurs.
  pub fn on_tap(mut self, f: impl FnMut(&mut PointerEvent) + 'static) -> Self {
    on_mixin!(self, on_tap, f)
  }

  /// Attaches a handler to the widget that is triggered during the capture
  /// phase of a tap event. This is similar to `on_tap`, but it's triggered
  /// earlier in the event flow. For more information on event capturing, see
  /// [Event capture](https://www.w3.org/TR/DOM-Level-2-Events/events.html#Events-flow-capture).
  pub fn on_tap_capture(mut self, f: impl FnMut(&mut PointerEvent) + 'static) -> Self {
    on_mixin!(self, on_tap_capture, f)
  }

  /// Attaches a handler to the widget that is triggered when a double tap
  /// occurs.
  pub fn on_double_tap(mut self, f: impl FnMut(&mut PointerEvent) + 'static) -> Self {
    on_mixin!(self, on_double_tap, f)
  }

  /// Attaches a handler to the widget that is triggered during the capture
  /// phase of a double tap event. This is similar to `on_double_tap`, but it's
  /// triggered earlier in the event flow. For more information on event
  /// capturing, see [Event capture](https://www.w3.org/TR/DOM-Level-2-Events/events.html#Events-flow-capture).
  pub fn on_double_tap_capture(mut self, f: impl FnMut(&mut PointerEvent) + 'static) -> Self {
    on_mixin!(self, on_double_tap_capture, f)
  }

  /// Attaches a handler to the widget that is triggered when a triple tap
  /// occurs.
  pub fn on_triple_tap(mut self, f: impl FnMut(&mut PointerEvent) + 'static) -> Self {
    on_mixin!(self, on_triple_tap, f)
  }

  /// Attaches a handler to the widget that is triggered when a triple tap
  /// occurs. This is similar to `on_double_tap`, but it's triggered earlier
  /// in the event flow. For more information on event capturing, see
  /// [Event capture](https://www.w3.org/TR/DOM-Level-2-Events/events.html#Events-flow-capture).
  pub fn on_triple_tap_capture(mut self, f: impl FnMut(&mut PointerEvent) + 'static) -> Self {
    on_mixin!(self, on_triple_tap_capture, f)
  }

  /// Attaches a handler to the widget that is triggered when a x-times tap
  /// occurs.
  pub fn on_x_times_tap(
    mut self, (times, f): (usize, impl FnMut(&mut PointerEvent) + 'static),
  ) -> Self {
    self
      .get_mix_builtin_widget()
      .on_x_times_tap((times, f));
    self
  }

  /// Attaches a handler to the widget that is triggered during the capture
  /// phase of a x-times tap event. This is similar to `on_x_times_tap`, but
  /// it's triggered earlier in the event flow. For more information on event
  /// capturing, see [Event capture](https://www.w3.org/TR/DOM-Level-2-Events/events.html#Events-flow-capture).
  pub fn on_x_times_tap_capture(
    mut self, (times, f): (usize, impl FnMut(&mut PointerEvent) + 'static),
  ) -> Self {
    self
      .get_mix_builtin_widget()
      .on_x_times_tap_capture((times, f));
    self
  }

  /// Attaches a handler to the widget that is triggered when the user rotates a
  /// wheel button on a pointing device (typically a mouse).
  pub fn on_wheel(mut self, f: impl FnMut(&mut WheelEvent) + 'static) -> Self {
    on_mixin!(self, on_wheel, f)
  }

  /// Attaches a handler to the widget that is triggered during the capture
  /// phase of a wheel event. This is similar to `on_wheel`, but it's triggered
  /// earlier in the event flow. For more information on event capturing, see
  /// [Event capture](https://www.w3.org/TR/DOM-Level-2-Events/events.html#Events-flow-capture).
  pub fn on_wheel_capture(mut self, f: impl FnMut(&mut WheelEvent) + 'static) -> Self {
    on_mixin!(self, on_wheel_capture, f)
  }

  /// Attaches a handler to the widget that is triggered when the input method
  /// pre-edit area is changed.
  pub fn on_ime_pre_edit(mut self, f: impl FnMut(&mut ImePreEditEvent) + 'static) -> Self {
    on_mixin!(self, on_ime_pre_edit, f)
  }

  /// Attaches a handler to the widget that is triggered during the capture
  /// phase of a ime pre-edit event. This is similar to `on_ime_pre_edit`,
  /// but it's triggered earlier in the event flow. For more information on
  /// event capturing, see [Event capture](https://www.w3.org/TR/DOM-Level-2-Events/events.html#Events-flow-capture).
  pub fn on_ime_pre_edit_capture(mut self, f: impl FnMut(&mut ImePreEditEvent) + 'static) -> Self {
    on_mixin!(self, on_ime_pre_edit_capture, f)
  }

  /// Attaches a handler to the widget that is triggered when the input method
  /// commits text or keyboard pressed the text key.
  pub fn on_chars(mut self, f: impl FnMut(&mut CharsEvent) + 'static) -> Self {
    on_mixin!(self, on_chars, f)
  }

  /// Attaches a handler to the widget that is triggered during the capture
  /// phase of a chars event. This is similar to `on_chars`, but it's triggered
  /// earlier in the event flow. For more information on event capturing,
  /// see [Event capture](https://www.w3.org/TR/DOM-Level-2-Events/events.html#Events-flow-capture).
  pub fn on_chars_capture(mut self, f: impl FnMut(&mut CharsEvent) + 'static) -> Self {
    on_mixin!(self, on_chars_capture, f)
  }

  /// Attaches a handler to the widget that is triggered when the keyboard key
  /// is pressed.
  pub fn on_key_down(mut self, f: impl FnMut(&mut KeyboardEvent) + 'static) -> Self {
    on_mixin!(self, on_key_down, f)
  }

  /// Attaches a handler to the widget that is triggered during the capture
  /// phase of a key down event. This is similar to `on_key_down`, but it's
  /// triggered earlier in the event flow. For more information on event
  /// capturing, see [Event capture](https://www.w3.org/TR/DOM-Level-2-Events/events.html#Events-flow-capture).
  pub fn on_key_down_capture(mut self, f: impl FnMut(&mut KeyboardEvent) + 'static) -> Self {
    on_mixin!(self, on_key_down_capture, f)
  }

  /// Attaches a handler to the widget that is triggered when the keyboard key
  /// is released.
  pub fn on_key_up(mut self, f: impl FnMut(&mut KeyboardEvent) + 'static) -> Self {
    on_mixin!(self, on_key_up, f)
  }

  /// Attaches a handler to the widget that is triggered during the capture
  /// phase of a key up event. This is similar to `on_key_up`, but it's
  /// triggered earlier in the event flow. For more information on event
  /// capturing, see [Event capture](https://www.w3.org/TR/DOM-Level-2-Events/events.html#Events-flow-capture).
  pub fn on_key_up_capture(mut self, f: impl FnMut(&mut KeyboardEvent) + 'static) -> Self {
    on_mixin!(self, on_key_up_capture, f)
  }

  /// Attaches a handler to the widget that is triggered when the widget is
  /// focused.
  pub fn on_focus(mut self, f: impl FnMut(&mut FocusEvent) + 'static) -> Self {
    on_mixin!(self, on_focus, f)
  }

  /// Attaches a handler to the widget that is triggered when the widget is lost
  /// focus.
  pub fn on_blur(mut self, f: impl FnMut(&mut FocusEvent) + 'static) -> Self {
    on_mixin!(self, on_blur, f)
  }

  /// Attaches a handler to the widget that is triggered when the widget or its
  /// descendants are focused. The main difference between this event and focus
  /// is that focusin bubbles while focus does not.
  pub fn on_focus_in(mut self, f: impl FnMut(&mut FocusEvent) + 'static) -> Self {
    on_mixin!(self, on_focus_in, f)
  }

  /// Attaches a handler to the widget that is triggered during the capture
  /// phase of a focus in event. This is similar to `on_focus_in`, but it's
  /// triggered earlier in the event flow. For more information on event
  /// capturing, see [Event capture](https://www.w3.org/TR/DOM-Level-2-Events/events.html#Events-flow-capture).
  pub fn on_focus_in_capture(mut self, f: impl FnMut(&mut FocusEvent) + 'static) -> Self {
    on_mixin!(self, on_focus_in_capture, f)
  }

  /// Attaches a handler to the widget that is triggered when the widget or its
  /// descendants are lost focus. The main difference between this event and
  /// focusout is that focusout bubbles while blur does not.
  pub fn on_focus_out(mut self, f: impl FnMut(&mut FocusEvent) + 'static) -> Self {
    on_mixin!(self, on_focus_out, f)
  }

  /// Attaches a handler to the widget that is triggered during the capture
  /// phase of a focus out event. This is similar to `on_focus_out`, but it's
  /// triggered earlier in the event flow. For more information on event
  /// capturing, see [Event capture](https://www.w3.org/TR/DOM-Level-2-Events/events.html#Events-flow-capture).
  pub fn on_focus_out_capture(mut self, f: impl FnMut(&mut FocusEvent) + 'static) -> Self {
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
  pub fn tab_index<const M: usize>(self, tab_idx: impl DeclareInto<i16, M>) -> Self {
    self.declare_builtin_init(
      tab_idx,
      |this| this.get_mix_builtin_widget().mix_flags(),
      |m, v| m.set_tab_index(v),
    )
  }

  /// Initializes the `Class` that should be applied to the widget.
  pub fn class<const M: usize>(self, cls: impl DeclareInto<ClassName, M>) -> Self {
    self.declare_builtin_init(cls, Self::get_class_widget, |c, cls| c.class = Some(cls))
  }

  /// Initializes whether the `widget` should automatically get focus when the
  /// window loads.
  ///
  /// Only one widget should have this attribute specified.  If there are
  /// several, the widget nearest the root, get the initial focus.
  pub fn auto_focus<const M: usize>(self, v: impl DeclareInto<bool, M>) -> Self {
    self.declare_builtin_init(
      v,
      |this| this.get_mix_builtin_widget().mix_flags(),
      |m, v| m.set_auto_focus(v),
    )
  }

  /// Initializes how its child should be scale to fit its box.
  pub fn box_fit<const M: usize>(self, v: impl DeclareInto<BoxFit, M>) -> Self {
    self.declare_builtin_init(v, Self::get_fitted_box_widget, |m, v| m.box_fit = v)
  }

  /// Initializes the painting style of this widget.
  pub fn painting_style<const M: usize>(self, v: impl DeclareInto<PaintingStyle, M>) -> Self {
    self.declare_builtin_init(v, Self::get_painting_style_widget, |m, v| m.painting_style = v)
  }

  /// Initializes the text style of this widget.
  pub fn text_style<const M: usize>(self, v: impl DeclareInto<TextStyle, M>) -> Self {
    self.declare_builtin_init(v, Self::get_text_style_widget, |m, v| m.text_style = v)
  }

  /// Initializes the font size of this widget.
  pub fn font_size<const M: usize>(self, v: impl DeclareInto<f32, M>) -> Self {
    self.declare_builtin_init(v, Self::get_text_style_widget, |m, v| m.text_style.font_size = v)
  }

  /// Initializes the font face of this widget.
  pub fn font_face<const M: usize>(self, v: impl DeclareInto<FontFace, M>) -> Self {
    self.declare_builtin_init(v, Self::get_text_style_widget, |m, v| m.text_style.font_face = v)
  }

  /// Initializes the letter space of this widget.
  pub fn letter_spacing<const M: usize>(self, v: impl DeclareInto<f32, M>) -> Self {
    self.declare_builtin_init(v, Self::get_text_style_widget, |m, v| m.text_style.letter_space = v)
  }

  /// Initializes the text line height of this widget.
  pub fn text_line_height<const M: usize>(self, v: impl DeclareInto<f32, M>) -> Self {
    self.declare_builtin_init(v, Self::get_text_style_widget, |m, v| m.text_style.line_height = v)
  }

  /// Initializes the text overflow of this widget.
  pub fn text_overflow<const M: usize>(self, v: impl DeclareInto<TextOverflow, M>) -> Self {
    self.declare_builtin_init(v, Self::get_text_style_widget, |m, v| m.text_style.overflow = v)
  }

  /// Initializes the background of the widget.
  pub fn background<const M: usize>(self, v: impl DeclareInto<Option<Brush>, M>) -> Self {
    self.declare_builtin_init(v, Self::get_box_decoration_widget, |m, v| m.background = v)
  }

  /// Initializes the foreground of the widget.
  pub fn foreground<const M: usize>(self, v: impl DeclareInto<Brush, M>) -> Self {
    self.declare_builtin_init(v, Self::get_foreground_widget, |m, v| m.foreground = v)
  }

  /// Initializes the border of the widget.
  pub fn border<const M: usize>(self, v: impl DeclareInto<Option<Border>, M>) -> Self {
    self.declare_builtin_init(v, Self::get_box_decoration_widget, |m, v| m.border = v)
  }

  /// Initializes the border radius of the widget.
  pub fn border_radius<const M: usize>(self, v: impl DeclareInto<Option<Radius>, M>) -> Self {
    self.declare_builtin_init(v, Self::get_box_decoration_widget, |m, v| m.border_radius = v)
  }

  /// Initializes the extra space within the widget.
  pub fn padding<const M: usize>(self, v: impl DeclareInto<EdgeInsets, M>) -> Self {
    self.declare_builtin_init(v, Self::get_padding_widget, |m, v| m.padding = v)
  }

  /// Initializes the cursor of the widget.
  pub fn cursor<const M: usize>(self, v: impl DeclareInto<CursorIcon, M>) -> Self {
    self.declare_builtin_init(v, Self::get_cursor_widget, |m, v| m.cursor = v)
  }

  /// Initializes the space around the widget.
  pub fn margin<const M: usize>(self, v: impl DeclareInto<EdgeInsets, M>) -> Self {
    self.declare_builtin_init(v, Self::get_margin_widget, |m, v| m.margin = v)
  }

  /// Initializes the constraints clamp of the widget.
  pub fn clamp<const M: usize>(self, v: impl DeclareInto<BoxClamp, M>) -> Self {
    self.declare_builtin_init(v, Self::get_constrained_box_widget, |m, v| m.clamp = v)
  }

  /// Initializes how user can scroll the widget.
  pub fn scrollable<const M: usize>(self, v: impl DeclareInto<Scrollable, M>) -> Self {
    self.declare_builtin_init(v, Self::get_scrollable_widget, |m, v| m.scrollable = v)
  }

  /// Initializes the transformation of the widget.
  pub fn transform<const M: usize>(self, v: impl DeclareInto<Transform, M>) -> Self {
    self.declare_builtin_init(v, Self::get_transform_widget, |m, v| m.transform = v)
  }

  /// Initializes how the widget should be aligned horizontally.
  pub fn h_align<const M: usize>(self, v: impl DeclareInto<HAlign, M>) -> Self {
    self.declare_builtin_init(v, Self::get_h_align_widget, |m, v| m.h_align = v)
  }

  /// Initializes how the widget should be aligned vertically.
  pub fn v_align<const M: usize>(self, v: impl DeclareInto<VAlign, M>) -> Self {
    self.declare_builtin_init(v, Self::get_v_align_widget, |m, v| m.v_align = v)
  }

  /// Initializes the relative anchor to the parent of the widget.
  pub fn anchor<const M: usize>(self, v: impl DeclareInto<Anchor, M>) -> Self {
    self.declare_builtin_init(v, Self::get_relative_anchor_widget, |m, v| m.anchor = v)
  }

  /// Initializes the horizontal global anchor of the widget.
  pub fn global_anchor_x<const M: usize>(self, v: impl DeclareInto<GlobalAnchorX, M>) -> Self {
    self.declare_builtin_init(v, Self::get_global_anchor_widget, |m, v| m.global_anchor_x = v)
  }

  /// Initializes the vertical global anchor of the widget.
  pub fn global_anchor_y<const M: usize>(self, v: impl DeclareInto<GlobalAnchorY, M>) -> Self {
    self.declare_builtin_init(v, Self::get_global_anchor_widget, |m, v| m.global_anchor_y = v)
  }

  /// Initializes the visibility of the widget.
  pub fn visible<const M: usize>(self, v: impl DeclareInto<bool, M>) -> Self {
    self.declare_builtin_init(v, Self::get_visibility_widget, |m, v| m.visible = v)
  }

  /// Initializes the opacity of the widget.
  pub fn opacity<const M: usize>(self, v: impl DeclareInto<f32, M>) -> Self {
    self.declare_builtin_init(v, Self::get_opacity_widget, |m, v| m.opacity = v)
  }

  /// Initializes the tooltips of the widget.
  pub fn tooltips<const M: usize>(self, v: impl DeclareInto<CowArc<str>, M>) -> Self {
    self.declare_builtin_init(v, Self::get_tooltips_widget, |m, v| m.tooltips = v)
  }

  /// Initializes the `keep_alive` value of the `KeepAlive` widget.
  pub fn keep_alive<const M: usize>(mut self, v: impl DeclareInto<bool, M>) -> Self {
    let (v, o) = v.declare_into().unzip();
    let d = self.get_keep_alive_widget();
    d.write().keep_alive = v;
    if let Some(o) = o {
      let c_delay = d.clone_writer();

      // KeepAliveWidget may continue to exist after `on_disposed` is fired. It needs
      // to accept value changes to determine when to drop. So instead of
      // unsubscribing in `on_disposed`, we unsubscribe when the widget node is
      // dropped.
      let u = o
        .subscribe(move |(_, v)| {
          c_delay.write().keep_alive = v;
        })
        .unsubscribe_when_dropped();
      self.keep_alive_unsubscribe_handle = Some(Box::new(u));
    }
    self
  }

  /// Initializes the track_id of the widget.
  pub fn track_id(mut self) -> Self {
    self.get_track_id_widget();
    self
  }

  fn declare_builtin_init<V: 'static, B: 'static, const M: usize>(
    mut self, init: impl DeclareInto<V, M>, get_builtin: impl FnOnce(&mut Self) -> &State<B>,
    set_value: fn(&mut B, V),
  ) -> Self {
    let builtin = get_builtin(&mut self);
    let (v, o) = init.declare_into().unzip();
    set_value(&mut *builtin.silent(), v);
    if let Some(o) = o {
      let c_builtin = builtin.clone_writer();
      let u = o.subscribe(move |(_, v)| set_value(&mut *c_builtin.write(), v));
      self.on_disposed(move |_| u.unsubscribe())
    } else {
      self
    }
  }
}

pub trait FatDeclarerExtend: Sized {
  type Target;
  fn finish(this: FatObj<Self>) -> FatObj<Self::Target>;
}

impl<T: FatDeclarerExtend> ObjDeclarer for FatObj<T> {
  type Target = FatObj<T::Target>;

  fn finish(self) -> Self::Target { T::finish(self) }
}

impl FatDeclarerExtend for () {
  type Target = ();
  fn finish(this: FatObj<()>) -> FatObj<()> { this }
}

impl<'w, T, const M: usize> IntoWidgetStrict<'w, M> for FatObj<T>
where
  T: IntoWidget<'w, M>,
{
  fn into_widget_strict(self) -> Widget<'w> { self.map(|w| w.into_widget()).compose() }
}

impl<'a> FatObj<Widget<'a>> {
  fn compose(self) -> Widget<'a> {
    macro_rules! compose_builtin_widgets {
      ($host: ident + [$($field: ident),*]) => {
        $(
          if let Some($field) = self.$field {
            $host = $field.with_child($host).into_widget();
          }
        )*
      };
    }
    let mut host = self.host;
    compose_builtin_widgets!(
      host
        + [
          track_id,
          padding,
          fitted_box,
          foreground,
          box_decoration,
          painting_style,
          text_style,
          scrollable,
          layout_box,
          class,
          constrained_box,
          tooltips,
          margin,
          cursor,
          mix_builtin,
          request_focus,
          transform,
          opacity,
          visibility,
          h_align,
          v_align,
          relative_anchor,
          global_anchor,
          keep_alive
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
  subscribes: SmallVec<[BoxSubscription<'static>; 1]>,
}

impl<T> DeclarerWithSubscription<T> {
  pub fn new(host: T, subscribes: SmallVec<[BoxSubscription<'static>; 1]>) -> Self {
    Self { inner: host, subscribes }
  }
}

impl<T> Deref for DeclarerWithSubscription<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target { &self.inner }
}

impl<T> DerefMut for DeclarerWithSubscription<T> {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.inner }
}

impl<T> DeclarerWithSubscription<T> {
  fn map<M>(self, f: impl FnOnce(T) -> M) -> DeclarerWithSubscription<M> {
    DeclarerWithSubscription { inner: f(self.inner), subscribes: self.subscribes }
  }
}

impl<'w, T, const M: usize> IntoWidgetStrict<'w, M> for DeclarerWithSubscription<T>
where
  T: IntoWidget<'w, M>,
{
  fn into_widget_strict(self) -> Widget<'w> {
    let DeclarerWithSubscription { inner: host, subscribes } = self;
    let w = host.into_widget();
    if subscribes.is_empty() {
      w
    } else {
      fn_widget! {
        let w = FatObj::new(w);
        @ $w {
          on_disposed: move |_| {
            subscribes.into_iter().for_each(|u| u.unsubscribe());
          }
        }
      }
      .into_widget()
    }
  }
}

impl<T: SingleChild> SingleChild for DeclarerWithSubscription<T> {
  fn with_child<'c, const M: usize>(self, child: impl IntoChildSingle<'c, M>) -> Widget<'c> {
    self.map(|w| w.with_child(child)).into_widget()
  }

  fn into_parent(self: Box<Self>) -> Widget<'static> { (*self).into_widget() }
}

impl<T: MultiChild> MultiChild for DeclarerWithSubscription<T> {
  fn with_child<'c, const N: usize, const M: usize>(
    self, child: impl IntoChildMulti<'c, N, M>,
  ) -> MultiPair<'c> {
    MultiPair::new(self, child)
  }

  fn into_parent(self: Box<Self>) -> Widget<'static> { (*self).into_widget() }
}

impl<'w, T, C, const TML: bool, const WRITER: bool, const N: usize, const M: usize>
  ComposeWithChild<'w, C, WRITER, TML, N, M> for DeclarerWithSubscription<T>
where
  T: ComposeWithChild<'w, C, WRITER, TML, N, M>,
{
  type Target = DeclarerWithSubscription<T::Target>;

  fn with_child(self, child: C) -> Self::Target { self.map(|host| host.with_child(child)) }
}

impl Declare for FatObj<()> {
  type Builder = Self;
  fn declarer() -> Self::Builder { FatObj::default() }
}
