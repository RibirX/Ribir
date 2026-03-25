use std::cell::{Cell, RefCell};

use ribir_algo::Rc;

use crate::{declare::ValueKind, prelude::*};

class_names! {
  #[doc = "Class name for tooltip content"]
  TOOLTIP,
}

const TOOLTIP_SHOW_DELAY: Duration = Duration::from_millis(500);
const TOOLTIP_HIDE_DELAY: Duration = Duration::from_millis(150);

/// Tooltip content that can be rendered either as the default text bubble or
/// as a custom widget tree such as a card.
pub enum TooltipContent {
  Text(TextValue),
  Widget(Widget<'static>),
}

impl TooltipContent {
  pub fn into_widget(self) -> Widget<'static> {
    match self {
      TooltipContent::Text(text) => text! { text, class: TOOLTIP }.into_widget(),
      TooltipContent::Widget(widget) => widget,
    }
  }
}

/// Trait for customizing tooltip behavior.
///
/// Core owns the orchestration lifecycle. Implementors override individual
/// hooks to customize specific aspects without replacing the entire flow.
pub trait CustomTooltip: 'static {
  /// Builds the tooltip bubble widget.
  ///
  /// Override to customize the look, feel, or placement of the tooltip.
  fn build_bubble(&self, host_track: TrackId, content: TooltipContent) -> Widget<'static> {
    let mut bubble = Follow::declarer();
    bubble
      .with_target(host_track)
      .with_x_align(AnchorX::center())
      .with_y_align(AnchorY::above());

    bubble
      .finish()
      .with_child(content.into_widget())
      .into_widget()
  }

  /// Spawns a background task that listens to visibility changes and
  /// mounts/unmounts the bubble accordingly.
  ///
  /// This does NOT immediately mount the bubble. Instead, it sets up a
  /// subscription that watches the `visible` state and mounts the bubble when
  /// `visible` becomes true, then unmounts it when `visible` becomes false.
  ///
  /// Returns a closure that stops the background task and performs cleanup.
  fn spawn_bubble(
    &self, bubble: Widget<'static>, visible: Stateful<bool>, host_track: TrackId,
  ) -> Box<dyn FnOnce()> {
    let reusable = Reusable::new(bubble);
    let mounted: Rc<RefCell<Option<MountHandle>>> = Rc::default();
    let sub = watch!(*$read(visible))
      .distinct_until_changed()
      .subscribe({
        let mounted = mounted.clone();
        let wnd = BuildCtx::get().window();
        move |visible| {
          let mut mounted = mounted.borrow_mut();
          if visible && mounted.is_none() && host_track.get().is_some() {
            *mounted = Some(wnd.mount(reusable.get_widget()));
          } else if !visible && let Some(handle) = mounted.take() {
            handle.close();
          }
        }
      });

    Box::new(move || {
      if let Some(handle) = mounted.borrow_mut().take() {
        handle.close();
      }
      sub.unsubscribe();
    })
  }

  /// Whether core should set up the default hover/focus delay trigger.
  ///
  /// Return `false` to disable automatic triggering and rely solely on
  /// `Tooltip::show()` / `Tooltip::hide()` for manual control.
  fn auto_trigger(&self) -> bool { true }
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
pub struct Tooltip {
  content: Rc<RefCell<Option<TooltipContent>>>,
  visible: Stateful<bool>,
  bound: Rc<Cell<bool>>,
}

impl Default for Tooltip {
  fn default() -> Self {
    Self::from_content(TooltipContent::Text(PipeValue::Value(CowArc::default())))
  }
}

impl Clone for Tooltip {
  fn clone(&self) -> Self {
    Self {
      content: self.content.clone(),
      visible: self.visible.clone_writer(),
      bound: self.bound.clone(),
    }
  }
}

impl Declare for Tooltip {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl Tooltip {
  pub fn new<K: ?Sized>(text: impl RInto<TextValue, K>) -> Self {
    Self::from_content(TooltipContent::Text(text.r_into()))
  }

