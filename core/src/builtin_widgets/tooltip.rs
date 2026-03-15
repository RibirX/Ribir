use std::{cell::RefCell, mem};

use ribir_algo::Rc;

use crate::{declare::ValueKind, prelude::*};

class_names! {
  #[doc = "Class name for tooltip content"]
  TOOLTIP,
}

/// A provider contract for overriding the default tooltip behavior.
///
/// `Tooltip` resolves this lazily during `compose_child`, so providers declared
/// on the same node through `providers:` can override the built-in fallback.
///
/// The closure receives:
/// - the host widget being wrapped
/// - the tooltip text payload
///
/// It must return the wrapped widget together with the tooltip control object.
type TooltipComposeFn =
  Box<dyn for<'r> Fn(Widget<'r>, TextValue) -> (Widget<'r>, Box<dyn TooltipControl>) + 'static>;

pub struct CustomTooltip(pub TooltipComposeFn);

#[doc(hidden)]
pub trait TooltipControl {
  fn show(&mut self);
  fn hide(&mut self);
  fn is_showing(&self) -> bool;
}

impl<T> TooltipControl for Rc<RefCell<T>>
where
  T: TooltipControl,
{
  fn show(&mut self) { self.borrow_mut().show(); }

  fn hide(&mut self) { self.borrow_mut().hide(); }

  fn is_showing(&self) -> bool { self.borrow().is_showing() }
}

enum TooltipInner {
  Init(TextValue),
  Ready(Box<dyn TooltipControl>),
  Binding,
}

impl Default for TooltipInner {
  fn default() -> Self { Self::Init(PipeValue::Value(CowArc::default())) }
}

/// Adds tooltip behavior to a widget declarer.
///
/// This built-in `FatObj` field attaches declarative tooltip semantics to the
/// host widget. The actual behavior is resolved lazily:
///
/// - If a [`CustomTooltip`] provider is visible, it handles the tooltip.
/// - Otherwise, a lightweight fallback mounts a plain tooltip into the window.
///
/// # Example
///
/// ```rust no_run
/// use ribir::prelude::*;
///
/// text! {
///   text: "Hover me",
///   tooltip: "I'm a tooltip!",
/// };
/// ```
pub struct Tooltip(Rc<RefCell<TooltipInner>>);

impl Default for Tooltip {
  fn default() -> Self { Self::from_text(PipeValue::Value(CowArc::default())) }
}

impl Clone for Tooltip {
  fn clone(&self) -> Self { Self(self.0.clone()) }
}

struct FallbackTooltip {
  reusable: Reusable,
  mounted: Option<MountHandle>,
  wnd: Rc<Window>,
  host: TrackId,
}

impl FallbackTooltip {
  fn new(tooltip: TextValue, wnd: Rc<Window>, host: TrackId) -> Self {
    Self {
      reusable: Reusable::new(text! { text: tooltip, class: TOOLTIP }),
      mounted: None,
      wnd,
      host,
    }
  }

  fn mount(&mut self, id: WidgetId) {
    if self.mounted.is_some() {
      return;
    }

    let point = Tooltip::mount_point(&self.wnd, id);
    let mut tooltip = FatObj::new(self.reusable.get_widget());
    tooltip.with_x(point.x);
    tooltip.with_y(point.y);
    self.mounted = Some(self.wnd.mount(tooltip.into_widget()));
  }

  fn close(&mut self) {
    if let Some(mounted) = self.mounted.take() {
      mounted.close();
    }
  }
}

impl TooltipControl for FallbackTooltip {
  fn show(&mut self) {
    if let Some(id) = self.host.get() {
      self.mount(id);
    } else {
      self.close();
    }
  }

  fn hide(&mut self) { self.close(); }

  fn is_showing(&self) -> bool { self.mounted.is_some() }
}

impl Declare for Tooltip {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl Tooltip {
  pub fn new<K: ?Sized>(text: impl RInto<TextValue, K>) -> Self { Self::from_text(text.r_into()) }

  fn from_text(tooltip: TextValue) -> Self {
    Self(Rc::new(RefCell::new(TooltipInner::Init(tooltip))))
  }

  fn mount_point(wnd: &Window, id: WidgetId) -> Point {
    let size = wnd.widget_size(id).unwrap_or_default();
    let pos = wnd.widget_pos(id).unwrap_or_default();
    let global_pos = id
      .parent(wnd.tree())
      .map_or(pos, |p| wnd.map_to_global(pos, p));

    Point::new(global_pos.x + size.width / 2., global_pos.y + size.height + 4.)
  }

  pub fn show(&self) { self.with_control(|c| c.show()); }

  pub fn hide(&self) { self.with_control(|c| c.hide()); }

  pub fn is_showing(&self) -> bool {
    self
      .with_control(|c| c.is_showing())
      .unwrap_or(false)
  }

  fn with_control<R>(&self, f: impl FnOnce(&mut dyn TooltipControl) -> R) -> Option<R> {
    let mut inner = self.0.borrow_mut();
    if let TooltipInner::Ready(control) = &mut *inner { Some(f(&mut **control)) } else { None }
  }

  fn compose_fallback<'c>(
    child: Widget<'c>, text: TextValue,
  ) -> (Widget<'c>, Box<dyn TooltipControl>) {
    let wnd = BuildCtx::get().window();
    let mut child = FatObj::new(child);
    let control = Rc::new(RefCell::new(FallbackTooltip::new(text, wnd, child.track_id())));
    let child = Self::bind_hover_focus(child, control.clone());
    (child, Box::new(control))
  }

  /// Wire tooltip show/hide to the host's hover and focus state, and ensure
  /// cleanup on dispose. Returns the wrapped host widget.
  ///
  /// This is the canonical wiring shared by all tooltip implementations.
  pub fn bind_hover_focus<'c, T: TooltipControl + 'static>(
    mut host: FatObj<Widget<'c>>, control: Rc<RefCell<T>>,
  ) -> Widget<'c> {
    let subscription = watch!(*$read(host.is_hovered()) || *$read(host.is_focused())).subscribe({
      let control = control.clone();
      move |active| {
        let mut control = control.borrow_mut();
        if active {
          control.show();
        } else {
          control.hide();
        }
      }
    });

    host.on_disposed({
      let control = control.clone();
      move |_| {
        control.borrow_mut().hide();
        subscription.unsubscribe();
      }
    });
    host.into_widget()
  }
}

