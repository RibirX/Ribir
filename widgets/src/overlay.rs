use std::cell::RefCell;

use crate::core::prelude::{Rc, *};

/// Overlay let independent the widget "float" visual elements on top of
/// other widgets by inserting them into the root stack of the widget stack.
///
/// ### Example
///
/// ```rust no_run
/// use ribir::prelude::*;
///
/// let w = fn_widget! {
///   let overlay = Overlay::new(
///     fn_widget! {
///       @Text {
///         on_tap: move |e| Overlay::of(&**e).unwrap().close(),
///         x: AnchorX::center(),
///         y: AnchorY::center(),
///         text: "Click me to close overlay!"
///       }
///     },
///     OverlayStyle { auto_close_policy: AutoClosePolicy::TAP_OUTSIDE, mask: None });
///   @FilledButton{
///     on_tap: move |e| overlay.show(e.window()),
///     @{ "Click me to show overlay" }
///   }
/// };
/// App::run(w);
/// ```
#[derive(Clone)]
pub struct Overlay(Rc<RefCell<InnerOverlay>>);

bitflags! {
  #[derive(Clone, Copy)]
  pub struct AutoClosePolicy: u8 {
    const NOT_AUTO_CLOSE = 0b0000;
    const ESC = 0b0001;
    const TAP_OUTSIDE = 0b0010;
  }
}

#[derive(Clone)]
pub struct OverlayStyle {
  /// the auto close policy of the overlay.
  pub auto_close_policy: AutoClosePolicy,
  /// the mask brush for the background of the overly.
  pub mask: Option<Brush>,
}

struct InnerOverlay {
  gen_widget: GenWidget,
  auto_close_policy: AutoClosePolicy,
  mask: Option<Brush>,
  showing: Option<MountHandle>,
}

#[derive(Clone)]
struct CurrentOverlay(Overlay);

impl Overlay {
  /// Create overlay from a function widget that may call many times.
  pub fn new<K: ?Sized>(gen_widget: impl RInto<GenWidget, K>, style: OverlayStyle) -> Self {
    let gen_widget = gen_widget.r_into();
    let OverlayStyle { auto_close_policy, mask } = style;
    Self(Rc::new(RefCell::new(InnerOverlay { gen_widget, auto_close_policy, mask, showing: None })))
  }

  /// Return the overlay that the `ctx` belongs to if it is within an overlay.
  pub fn of<C: AsRef<ProviderCtx> + WidgetCtx>(ctx: &C) -> Option<Self> {
    Provider::of::<CurrentOverlay>(ctx).map(|overlay| overlay.0.clone())
  }

  /// Get the auto close policy of the overlay.
  pub fn auto_close_policy(&self) -> AutoClosePolicy { self.0.borrow().auto_close_policy }

  /// Get the mask of the the background of the overlay used.
  pub fn mask(&self) -> Option<Brush> { self.0.borrow().mask.clone() }

  /// Show the overlay.
  pub fn show(&self, wnd: Rc<Window>) {
    if self.is_showing() {
      return;
    }
    let gen_widget = self.0.borrow().gen_widget.clone();
    self.inner_show(gen_widget, wnd);
  }

  /// ### Example
  ///
  /// Overlay widget which auto align horizontal position to the src button even
  /// when window's size changed
  ///
  /// ```rust no_run
  /// use ribir::prelude::*;
  /// let w = fn_widget! {
  ///   let mut button = @FilledButton {};
  ///   let overlay = Overlay::new(
  ///     text! { text: "overlay" },
  ///     OverlayStyle { auto_close_policy: AutoClosePolicy::TAP_OUTSIDE, mask: None }
  ///   );
  ///   @(button) {
  ///     x: AnchorX::center(),
  ///     y: AnchorY::center(),
  ///     on_tap: move |e| {
  ///       let wnd = e.window();
  ///       overlay.show_map(move |w| fn_widget! {
  ///         @Follow {
  ///           target: $clone(button.track_id()),
  ///           x_align: AnchorX::center(),
  ///           y_align: AnchorY::under(),
  ///           @ { w }
  ///         }
  ///       }.into_widget(), wnd);
  ///     },
  ///     @{ "Click to show overlay" }
  ///   }
  /// };
  /// App::run(w);
  /// ```
  pub fn show_map<F>(&self, mut f: F, wnd: Rc<Window>)
  where
    F: FnMut(Widget<'static>) -> Widget<'static> + 'static,
  {
    if self.is_showing() {
      return;
    }
    let gen_widget = self.0.borrow().gen_widget.clone();
    let gen_widget = move || f(gen_widget.gen_widget());
    self.inner_show(gen_widget.r_into(), wnd);
  }

  /// Show the widget at the give position.
  /// if the overlay is showing, nothing will happen.
  pub fn show_at(&self, pos: Point, wnd: Rc<Window>) {
    if self.is_showing() {
      return;
    }
    self.show_map(
      move |w| {
        let mut obj = FatObj::new(w);
        obj.with_x(pos.x).with_y(pos.y);
        obj.into_widget()
      },
      wnd,
    );
  }

  /// return whether the overlay is showing.
  pub fn is_showing(&self) -> bool { self.0.borrow().showing.is_some() }

  /// Close the overlay; all widgets within the overlay will be removed.
  pub fn close(&self) {
    if let Some(handle) = self.0.borrow_mut().showing.take() {
      AppCtx::spawn_local(async move {
        handle.close();
      });
    }
  }

  fn inner_show(&self, content: GenWidget, wnd: Rc<Window>) {
    let background = self.mask();
    let close_policy = self.auto_close_policy();
    let overlay = CurrentOverlay(self.clone());
    let gen_widget = fn_widget! {
      let w = content.gen_widget();
      let mut w = if background.is_some() || close_policy.contains(AutoClosePolicy::TAP_OUTSIDE) {
        let mut container = @Container {};
        if let Some(background) = background.clone() {
          container.with_background(background);
        }
        if close_policy.contains(AutoClosePolicy::TAP_OUTSIDE) {
          container.on_tap(move |e| {
            if e.target() == e.current_target()
              && let Some(overlay) = Overlay::of(&**e)
            {
                overlay.close();
            }
          });
        }
        container.map(|c| c.with_child(w).into_widget())
      } else {
        FatObj::new(w)
      };
      if close_policy.contains(AutoClosePolicy::ESC) {
        w.on_key_down(move |e| {
          if *e.key() == VirtualKey::Named(NamedKey::Escape) &&
            let Some(overlay) = Overlay::of(&**e)
          {
            overlay.close();
          }
        });
      }
      @Providers {
        providers: [Provider::new(overlay.clone())],
        @ { w }
      }
    };

    let this = self.clone();
    AppCtx::spawn_local(async move {
      let generator: GenWidget = gen_widget.r_into();
      this.0.borrow_mut().showing = Some(wnd.mount_gen(generator));
    });
  }
}

#[cfg(test)]
mod tests {
  use std::{cell::RefCell, rc::Rc};