  pub fn from_widget<K>(widget: impl IntoWidget<'static, K>) -> Self {
    Self::from_content(TooltipContent::Widget(widget.into_widget()))
  }

  fn from_content(content: TooltipContent) -> Self {
    Self {
      content: Rc::new(RefCell::new(Some(content))),
      visible: Stateful::new(false),
      bound: Rc::new(Cell::new(false)),
    }
  }

  pub fn show(&self) { *self.visible.write() = true; }

  pub fn hide(&self) { *self.visible.write() = false; }

  pub fn is_visible(&self) -> bool { *self.visible.read() }
}

impl<T, K: ?Sized> RFrom<T, ValueKind<K>> for Tooltip
where
  TextValue: RFrom<T, K>,
{
  fn r_from(value: T) -> Self { Self::new(TextValue::r_from(value)) }
}

impl<'c> ComposeChild<'c> for Tooltip {
  type Child = Widget<'c>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    let f = move || {
      let tooltip = match this.try_into_value() {
        Ok(t) => t,
        Err(_) => panic!("Tooltip should be a stateless widget"),
      };

      assert!(!tooltip.bound.get(), "A Tooltip instance can only be bound to one host.");
      tooltip.bound.set(true);

      let content = tooltip
        .content
        .borrow_mut()
        .take()
        .expect("Tooltip content already taken");

      let visible = tooltip.visible.clone_writer();

      struct FallbackTooltip;
      impl CustomTooltip for FallbackTooltip {}

      let provider = Provider::of::<Box<dyn CustomTooltip>>(BuildCtx::get());
      let provider: &dyn CustomTooltip = match provider {
        Some(ref boxed) => boxed.as_ref(),
        None => &FallbackTooltip,
      };

      // 1. Collect host states
      let mut host = FatObj::new(child);
      let host_track = host.track_id();

      // 2. Build and mount the bubble
      let bubble = provider.build_bubble(host_track.clone(), content);
      let mut bubble_obj = FatObj::new(bubble);

      // 3. Setup trigger
      let trigger_sub = provider.auto_trigger().then(|| {
        watch!(
          *$read(host.is_hovered())
          || *$read(host.is_focused())
          || *$read(bubble_obj.is_hovered())
        )
        .delay_bool(TOOLTIP_SHOW_DELAY, TOOLTIP_HIDE_DELAY)
        .subscribe({
          let visible = visible.clone_writer();
          move |now_active| {
            if *visible.read() != now_active {
              *visible.write() = now_active;
            }
          }
        })
      });

      let unmount =
        provider.spawn_bubble(bubble_obj.into_widget(), visible.clone_writer(), host_track);
      // 4. Cleanup on host disposal
      host.on_disposed(move |_| {
        if let Some(sub) = trigger_sub {
          sub.unsubscribe();
        }
        unmount();
        *visible.write() = false;
      });
      host.into_widget()
    };

    FnWidget::new(f).into_widget()
  }
}

#[cfg(test)]
mod tests {
  use std::{cell::Cell, rc::Rc};

  use crate::{prelude::*, reset_test_env, test_helper::*};

  const HOST_POINT: Point = Point::new(10., 10.);
  const OUTSIDE_POINT: Point = Point::new(100., 70.);

  fn wait_for_tooltip_show_delay() {
    AppCtx::run_until(AppCtx::timer(super::TOOLTIP_SHOW_DELAY + Duration::from_millis(20)));
    AppCtx::run_until_stalled();
  }

  fn wait_for_tooltip_hide_delay() {
    AppCtx::run_until(AppCtx::timer(super::TOOLTIP_HIDE_DELAY + Duration::from_millis(20)));
    AppCtx::run_until_stalled();
  }

  fn tree_count(wnd: &TestWindow) -> usize { wnd.tree().count(wnd.tree().root()) }

  fn move_cursor_and_draw(wnd: &TestWindow, point: Point) {
    wnd.process_cursor_move(point);
    wnd.draw_frame();
  }