impl<T, K: ?Sized> RFrom<T, ValueKind<K>> for Tooltip
where
  TextValue: RFrom<T, K>,
{
  fn r_from(value: T) -> Self { Self::from_text(TextValue::r_from(value)) }
}

impl<'c> ComposeChild<'c> for Tooltip {
  type Child = Widget<'c>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    let Ok(tooltip) = this.try_into_value() else {
      panic!("Tooltip should be a stateless widget");
    };

    fn_widget! {
      let text = {
        let mut inner = tooltip.0.borrow_mut();
        match mem::replace(&mut *inner, TooltipInner::Binding) {
          TooltipInner::Init(text) => text,
          TooltipInner::Ready(control) => {
            *inner = TooltipInner::Ready(control);
            panic!("A Tooltip instance can only be bound to one host.");
          }
          TooltipInner::Binding => panic!("Tooltip binding re-entered unexpectedly."),
        }
      };

      let custom = Provider::of::<CustomTooltip>(BuildCtx::get());
      let (child, control) = if let Some(c) = custom {
        (c.0)(child, text)
      } else {
        Tooltip::compose_fallback(child, text)
      };
      *tooltip.0.borrow_mut() = TooltipInner::Ready(control);
      child
    }
    .into_widget()
  }
}

#[cfg(test)]
mod tests {
  use std::{cell::Cell, rc::Rc};

