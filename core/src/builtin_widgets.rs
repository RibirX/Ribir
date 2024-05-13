//! Built-in widgets is a set of minimal widgets that describes the most common
//! UI elements. The most of them can be used to extend other object in the
//! declare syntax, so other objects can use the builtin fields and methods like
//! self fields and methods.

pub mod key;
use std::cell::Cell;

pub use key::{Key, KeyWidget};
pub mod image_widget;
pub mod keep_alive;
pub use keep_alive::*;
mod theme;
use ribir_algo::Sc;
pub use theme::*;
mod cursor;
pub use cursor::*;
pub use winit::window::CursorIcon;
mod margin;
pub use margin::*;
mod padding;
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

pub mod has_focus;
pub use has_focus::*;
pub mod mouse_hover;
pub use mouse_hover::*;
pub mod clip;
pub use clip::*;
pub mod pointer_pressed;
pub use pointer_pressed::*;
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

use crate::prelude::*;

#[derive(Clone)]
/// LazyWidgetId is a widget id that will be valid after widget build.
pub struct LazyWidgetId(Sc<Cell<Option<WidgetId>>>);

/// A fat object that extend the `T` object with all builtin widgets ability. A
/// `FatObj` will create during the compose phase, and compose with the builtin
/// widgets it actually use, and drop after composed.
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
///     .build(ctx)
/// };
/// ```
pub struct FatObj<T> {
  host: T,
  host_id: LazyWidgetId,
  id: LazyWidgetId,
  mix_builtin: Option<State<MixBuiltin>>,
  request_focus: Option<State<RequestFocus>>,
  has_focus: Option<State<HasFocus>>,
  mouse_hover: Option<State<MouseHover>>,
  pointer_pressed: Option<State<PointerPressed>>,
  fitted_box: Option<State<FittedBox>>,
  box_decoration: Option<State<BoxDecoration>>,
  padding: Option<State<Padding>>,
  layout_box: Option<State<LayoutBox>>,
  cursor: Option<State<Cursor>>,
  margin: Option<State<Margin>>,
  scrollable: Option<State<ScrollableWidget>>,
  transform: Option<State<TransformWidget>>,
  h_align: Option<State<HAlignWidget>>,
  v_align: Option<State<VAlignWidget>>,
  relative_anchor: Option<State<RelativeAnchor>>,
  global_anchor: Option<State<GlobalAnchor>>,
  visibility: Option<State<Visibility>>,
  opacity: Option<State<Opacity>>,
  keep_alive: Option<State<KeepAlive>>,
  keep_alive_unsubscribe_handle: Option<Box<dyn Any>>,
}

impl LazyWidgetId {
  pub fn id(&self) -> Option<WidgetId> { self.0.get() }

  pub fn assert_id(&self) -> WidgetId { self.0.get().unwrap() }

  fn set(&self, wid: WidgetId) { self.0.set(Some(wid)); }

  fn ref_count(&self) -> usize { self.0.ref_count() }
}

impl Default for LazyWidgetId {
  fn default() -> Self { Self(Sc::new(Cell::new(None))) }
}

impl<T> FatObj<T> {
  /// Create a new `FatObj` with the given host object.
  pub fn new(host: T) -> Self {
    Self {
      host,
      host_id: LazyWidgetId::default(),
      id: LazyWidgetId::default(),
      mix_builtin: None,
      request_focus: None,
      has_focus: None,
      mouse_hover: None,
      pointer_pressed: None,
      fitted_box: None,
      box_decoration: None,
      padding: None,
      layout_box: None,
      cursor: None,
      margin: None,
      scrollable: None,
      transform: None,
      h_align: None,
      v_align: None,
      relative_anchor: None,
      global_anchor: None,
      visibility: None,
      opacity: None,
      keep_alive: None,
      keep_alive_unsubscribe_handle: None,
    }
  }