  fn hover_and_show(wnd: &TestWindow, point: Point) -> usize {
    move_cursor_and_draw(wnd, point);
    wait_for_tooltip_show_delay();
    wnd.draw_frame();
    tree_count(wnd)
  }

  fn focus_host(wnd: &TestWindow, point: Point, after_focus: Point) {
    wnd.process_cursor_move(point);
    wnd.process_mouse_press(Box::new(DummyDeviceId), MouseButtons::PRIMARY);
    wnd.process_mouse_release(Box::new(DummyDeviceId), MouseButtons::PRIMARY);
    wnd.process_cursor_move(after_focus);
    wnd.draw_frame();
  }

  fn fallback_bubble_id(wnd: &TestWindow) -> WidgetId {
    let overlay_root = wnd
      .children(wnd.root())
      .last()
      .expect("fallback tooltip should mount into root");
    wnd
      .children(overlay_root)
      .last()
      .unwrap_or(overlay_root)
  }

  fn fallback_bubble_rect(wnd: &TestWindow) -> (Point, Size) {
    let bubble = fallback_bubble_id(wnd);
    let pos = wnd
      .widget_pos(bubble)
      .expect("fallback tooltip should have layout position");
    let size = wnd
      .widget_size(bubble)
      .expect("fallback tooltip should have layout size");
    (pos, size)
  }

  fn fallback_bubble_center_global(wnd: &TestWindow) -> Point {
    let bubble = fallback_bubble_id(wnd);
    let global = wnd.map_to_global(Point::zero(), bubble);
    let size = wnd
      .widget_size(bubble)
      .expect("fallback tooltip should have layout size");
    Point::new(global.x + size.width / 2., global.y + size.height / 2.)
  }

  #[test]
  fn tooltip_manual_control_mounts_bound_tooltip() {
    reset_test_env!();

    let tooltip = Tooltip::new("tip");
    let tooltip_in_widget = tooltip.clone();
    let wnd = TestWindow::new_with_size(
      fn_widget! {
        let tooltip = tooltip_in_widget.clone();
        @MockBox {
          size: Size::new(40., 20.),
          tooltip,
        }
      },
      Size::new(120., 80.),
    );
    wnd.draw_frame();

    let before = tree_count(&wnd);
    wait_for_tooltip_hide_delay();
    tooltip.show();
    wnd.draw_frame();
    assert!(tooltip.is_visible());
    let shown = tree_count(&wnd);
    assert!(shown > before, "manual show should mount tooltip content");

    tooltip.hide();
    wnd.draw_frame();
    assert_eq!(tree_count(&wnd), before);
    assert!(!tooltip.is_visible());
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

    let before = tree_count(&wnd);
    let shown = hover_and_show(&wnd, HOST_POINT);

    assert!(shown > before, "tooltip should mount extra content when hovered");

    move_cursor_and_draw(&wnd, OUTSIDE_POINT);
    assert_eq!(tree_count(&wnd), shown);
    wait_for_tooltip_hide_delay();
    wnd.draw_frame();
    assert_eq!(tree_count(&wnd), before);
  }

  #[test]
  fn fallback_tooltip_does_not_mount_if_hover_ends_before_delay() {
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

    let before = tree_count(&wnd);
    move_cursor_and_draw(&wnd, HOST_POINT);
    move_cursor_and_draw(&wnd, OUTSIDE_POINT);
    wait_for_tooltip_show_delay();
    wnd.draw_frame();

    assert_eq!(tree_count(&wnd), before);
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

    let before = tree_count(&wnd);
    let shown = hover_and_show(&wnd, HOST_POINT);
    assert!(shown > before, "tooltip should mount while hovered even when text is empty");

    *tooltip_text_writer.write() = "tip".into();
    wnd.draw_frame();
    assert_eq!(tree_count(&wnd), shown);

    *tooltip_text_writer.write() = String::new();
    wnd.draw_frame();
    assert_eq!(tree_count(&wnd), shown);
  }