  use crate::{prelude::*, reset_test_env, test_helper::*};

  struct NoopTooltipControl;

  impl TooltipControl for NoopTooltipControl {
    fn show(&mut self) {}

    fn hide(&mut self) {}

    fn is_showing(&self) -> bool { false }
  }

  #[test]
  fn tooltip_manual_control_is_noop_before_binding() {
    let tooltip = Tooltip::new("tip");

    tooltip.show();
    assert!(!tooltip.is_showing());

    tooltip.hide();
    assert!(!tooltip.is_showing());
  }

  #[test]
  fn fallback_tooltip_mounts_on_hover() {
    reset_test_env!();

    let wnd = TestWindow::new_with_size(
      fn_widget! {
        @MockBox {
          size: Size::new(40., 20.),
          tooltip: "tip",
        }
      },
      Size::new(120., 80.),
    );
    wnd.draw_frame();

    let before = wnd.tree().count(wnd.tree().root());
    wnd.process_cursor_move(Point::new(10., 10.));
    wnd.draw_frame();
    let after = wnd.tree().count(wnd.tree().root());

    assert!(after > before, "tooltip should mount extra content when hovered");

    wnd.process_cursor_move(Point::new(100., 70.));
    wnd.draw_frame();
    assert_eq!(wnd.tree().count(wnd.tree().root()), before);

    wnd.process_cursor_move(Point::new(10., 10.));
    wnd.draw_frame();
    assert_eq!(wnd.tree().count(wnd.tree().root()), after);
  }

  #[test]
  fn empty_tooltip_still_mounts() {
    reset_test_env!();

    let wnd = TestWindow::new_with_size(
      fn_widget! {
        @MockBox {
          size: Size::new(40., 20.),
          tooltip: "",
        }
      },
      Size::new(120., 80.),
    );
    wnd.draw_frame();

    let before = wnd.tree().count(wnd.tree().root());
    wnd.process_cursor_move(Point::new(10., 10.));
    wnd.draw_frame();
    assert!(wnd.tree().count(wnd.tree().root()) > before);
  }

  #[test]
  fn tooltip_visibility_tracks_text_changes_while_hovered() {
    reset_test_env!();

    let (tooltip_text, tooltip_text_writer) = split_value(String::new());
    let wnd = TestWindow::new_with_size(
      fn_widget! {
        @MockBox {
          size: Size::new(40., 20.),
          tooltip: pipe!($read(tooltip_text).clone()),
        }
      },
      Size::new(120., 80.),
    );
    wnd.draw_frame();

    let before = wnd.tree().count(wnd.tree().root());
    wnd.process_cursor_move(Point::new(10., 10.));
    wnd.draw_frame();
    let shown = wnd.tree().count(wnd.tree().root());
    assert!(shown > before, "tooltip should mount while hovered even when text is empty");

    *tooltip_text_writer.write() = "tip".into();
    wnd.draw_frame();
    assert_eq!(wnd.tree().count(wnd.tree().root()), shown);

    *tooltip_text_writer.write() = String::new();
    wnd.draw_frame();
    assert_eq!(wnd.tree().count(wnd.tree().root()), shown);
  }

  #[test]
  fn fallback_tooltip_mounts_on_focus() {
    reset_test_env!();

    let wnd = TestWindow::new_with_size(
      fn_widget! {
        @MockBox {
          size: Size::new(40., 20.),
          tab_index: 0i16,
          tooltip: "tip",
        }
      },
      Size::new(120., 80.),
    );
    wnd.draw_frame();

    let before = wnd.tree().count(wnd.tree().root());
    wnd.process_cursor_move(Point::new(10., 10.));
    wnd.process_mouse_press(Box::new(DummyDeviceId), MouseButtons::PRIMARY);
    wnd.process_mouse_release(Box::new(DummyDeviceId), MouseButtons::PRIMARY);
    wnd.draw_frame();
    let after_focus = wnd.tree().count(wnd.tree().root());
    assert!(after_focus > before, "tooltip should mount when focused");

    wnd.process_cursor_move(Point::new(100., 70.));
    wnd
      .focus_mgr
      .borrow_mut()
      .blur(FocusReason::Other);
    wnd.draw_frame();
    assert_eq!(wnd.tree().count(wnd.tree().root()), before);
  }