  /// Maps an `FatObj<T>` to `FatObj<V>` by applying a function to the host
  /// object.
  #[track_caller]
  pub fn map<V>(self, f: impl FnOnce(T) -> V) -> FatObj<V> {
    FatObj {
      host: f(self.host),
      host_id: self.host_id,
      id: self.id,
      mix_builtin: self.mix_builtin,
      request_focus: self.request_focus,
      has_focus: self.has_focus,
      mouse_hover: self.mouse_hover,
      pointer_pressed: self.pointer_pressed,
      fitted_box: self.fitted_box,
      box_decoration: self.box_decoration,
      padding: self.padding,
      layout_box: self.layout_box,
      cursor: self.cursor,
      margin: self.margin,
      scrollable: self.scrollable,
      transform: self.transform,
      h_align: self.h_align,
      v_align: self.v_align,
      relative_anchor: self.relative_anchor,
      global_anchor: self.global_anchor,
      visibility: self.visibility,
      opacity: self.opacity,
      keep_alive: self.keep_alive,
      keep_alive_unsubscribe_handle: self.keep_alive_unsubscribe_handle,
    }
  }

  /// Return true if the FatObj not contains any builtin widgets.
  pub fn is_empty(&self) -> bool {
    self.host_id.ref_count() == 1
      && self.id.ref_count() == 1
      && self.mix_builtin.is_none()
      && self.request_focus.is_none()
      && self.has_focus.is_none()
      && self.mouse_hover.is_none()
      && self.pointer_pressed.is_none()
      && self.fitted_box.is_none()
      && self.box_decoration.is_none()
      && self.padding.is_none()
      && self.layout_box.is_none()
      && self.cursor.is_none()
      && self.margin.is_none()
      && self.scrollable.is_none()
      && self.transform.is_none()
      && self.h_align.is_none()
      && self.v_align.is_none()
      && self.relative_anchor.is_none()
      && self.global_anchor.is_none()
      && self.visibility.is_none()
      && self.opacity.is_none()
      && self.keep_alive.is_none()
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

  /// Return the LazyWidgetId of the host widget, through which you can access
  /// the WidgetId after building.
  pub fn lazy_host_id(&self) -> LazyWidgetId { self.host_id.clone() }

  /// Return the LazyWidgetId point to WidgetId of the root of the sub widget
  /// tree after the FatObj has built.
  pub fn lazy_id(&self) -> LazyWidgetId { self.id.clone() }
}

// builtin widgets accessors
impl<T> FatObj<T> {
  pub fn get_mix_builtin_widget(&mut self) -> &mut State<MixBuiltin> {
    self
      .mix_builtin
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  /// Returns the `State<RequestFocus>` widget from the FatObj. If it doesn't
  /// exist, a new one is created.
  pub fn get_request_focus_widget(&mut self) -> &mut State<RequestFocus> {
    self
      .request_focus
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  /// Returns the `State<HasFocus>` widget from the FatObj. If it doesn't exist,
  /// a new one is created.
  pub fn get_has_focus_widget(&mut self) -> &mut State<HasFocus> {
    self
      .has_focus
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  /// Returns the `State<MouseHover>` widget from the FatObj. If it doesn't
  /// exist, a new one is created.
  pub fn get_mouse_hover_widget(&mut self) -> &mut State<MouseHover> {
    self
      .mouse_hover
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  /// Returns the `State<PointerPressed>` widget from the FatObj. If it doesn't
  /// exist, a new one is created.
  pub fn get_pointer_pressed_widget(&mut self) -> &mut State<PointerPressed> {
    self
      .pointer_pressed
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  /// Returns the `State<FittedBox>` widget from the FatObj. If it doesn't
  /// exist, a new one is created.
  pub fn get_fitted_box_widget(&mut self) -> &mut State<FittedBox> {
    self
      .fitted_box
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  /// Returns the `State<BoxDecoration>` widget from the FatObj. If it doesn't
  /// exist, a new one is created.
  pub fn get_box_decoration_widget(&mut self) -> &mut State<BoxDecoration> {
    self
      .box_decoration
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  /// Returns the `State<Padding>` widget from the FatObj. If it doesn't exist,
  /// a new one is created.
  pub fn get_padding_widget(&mut self) -> &mut State<Padding> {
    self
      .padding
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  /// Returns the `State<LayoutBox>` widget from the FatObj. If it doesn't
  /// exist, a new one is created.
  pub fn get_layout_box_widget(&mut self) -> &mut State<LayoutBox> {
    self
      .layout_box
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  /// Returns the `State<Cursor>` widget from the FatObj. If it doesn't exist, a
  /// new one is created.
  pub fn get_cursor_widget(&mut self) -> &mut State<Cursor> {
    self
      .cursor
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  /// Returns the `State<Margin>` widget from the FatObj. If it doesn't exist, a
  /// new one is created.
  pub fn get_margin_widget(&mut self) -> &mut State<Margin> {
    self
      .margin
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  /// Returns the `State<ScrollableWidget>` widget from the FatObj. If it
  /// doesn't exist, a new one is created.
  pub fn get_scrollable_widget(&mut self) -> &mut State<ScrollableWidget> {
    self
      .scrollable
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  /// Returns the `State<TransformWidget>` widget from the FatObj. If it doesn't
  /// exist, a new one is created.
  pub fn get_transform_widget(&mut self) -> &mut State<TransformWidget> {
    self
      .transform
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  /// Returns the `State<HAlignWidget>` widget from the FatObj. If it doesn't
  /// exist, a new one is created.
  pub fn get_h_align_widget(&mut self) -> &mut State<HAlignWidget> {
    self
      .h_align
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  /// Returns the `State<VAlignWidget>` widget from the FatObj. If it doesn't
  /// exist, a new one is created.
  pub fn get_v_align_widget(&mut self) -> &mut State<VAlignWidget> {
    self
      .v_align
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  /// Returns the `State<RelativeAnchor>` widget from the FatObj. If it doesn't
  /// exist, a new one is created.
  pub fn get_relative_anchor_widget(&mut self) -> &mut State<RelativeAnchor> {
    self
      .relative_anchor
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  /// Returns the `State<GlobalAnchor>` widget from the FatObj. If it doesn't
  /// exist, a new one is created.
  pub fn get_global_anchor_widget(&mut self) -> &mut State<GlobalAnchor> {
    self
      .global_anchor
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  /// Returns the `State<Visibility>` widget from the FatObj. If it doesn't
  /// exist, a new one is created.
  pub fn get_visibility_widget(&mut self) -> &mut State<Visibility> {
    self
      .visibility
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  /// Returns the `State<Opacity>` widget from the FatObj. If it doesn't exist,
  /// a new one is created.
  pub fn get_opacity_widget(&mut self) -> &mut State<Opacity> {
    self
      .opacity
      .get_or_insert_with(|| State::value(<_>::default()))
  }

  /// Returns the `State<KeepAlive>` widget from the FatObj. If it doesn't
  /// exist, a new one is created.
  pub fn get_keep_alive_widget(&mut self) -> &mut State<KeepAlive> {
    self
      .keep_alive
      .get_or_insert_with(|| State::value(<_>::default()))
  }
}

macro_rules! on_mixin {
  ($this:ident, $on_method:ident, $f:ident) => {{
    $this
      .get_mix_builtin_widget()
      .read()
      .$on_method($f);
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
      .read()
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
      .read()
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
  pub fn tab_index<V, M>(self, tab_idx: V) -> Self
  where
    DeclareInit<i16>: DeclareFrom<V, M>,
  {
    self.declare_builtin_init(
      DeclareFrom::declare_from(tab_idx),
      Self::get_mix_builtin_widget,
      |mixin, v| {
        mixin.set_tab_index(v);
      },
    )
  }

  /// Initializes whether the `widget` should automatically get focus when the
  /// window loads.
  ///
  /// Only one widget should have this attribute specified.  If there are
  /// several, the widget nearest the root, get the initial focus.
  pub fn auto_focus<V, M>(self, v: V) -> Self
  where
    DeclareInit<bool>: DeclareFrom<V, M>,
  {
    self.declare_builtin_init(DeclareFrom::declare_from(v), Self::get_mix_builtin_widget, |m, v| {
      m.set_auto_focus(v);
    })
  }

  /// Initializes how its child should be scale to fit its box.
  pub fn box_fit<V, M>(self, v: V) -> Self
  where
    DeclareInit<BoxFit>: DeclareFrom<V, M>,
  {
    self.declare_builtin_init(DeclareFrom::declare_from(v), Self::get_fitted_box_widget, |m, v| {
      m.box_fit = v
    })
  }

  /// Initializes the background of the widget.
  pub fn background<V, M>(self, v: V) -> Self
  where
    DeclareInit<Option<Brush>>: DeclareFrom<V, M>,
  {
    self.declare_builtin_init(
      DeclareFrom::declare_from(v),
      Self::get_box_decoration_widget,
      |m, v| m.background = v,
    )
  }

  /// Initializes the border of the widget.
  pub fn border<V, M>(self, v: V) -> Self
  where
    DeclareInit<Option<Border>>: DeclareFrom<V, M>,
  {
    self.declare_builtin_init(
      DeclareFrom::declare_from(v),
      Self::get_box_decoration_widget,
      |m, v| m.border = v,
    )
  }

  /// Initializes the border radius of the widget.
  pub fn border_radius<V, M>(self, v: V) -> Self
  where
    DeclareInit<Option<Radius>>: DeclareFrom<V, M>,
  {
    self.declare_builtin_init(
      DeclareFrom::declare_from(v),
      Self::get_box_decoration_widget,
      |m, v| m.border_radius = v,
    )
  }

  /// Initializes the extra space within the widget.
  pub fn padding<V, M>(self, v: V) -> Self
  where
    DeclareInit<EdgeInsets>: DeclareFrom<V, M>,
  {
    self.declare_builtin_init(DeclareFrom::declare_from(v), Self::get_padding_widget, |m, v| {
      m.padding = v
    })
  }

  /// Initializes the cursor of the widget.
  pub fn cursor<V, M>(self, v: V) -> Self
  where
    DeclareInit<CursorIcon>: DeclareFrom<V, M>,
  {
    self.declare_builtin_init(DeclareFrom::declare_from(v), Self::get_cursor_widget, |m, v| {
      m.cursor = v
    })
  }

  /// Initializes the space around the widget.
  pub fn margin<V, M>(self, v: V) -> Self
  where
    DeclareInit<EdgeInsets>: DeclareFrom<V, M>,
  {
    self.declare_builtin_init(DeclareFrom::declare_from(v), Self::get_margin_widget, |m, v| {
      m.margin = v
    })
  }

  /// Initializes how user can scroll the widget.
  pub fn scrollable<V, M>(self, v: V) -> Self
  where
    DeclareInit<Scrollable>: DeclareFrom<V, M>,
  {
    self.declare_builtin_init(DeclareFrom::declare_from(v), Self::get_scrollable_widget, |m, v| {
      m.scrollable = v
    })
  }

  /// Initializes the position of the widget's scroll.
  pub fn scroll_pos<V, M>(self, v: V) -> Self
  where
    DeclareInit<Point>: DeclareFrom<V, M>,
  {
    self.declare_builtin_init(DeclareFrom::declare_from(v), Self::get_scrollable_widget, |m, v| {
      m.scroll_pos = v
    })
  }

  /// Initializes the transformation of the widget.
  pub fn transform<V, M>(self, v: V) -> Self
  where
    DeclareInit<Transform>: DeclareFrom<V, M>,
  {
    self.declare_builtin_init(DeclareFrom::declare_from(v), Self::get_transform_widget, |m, v| {
      m.transform = v
    })
  }

  /// Initializes how the widget should be aligned horizontally.
  pub fn h_align<V, M>(self, v: V) -> Self
  where
    DeclareInit<HAlign>: DeclareFrom<V, M>,
  {
    self.declare_builtin_init(DeclareFrom::declare_from(v), Self::get_h_align_widget, |m, v| {
      m.h_align = v
    })
  }

  /// Initializes how the widget should be aligned vertically.
  pub fn v_align<V, M>(self, v: V) -> Self
  where
    DeclareInit<VAlign>: DeclareFrom<V, M>,
  {
    self.declare_builtin_init(DeclareFrom::declare_from(v), Self::get_v_align_widget, |m, v| {
      m.v_align = v
    })
  }

  /// Initializes the relative anchor to the parent of the widget.
  pub fn anchor<V, M>(self, v: V) -> Self
  where
    DeclareInit<Anchor>: DeclareFrom<V, M>,
  {
    self.declare_builtin_init(
      DeclareFrom::declare_from(v),
      Self::get_relative_anchor_widget,
      |m, v| m.anchor = v,
    )
  }

  /// Initializes the global anchor of the widget.
  pub fn global_anchor<V, M>(self, v: V) -> Self
  where
    DeclareInit<Anchor>: DeclareFrom<V, M>,
  {
    self.declare_builtin_init(
      DeclareFrom::declare_from(v),
      Self::get_global_anchor_widget,
      |m, v| m.global_anchor = v,
    )
  }

  /// Initializes the visibility of the widget.
  pub fn visible<V, M>(self, v: V) -> Self
  where
    DeclareInit<bool>: DeclareFrom<V, M>,
  {
    self.declare_builtin_init(DeclareFrom::declare_from(v), Self::get_visibility_widget, |m, v| {
      m.visible = v
    })
  }

  /// Initializes the opacity of the widget.
  pub fn opacity<V, M>(self, v: V) -> Self
  where
    DeclareInit<f32>: DeclareFrom<V, M>,
  {
    self.declare_builtin_init(DeclareFrom::declare_from(v), Self::get_opacity_widget, |m, v| {
      m.opacity = v
    })
  }

  /// Initializes the `keep_alive` value of the `KeepAlive` widget.
  pub fn keep_alive<V, M>(mut self, v: V) -> Self
  where
    DeclareInit<bool>: DeclareFrom<V, M>,
  {
    let (v, o) = DeclareInit::declare_from(v).unzip();
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

  fn declare_builtin_init<V: 'static, B: 'static>(
    mut self, init: DeclareInit<V>, get_builtin: impl FnOnce(&mut Self) -> &mut State<B>,
    set_value: fn(&mut B, V),
  ) -> Self {
    let builtin = get_builtin(&mut self);
    let (v, o) = init.unzip();
    set_value(&mut *builtin.write(), v);
    if let Some(o) = o {
      let c_builtin = builtin.clone_writer();
      let u = o.subscribe(move |(_, v)| {
        set_value(&mut *c_builtin.write(), v);
      });
      self.on_disposed(move |_| u.unsubscribe())
    } else {
      self
    }
  }
}

impl<T> ObjDeclarer for FatObj<T> {
  type Target = Self;

  fn finish(self, _: &BuildCtx) -> Self::Target { self }
}

impl<T: SingleChild> SingleChild for FatObj<T> {}
impl<T: MultiChild> MultiChild for FatObj<T> {}

crate::widget::multi_build_replace_impl! {
  impl<T: {#} > {#} for FatObj<T> {
    #[track_caller]
    fn build(self, ctx: &BuildCtx) -> Widget {
      self.map(|host| host.build(ctx)).build(ctx)
    }
  }
}

impl WidgetBuilder for FatObj<Widget> {
  #[inline]
  #[track_caller]
  fn build(self, ctx: &BuildCtx) -> Widget {
    let mut host = self.host;
    self.host_id.set(host.id());
    if let Some(mix_builtin) = self.mix_builtin {
      host = mix_builtin.with_child(host, ctx).build(ctx)
    }
    if let Some(request_focus) = self.request_focus {
      host = request_focus.with_child(host, ctx).build(ctx);
    }
    if let Some(has_focus) = self.has_focus {
      host = has_focus.with_child(host, ctx).build(ctx);
    }
    if let Some(mouse_hover) = self.mouse_hover {
      host = mouse_hover.with_child(host, ctx).build(ctx);
    }
    if let Some(pointer_pressed) = self.pointer_pressed {
      host = pointer_pressed.with_child(host, ctx).build(ctx);
    }
    if let Some(fitted_box) = self.fitted_box {
      host = fitted_box.with_child(host, ctx).build(ctx);
    }
    if let Some(box_decoration) = self.box_decoration {
      host = box_decoration.with_child(host, ctx).build(ctx);
    }
    if let Some(padding) = self.padding {
      host = padding.with_child(host, ctx).build(ctx);
    }
    if let Some(layout_box) = self.layout_box {
      host = layout_box.with_child(host, ctx).build(ctx);
    }
    if let Some(cursor) = self.cursor {
      host = cursor.with_child(host, ctx).build(ctx);
    }
    if let Some(margin) = self.margin {
      host = margin.with_child(host, ctx).build(ctx);
    }
    if let Some(scrollable) = self.scrollable {
      host = scrollable.with_child(host, ctx).build(ctx);
    }
    if let Some(transform) = self.transform {
      host = transform.with_child(host, ctx).build(ctx);
    }
    if let Some(h_align) = self.h_align {
      host = h_align.with_child(host, ctx).build(ctx);
    }
    if let Some(v_align) = self.v_align {
      host = v_align.with_child(host, ctx).build(ctx);
    }
    if let Some(relative_anchor) = self.relative_anchor {
      host = relative_anchor.with_child(host, ctx).build(ctx);
    }
    if let Some(global_anchor) = self.global_anchor {
      host = global_anchor.with_child(host, ctx).build(ctx);
    }
    if let Some(visibility) = self.visibility {
      host = visibility.with_child(host, ctx).build(ctx);
    }
    if let Some(opacity) = self.opacity {
      host = opacity.with_child(host, ctx).build(ctx);
    }
    if let Some(keep_alive) = self.keep_alive {
      host = keep_alive.with_child(host, ctx).build(ctx);
    }
    if let Some(h) = self.keep_alive_unsubscribe_handle {
      let arena = &mut ctx.tree.borrow_mut().arena;
      host.id().attach_anonymous_data(h, arena);
    }
    self.id.set(host.id());
    host
  }
}

impl<T: ComposeWithChild<C, M>, C, M> ComposeWithChild<C, M> for FatObj<T> {
  type Target = FatObj<T::Target>;

  #[inline]
  #[track_caller]
  fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target {
    self.map(
      #[cfg_attr(feature = "nightly", track_caller)]
      |host| host.with_child(child, ctx),
    )
  }
}

impl<C> SingleWithChild<C, ()> for FatObj<()> {
  type Target = FatObj<C>;

  #[inline]
  #[track_caller]
  fn with_child(self, child: C, _: &BuildCtx) -> Self::Target { self.map(move |_| child) }
}

impl<T: PairWithChild<C>, C> PairWithChild<C> for FatObj<T> {
  type Target = Pair<FatObj<T>, C>;

  #[inline]
  #[track_caller]
  fn with_child(self, child: C, _: &BuildCtx) -> Self::Target { Pair::new(self, child) }
}

impl<T: SingleParent + 'static> SingleParent for FatObj<T> {
  #[track_caller]
  fn compose_child(self, child: Widget, ctx: &BuildCtx) -> Widget {
    self
      .map(|host| host.compose_child(child, ctx))
      .build(ctx)
  }
}

impl<T: MultiParent + 'static> MultiParent for FatObj<T> {
  #[track_caller]
  fn compose_children(self, children: impl Iterator<Item = Widget>, ctx: &BuildCtx) -> Widget {
    self
      .map(|host| host.compose_children(children, ctx))
      .build(ctx)
  }
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