  use ribir_core::{
    prelude::{Point, Size},
    reset_test_env,
    test_helper::*,
  };

  use super::*;

  #[test]
  fn overlay() {
    reset_test_env!();
    let size = Size::zero();
    let widget = fn_widget! {
      @MockBox {
        size,
        @MockBox { size }
      }
    };

    let wnd = TestWindow::from_widget(widget);
    let w_log = Rc::new(RefCell::new(vec![]));
    let r_log = w_log.clone();
    let overlay = Overlay::new(
      fn_widget! {
        @MockBox {
          size,
          on_mounted: {
            let w_log = w_log.clone();
            move |_| { w_log.borrow_mut().push("mounted");}
          },
          on_disposed: {
            let w_log = w_log.clone();
            move |_| { w_log.borrow_mut().push("disposed");}
          }
        }
      },
      OverlayStyle { auto_close_policy: AutoClosePolicy::NOT_AUTO_CLOSE, mask: None },
    );
    wnd.draw_frame();

    let root = wnd.root();
    assert_eq!(wnd.count(root), 3);

    overlay.show_at(Point::new(50., 30.), wnd.0.clone());
    wnd.draw_frame();
    assert_eq!(*r_log.borrow(), &["mounted"]);

    let id = wnd.widget_id_by_path(&[1]);
    assert_eq!(wnd.widget_pos(id).unwrap(), Point::new(50., 30.));

    overlay.close();
    wnd.draw_frame();
    assert_eq!(*r_log.borrow(), &["mounted", "disposed"]);
    assert_eq!(wnd.count(root), 3);
  }

  #[test]
  fn overlay_of_still_resolves_after_mount_gen_migration() {
    reset_test_env!();

    let mounted_id = Stateful::new(None::<WidgetId>);
    let found_overlay = Stateful::new(false);
    let mounted_id_reader = mounted_id.clone_reader();
    let found_overlay_reader = found_overlay.clone_reader();
    let wnd = TestWindow::from_widget(fn_widget! { @MockBox { size: Size::zero() } });
    let overlay = Overlay::new(
      fn_widget! {
        @MockBox {
          size: Size::zero(),
          on_mounted: move |e| *$write(mounted_id) = Some(e.current_target()),
          on_event: move |e| {
            if matches!(e, Event::CustomEvent(_)) {
              *$write(found_overlay) = Overlay::of(&**e).is_some();
            }
          }
        }
      },
      OverlayStyle { auto_close_policy: AutoClosePolicy::NOT_AUTO_CLOSE, mask: None },
    );

    overlay.show(wnd.0.clone());
    wnd.draw_frame();
    let overlay_root = mounted_id_reader
      .read()
      .expect("overlay root should mount");

    wnd.bubble_custom_event(overlay_root, ());
    wnd.draw_frame();
    assert!(*found_overlay_reader.read());
  }

  #[test]
  fn overlay_rebuild_remounts_through_window_rebuild_mounts() {
    reset_test_env!();

    let mounted_count = Stateful::new(0);
    let mounted_count_reader = mounted_count.clone_reader();
    let wnd = TestWindow::from_widget(fn_widget! { @MockBox { size: Size::zero() } });
    let overlay = Overlay::new(
      fn_widget! {
        @MockBox {
          size: Size::zero(),
          on_mounted: move |_| *$write(mounted_count) += 1,
        }
      },
      OverlayStyle { auto_close_policy: AutoClosePolicy::NOT_AUTO_CLOSE, mask: None },
    );

    overlay.show(wnd.0.clone());
    wnd.draw_frame();
    assert_eq!(*mounted_count_reader.read(), 1);

    {
      wnd.rebuild_mounts();
    }

    wnd.draw_frame();
    assert_eq!(*mounted_count_reader.read(), 2);
    assert!(overlay.is_showing());
  }
}