  #[test]
  fn same_node_provider_can_override_tooltip() {
    reset_test_env!();

    let hit = Rc::new(Cell::new(0usize));
    let hit_in_widget = hit.clone();

    let wnd = TestWindow::new_with_size(
      fn_widget! {
        let hit = hit_in_widget.clone();
        let custom = CustomTooltip(Box::new(move |child, _| {
          hit.set(hit.get() + 1);
          (child, Box::new(NoopTooltipControl))
        }));
        @MockBox {
          size: Size::new(40., 20.),
          providers: [Provider::new(custom)],
          tooltip: "tip",
        }
      },
      Size::new(120., 80.),
    );
    wnd.draw_frame();

    assert_eq!(hit.get(), 1);
  }

  #[test]
  fn fallback_tooltip_can_be_manually_controlled() {
    reset_test_env!();

    let tooltip = Tooltip::new("tip");
    let tooltip_in_widget = tooltip.clone();
    let wnd = TestWindow::new_with_size(
      fn_widget! {
        let tooltip = tooltip_in_widget.clone();
        @MockBox {
          size: Size::new(40., 20.),
          tooltip: tooltip,
        }
      },
      Size::new(120., 80.),
    );
    wnd.draw_frame();

    let before = wnd.tree().count(wnd.tree().root());
    tooltip.show();
    wnd.draw_frame();
    let shown = wnd.tree().count(wnd.tree().root());

    assert!(tooltip.is_showing());
    assert!(shown > before, "manual show should mount tooltip content");

    wnd.process_cursor_move(Point::new(100., 70.));
    wnd.draw_frame();
    assert_eq!(wnd.tree().count(wnd.tree().root()), shown);

    tooltip.hide();
    wnd.draw_frame();
    assert!(!tooltip.is_showing());
    assert_eq!(wnd.tree().count(wnd.tree().root()), before);
  }

  #[test]
  fn tooltip_wrapper_can_reuse_manual_control() {
    reset_test_env!();

    let tooltip = Tooltip::new("tip");
    let tooltip_in_widget = tooltip.clone();
    let wnd = TestWindow::new_with_size(
      fn_widget! {
        let tooltip = tooltip_in_widget.clone();
        @Tooltip {
          tooltip: tooltip,
          @MockBox { size: Size::new(40., 20.) }
        }
      },
      Size::new(120., 80.),
    );
    wnd.draw_frame();

    let before = wnd.tree().count(wnd.tree().root());
    tooltip.show();
    wnd.draw_frame();

    assert!(wnd.tree().count(wnd.tree().root()) > before);
    assert!(tooltip.is_showing());
  }

  #[test]
  #[should_panic(expected = "A Tooltip instance can only be bound to one host.")]
  fn reusing_the_same_tooltip_for_multiple_hosts_panics() {
    reset_test_env!();

    let tooltip = Tooltip::new("tip");
    let first_tooltip = tooltip.clone();
    let second_tooltip = tooltip.clone();
    let wnd = TestWindow::new_with_size(
      fn_widget! {
        let first_tooltip = first_tooltip.clone();
        let second_tooltip = second_tooltip.clone();
        @MockMulti {
          @MockBox {
            size: Size::new(40., 20.),
            tooltip: first_tooltip,
          }
          @MockBox {
            size: Size::new(40., 20.),
            tooltip: second_tooltip,
          }
        }
      },
      Size::new(120., 120.),
    );

    wnd.draw_frame();
  }
}
