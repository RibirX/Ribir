use std::cell::RefCell;

use ribir_algo::Sc;

use crate::{prelude::*, window::WindowId};

/// Overlay let independent the widget "float" visual elements on top of
/// other widgets by inserting them into the root stack of the widget stack.
///
/// ### Example
///
/// ```no_run
/// use ribir::prelude::*;
///
/// let w = fn_widget! {
///   let overlay = Overlay::new(
///     fn_widget! {
///       @Text {
///         on_tap: move |e| Overlay::of(&**e).unwrap().close(),
///         h_align: HAlign::Center,
///         v_align: VAlign::Center,
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
pub struct Overlay(Sc<RefCell<InnerOverlay>>);

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
  gen: GenWidget,
  auto_close_policy: AutoClosePolicy,
  mask: Option<Brush>,
  showing: Option<ShowingInfo>,
  track_id: RefCell<Option<TrackId>>,
}

struct ShowingInfo {
  wnd_id: WindowId,
  generator: GenWidget,
}

impl Overlay {
  /// Create overlay from a function widget that may call many times.
  pub fn new(gen: impl Into<GenWidget>, style: OverlayStyle) -> Self {
    let gen = gen.into();
    let OverlayStyle { auto_close_policy, mask } = style;
    Self(Sc::new(RefCell::new(InnerOverlay {
      gen,
      auto_close_policy,
      mask,
      showing: None,
      track_id: RefCell::new(None),
    })))
  }

  /// Return the overlay that the `ctx` belongs to if it is within an overlay.
  pub fn of(ctx: &impl WidgetCtx) -> Option<Self> {
    let wnd = ctx.window();
    let tree = wnd.tree();
    let overlays = tree
      .root()
      .query_ref::<ShowingOverlays>(tree)
      .unwrap();

    overlays.showing_of(ctx)
  }

  /// Get the auto close policy of the overlay.
  pub fn auto_close_policy(&self) -> AutoClosePolicy { self.0.borrow().auto_close_policy }

  /// Get the mask of the the background of the overlay used.  
  pub fn mask(&self) -> Option<Brush> { self.0.borrow().mask.clone() }

  /// Show the overlay.
  pub fn show(&self, wnd: Sc<Window>) {
    if self.is_showing() {
      return;
    }
    let gen = self.0.borrow().gen.clone();
    self.inner_show(gen, wnd);
  }