  #[test]
  fn fallback_tooltip_mounts_on_focus() {
    reset_test_env!();

    let wnd = TestWindow::new_with_size(
      fn_widget! {
        @MockBox {
          size: Size::new(40., 20.),
          tab_index: 0_i16,
          tooltip: "tip",
        }
      },
      Size::new(120., 80.),
    );
    wnd.draw_frame();

    let before = tree_count(&wnd);
    focus_host(&wnd, HOST_POINT, OUTSIDE_POINT);
    wait_for_tooltip_show_delay();
    wnd.draw_frame();
    let after_focus = tree_count(&wnd);
    assert!(after_focus > before, "tooltip should mount when focused");

    wnd.process_cursor_move(OUTSIDE_POINT);
    wnd
      .focus_mgr
      .borrow_mut()
      .blur(FocusReason::Other);
    wnd.draw_frame();
    assert_eq!(tree_count(&wnd), after_focus);
    wait_for_tooltip_hide_delay();
    wnd.draw_frame();
    assert_eq!(tree_count(&wnd), before);
  }

  #[test]
  fn fallback_tooltip_stays_visible_while_hovering_tooltip() {
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

    let before = tree_count(&wnd);
    let shown = hover_and_show(&wnd, HOST_POINT);
    assert!(shown > before);

    wnd.process_cursor_move(fallback_bubble_center_global(&wnd));
    wnd.draw_frame();
    assert_eq!(tree_count(&wnd), shown);

    wnd.process_cursor_move(OUTSIDE_POINT);
    wnd.draw_frame();
    assert_eq!(tree_count(&wnd), shown);
    wait_for_tooltip_hide_delay();
    wnd.draw_frame();
    assert_eq!(tree_count(&wnd), before);
  }

  #[test]
  fn fallback_tooltip_positions_bubble_relative_to_host() {
    reset_test_env!();

    let wnd = TestWindow::new_with_size(
      fn_widget! {
        @MockBox {
          size: Size::new(40., 20.),
          x: 100.,
          y: 80.,
          tooltip: "tip",
        }
      },
      Size::new(240., 200.),
    );
    wnd.draw_frame();

    hover_and_show(&wnd, Point::new(110., 90.));

    let (overlay_pos, overlay_size) = fallback_bubble_rect(&wnd);

    let host_center_x = 100. + 20.;
    let bubble_center_x = overlay_pos.x + overlay_size.width / 2.;
    assert!((bubble_center_x - host_center_x).abs() < 1.0);
    assert!(overlay_pos.y < 80.);
    assert!((overlay_pos.y + overlay_size.height - 80.).abs() < 1.0);
  }

  #[test]
  fn fallback_tooltip_repositions_correctly_on_second_show() {
    reset_test_env!();

    let wnd = TestWindow::new_with_size(
      fn_widget! {
        @MockBox {
          size: Size::new(40., 20.),
          x: 100.,
          y: 80.,
          tooltip: "tip",
        }
      },
      Size::new(240., 200.),
    );
    wnd.draw_frame();
    let before = tree_count(&wnd);

    let (first_pos, first_size) = {
      hover_and_show(&wnd, Point::new(110., 90.));
      fallback_bubble_rect(&wnd)
    };

    move_cursor_and_draw(&wnd, HOST_POINT);
    wait_for_tooltip_hide_delay();
    wnd.draw_frame();
    assert_eq!(tree_count(&wnd), before);
    let (second_pos, second_size) = {
      hover_and_show(&wnd, Point::new(110., 90.));
      fallback_bubble_rect(&wnd)
    };

    assert_eq!(first_pos, second_pos);
    assert_eq!(first_size, second_size);
  }

  #[test]
  fn fallback_tooltip_reenter_during_hide_delay_stays_visible() {
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

    let shown = hover_and_show(&wnd, HOST_POINT);

    move_cursor_and_draw(&wnd, OUTSIDE_POINT);
    assert_eq!(tree_count(&wnd), shown);

    move_cursor_and_draw(&wnd, HOST_POINT);
    wait_for_tooltip_hide_delay();
    wnd.draw_frame();

    assert_eq!(tree_count(&wnd), shown);
  }