  /// User can make transform before the widget show at the top level of all
  /// widget. if the overlay is showing, nothing will happen.
  ///
  /// ### Example
  ///
  /// Overlay widget which auto align horizontal position to the src button even
  /// when window's size changed
  ///
  /// ``` no_run
  /// use ribir::prelude::*;
  /// let w = fn_widget! {
  ///   let overlay = Overlay::new(
  ///     fn_widget! { @Text { text: "overlay" } },
  ///     OverlayStyle { auto_close_policy: AutoClosePolicy::TAP_OUTSIDE, mask: None }
  ///   );
  ///   let mut button = @FilledButton {};
  ///   @$button {
  ///     h_align: HAlign::Center,
  ///     v_align: VAlign::Center,
  ///     on_tap: move |e| {
  ///       let wnd = e.window();
  ///       overlay.show_map(move |w| {
  ///         let mut w = FatObj::new(w);
  ///         @ $w {
  ///           global_anchor_x: GlobalAnchorX::left_align_to($button.track_id(), 0.),
  ///         }.into_widget()
  ///        },
  ///        e.window()
  ///       );
  ///     },
  ///     @{ "Click to show overlay" }
  ///   }
  /// };
  /// App::run(w);
  /// ```
  pub fn show_map<F>(&self, mut f: F, wnd: Sc<Window>)
  where
    F: FnMut(Widget<'static>) -> Widget<'static> + 'static,
  {
    if self.is_showing() {
      return;
    }
    let gen = self.0.borrow().gen.clone();
    let gen = move || f(gen.gen_widget());
    self.inner_show(gen.into(), wnd);
  }

  /// Show the widget at the give position.
  /// if the overlay is showing, nothing will happen.
  pub fn show_at(&self, pos: Point, wnd: Sc<Window>) {
    if self.is_showing() {
      return;
    }
    self.show_map(
      move |w| {
        FatObj::new(w)
          .anchor(Anchor::from_point(pos))
          .into_widget()
      },
      wnd,
    );
  }

  /// return whether the overlay is showing.
  pub fn is_showing(&self) -> bool { self.0.borrow().showing.is_some() }

  /// Close the overlay; all widgets within the overlay will be removed.
  pub fn close(&self) {
    let showing = self.0.borrow_mut().showing.take();
    let track_id = self.0.borrow_mut().track_id.take();
    if let Some(showing) = showing {
      let ShowingInfo { wnd_id, .. } = showing;
      if let Some(wnd) = AppCtx::get_window(wnd_id) {
        let _guard = BuildCtx::init_for(wnd.tree().root(), wnd.tree);
        let showing_overlays = Provider::of::<ShowingOverlays>(BuildCtx::get()).unwrap();
        showing_overlays.remove(self);

        if let Some(wid) = track_id.and_then(|track_id| track_id.get()) {
          AppCtx::once_next_frame(move |_| {
            let tree = wnd.tree_mut();
            let root = tree.root();
            wid.dispose_subtree(tree);
            tree.dirty_marker().mark(root);
          });
        }
      }
    }
  }

  fn inner_show(&self, content: GenWidget, wnd: Sc<Window>) {
    let background = self.mask();
    let close_policy = self.auto_close_policy();
    let inner = self.0.clone();
    let gen = fn_widget! {
      let mut w = content.gen_widget().into_widget();
      if background.is_some() || close_policy.contains(AutoClosePolicy::TAP_OUTSIDE) {
        w = @Container {
          size: Size::new(f32::INFINITY, f32::INFINITY),
          background: background.clone(),
          on_tap: move |e| {
            if e.target() == e.current_target() {
              if let Some(overlay) = Overlay::of(&**e)
              {
                overlay.close();
              }
            }
          },
          @{ w }
        }.into_widget();
      };
      if close_policy.contains(AutoClosePolicy::ESC) {
        let fat_obj = FatObj::new(w);
        w = @ $fat_obj{
          on_key_down: move |e| {
            if *e.key() == VirtualKey::Named(NamedKey::Escape) {
              if let Some(overlay) = Overlay::of(&**e)
              {
                overlay.close();
              }
            }
          }
        }.into_widget();
      }

      let mut w = FatObj::new(w);
      *inner.borrow().track_id.borrow_mut() = Some($w.track_id());
      @ { w }
    };

    let _guard = BuildCtx::init_for(wnd.tree().root(), wnd.tree);

    let wid = BuildCtx::get_mut().build(gen());
    let tree = wnd.tree_mut();
    tree.root().append(wid, tree);
    wid.on_mounted_subtree(tree);
    tree.dirty_marker().mark(wid);

    self.0.borrow_mut().showing = Some(ShowingInfo { generator: gen.into(), wnd_id: wnd.id() });

    let showing_overlays = Provider::of::<ShowingOverlays>(BuildCtx::get()).unwrap();
    showing_overlays.add(self.clone());
  }

  fn showing_root(&self) -> Option<WidgetId> {
    self
      .0
      .borrow()
      .track_id
      .borrow()
      .as_ref()
      .and_then(|s| s.get())
  }
}

pub(crate) struct ShowingOverlays(RefCell<Vec<Overlay>>);

impl ShowingOverlays {
  pub(crate) fn rebuild(&self) {
    for o in self.0.borrow().iter() {
      let o = o.0.borrow();
      let InnerOverlay { showing, track_id, .. } = &*o;
      let tree = BuildCtx::get_mut().tree_mut();
      {
        if let Some(id) = track_id.borrow().as_ref().and_then(|w| w.get()) {
          id.dispose_subtree(tree);
        }
      }

      let ShowingInfo { generator, .. } = showing.as_ref().unwrap();
      let wid = BuildCtx::get_mut().build(generator.gen_widget());
      tree.root().append(wid, tree);

      wid.on_mounted_subtree(tree);
    }
  }

  fn add(&self, overlay: Overlay) {
    assert!(overlay.showing_root().is_some());
    self.0.borrow_mut().push(overlay)
  }

  fn remove(&self, overlay: &Overlay) {
    assert!(overlay.showing_root().is_none());
    self
      .0
      .borrow_mut()
      .retain(|o| !Sc::ptr_eq(&o.0, &overlay.0))
  }

  fn showing_of(&self, ctx: &impl WidgetCtx) -> Option<Overlay> {
    self.0.borrow().iter().find_map(|o| {
      o.showing_root()
        .is_some_and(|wid| ctx.successor_of(wid))
        .then(|| o.clone())
    })
  }
}

impl Default for ShowingOverlays {
  fn default() -> Self { Self(RefCell::new(vec![])) }
}

#[cfg(test)]
mod tests {
  use std::{cell::RefCell, rc::Rc};

  use crate::{
    overlay::{AutoClosePolicy, OverlayStyle},
    prelude::*,
    reset_test_env,
    test_helper::*,
  };

  #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
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

    let mut wnd = TestWindow::new(widget);
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

    let root = wnd.tree().root();
    assert_eq!(wnd.tree().count(root), 3);

    overlay.show_at(Point::new(50., 30.), wnd.0.clone());
    wnd.draw_frame();
    assert_eq!(*r_log.borrow(), &["mounted"]);

    assert_eq!(wnd.layout_info_by_path(&[1]).unwrap().pos, Point::new(50., 30.));

    overlay.close();
    wnd.draw_frame();
    assert_eq!(*r_log.borrow(), &["mounted", "disposed"]);
    assert_eq!(wnd.tree().count(root), 3);
  }
}