  #[test]
  fn same_node_provider_can_override_tooltip() {
    reset_test_env!();

    let hit = Rc::new(Cell::new(0usize));
    let hit_in_widget = hit.clone();

    let wnd = TestWindow::new_with_size(
      fn_widget! {
        struct HitTooltip { hit: Rc<Cell<usize>> }
        impl CustomTooltip for HitTooltip {
          fn build_bubble(&self, _: TrackId, _: TooltipContent) -> Widget<'static> {
            self.hit.set(self.hit.get() + 1);
            fn_widget! { @MockBox { size: Size::zero() } }.into_widget()
          }
          fn auto_trigger(&self) -> bool { false }
        }
        let hit = hit_in_widget.clone();
        let custom = Box::new(HitTooltip { hit }) as Box<dyn CustomTooltip>;
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
  fn fallback_tooltip_supports_widget_content() {
    reset_test_env!();

    let tooltip = Tooltip::from_widget(fn_widget! {
      @MockBox { size: Size::new(60., 24.) }
    });
    let tooltip_in_widget = tooltip.clone();
    let wnd = TestWindow::new_with_size(
      fn_widget! {
        let tooltip = tooltip_in_widget.clone();
        @MockBox {
          size: Size::new(40., 20.),
          tooltip,
        }
      },
      Size::new(120., 80.),
    );
    wnd.draw_frame();

    hover_and_show(&wnd, HOST_POINT);

    let (_, overlay_size) = fallback_bubble_rect(&wnd);

    assert_eq!(overlay_size, Size::new(60., 24.));
  }

  #[test]
  fn fallback_tooltip_manual_visibility_yields_to_hover_updates() {
    reset_test_env!();

    let tooltip = Tooltip::new("tip");
    let tooltip_in_widget = tooltip.clone();
    let wnd = TestWindow::new_with_size(
      fn_widget! {
        let tooltip = tooltip_in_widget.clone();
        @MockBox {
          size: Size::new(40., 20.),
          tooltip,
        }
      },
      Size::new(120., 80.),
    );
    wnd.draw_frame();

    let before = tree_count(&wnd);
    wnd.process_cursor_move(HOST_POINT);
    tooltip.show();
    wnd.draw_frame();
    assert!(tooltip.is_visible());
    let shown = tree_count(&wnd);

    assert!(shown > before, "manual show should mount tooltip content");

    move_cursor_and_draw(&wnd, HOST_POINT);
    assert_eq!(tree_count(&wnd), shown);

    move_cursor_and_draw(&wnd, OUTSIDE_POINT);
    assert_eq!(tree_count(&wnd), shown);
    wait_for_tooltip_hide_delay();
    wnd.draw_frame();
    assert!(!tooltip.is_visible());
    assert_eq!(tree_count(&wnd), before);
  }

  #[test]
  fn custom_tooltip_can_disable_default_trigger() {
    reset_test_env!();

    let tooltip = Tooltip::new("tip");
    let tooltip_in_widget = tooltip.clone();
    let wnd = TestWindow::new_with_size(
      fn_widget! {
        struct ManualOnlyTooltip;
        impl CustomTooltip for ManualOnlyTooltip {
          fn auto_trigger(&self) -> bool { false }
        }
        let tooltip = tooltip_in_widget.clone();
        let custom = Box::new(ManualOnlyTooltip) as Box<dyn CustomTooltip>;
        @MockBox {
          size: Size::new(40., 20.),
          providers: [Provider::new(custom)],
          tooltip,
        }
      },
      Size::new(120., 80.),
    );
    wnd.draw_frame();

    let before = tree_count(&wnd);
    move_cursor_and_draw(&wnd, HOST_POINT);
    assert!(!tooltip.is_visible());
    assert_eq!(tree_count(&wnd), before);

    tooltip.show();
    wnd.draw_frame();
    assert!(tooltip.is_visible());
    assert!(tree_count(&wnd) > before);
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
