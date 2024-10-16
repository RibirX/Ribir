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
///   let overlay = Overlay::new(fn_widget! {
///     @Text {
///       on_tap: move |e| Overlay::of(&**e).unwrap().close(),
///       h_align: HAlign::Center,
///       v_align: VAlign::Center,
///       text: "Click me to close overlay!"
///     }
///   });
///   @FilledButton{
///     on_tap: move |e| overlay.show(e.window()),
///     @{ Label::new("Click me to show overlay") }
///   }
/// };
/// App::run(w);
/// ```
#[derive(Clone)]
pub struct Overlay(Sc<RefCell<InnerOverlay>>);

bitflags! {
  #[derive(Clone, Copy)]
  pub struct AutoClosePolicy: u8 {
    const NONE = 0b0000;
    const ESC = 0b0001;
    const TAP_OUTSIDE = 0b0010;
  }
}

struct InnerOverlay {
  gen: GenWidget,
  auto_close_policy: AutoClosePolicy,
  mask: Option<Brush>,
  showing: Option<ShowingInfo>,
}

struct ShowingInfo {
  id: WidgetId,
  wnd_id: WindowId,
  generator: GenWidget,
}

impl Overlay {
  /// Create overlay from a function widget that may call many times.
  pub fn new(gen: impl Into<GenWidget>) -> Self {
    let gen = gen.into();
    Self(Sc::new(RefCell::new(InnerOverlay {
      gen,
      auto_close_policy: AutoClosePolicy::ESC | AutoClosePolicy::TAP_OUTSIDE,
      mask: None,
      showing: None,
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

  /// Set the auto close policy of the overlay.
  pub fn set_auto_close_policy(&self, policy: AutoClosePolicy) {
    self.0.borrow_mut().auto_close_policy = policy
  }

  /// Get the auto close policy of the overlay.
  pub fn auto_close_policy(&self) -> AutoClosePolicy { self.0.borrow().auto_close_policy }

  /// Set the mask for the background of the overlay being used.
  pub fn set_mask(&self, mask: Brush) { self.0.borrow_mut().mask = Some(mask); }

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
  ///     fn_widget! { @Text { text: "overlay" } }
  ///   );
  ///   let button = @FilledButton{};
  ///   let wid = button.lazy_host_id();
  ///   @$button {
  ///     h_align: HAlign::Center,
  ///     v_align: VAlign::Center,
  ///     on_tap: move |e| {
  ///       let wid = wid.clone();
  ///       overlay.show_map(move |w| {
  ///         let wid = wid.clone();
  ///         fn_widget! {
  ///           let mut w = FatObj::new(w);
  ///           w.left_align_to(&wid, 0., ctx!());
  ///           w
  ///         }.into_widget()
  ///        },
  ///        e.window()
  ///       );
  ///     },
  ///     @{ Label::new("Click to show overlay") }
  ///   }
  /// };
  /// App::run(w);
  /// ```
  pub fn show_map<F>(&self, mut f: F, wnd: Sc<Window>)
  where
    F: FnMut(Widget) -> Widget + 'static,
  {
    if self.is_showing() {
      return;
    }
    let gen = self.0.borrow().gen.clone();
    let gen = move |_: &mut BuildCtx| f(gen.gen_widget());
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
    if let Some(showing) = showing {
      let ShowingInfo { id, wnd_id, .. } = showing;
      if let Some(wnd) = AppCtx::get_window(wnd_id) {
        let ctx = BuildCtx::create(wnd.tree().root(), wnd.tree);
        let showing_overlays = Provider::of::<ShowingOverlays>(&*ctx).unwrap();
        showing_overlays.remove(self);

        let tree = wnd.tree_mut();
        let root = tree.root();
        id.dispose_subtree(tree);
        tree.mark_dirty(root);
      }
    }
  }

  fn inner_show(&self, content: GenWidget, wnd: Sc<Window>) {
    let background = self.mask();
    let gen = fn_widget! {
      @Container {
        size: Size::new(f32::INFINITY, f32::INFINITY),
        background: background.clone(),
        on_tap: move |e| {
          if e.target() == e.current_target() {
            if let Some(overlay) = Overlay::of(&**e)
              .filter(|o| o.auto_close_policy().contains(AutoClosePolicy::TAP_OUTSIDE))
            {
              overlay.close();
            }
          }
        },
        on_key_down: move |e| {
          if *e.key() == VirtualKey::Named(NamedKey::Escape) {
            if let Some(overlay) = Overlay::of(&**e)
              .filter(|o| o.auto_close_policy().contains(AutoClosePolicy::ESC))
            {
              overlay.close();
            }
          }
        },
        @ { content.gen_widget() }
      }
    };

    let mut ctx = BuildCtx::create(wnd.tree().root(), wnd.tree);
    let id = gen(&mut ctx).build(&mut ctx);
    self.0.borrow_mut().showing = Some(ShowingInfo { id, generator: gen.into(), wnd_id: wnd.id() });

    let showing_overlays = Provider::of::<ShowingOverlays>(&*ctx).unwrap();
    showing_overlays.add(self.clone());

    let tree = wnd.tree_mut();
    tree.root().append(id, tree);
    id.on_mounted_subtree(tree);
    tree.mark_dirty(id);
  }

  fn showing_root(&self) -> Option<WidgetId> { self.0.borrow().showing.as_ref().map(|s| s.id) }
}

pub(crate) struct ShowingOverlays(RefCell<Vec<Overlay>>);

impl ShowingOverlays {
  pub(crate) fn rebuild(&self, ctx: &mut BuildCtx) {
    for o in self.0.borrow().iter() {
      let mut o = o.0.borrow_mut();
      let ShowingInfo { id, generator, .. } = o.showing.as_mut().unwrap();

      id.dispose_subtree(ctx.tree_mut());
      *id = generator.gen_widget().build(ctx);
      let tree = ctx.tree_mut();
      tree.root().append(*id, tree);
      id.on_mounted_subtree(tree);
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
        .map_or(false, |w| ctx.successor_of(w))
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

  use crate::{prelude::*, reset_test_env, test_helper::*};

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
    let overlay = Overlay::new(fn_widget! {
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
    });
    wnd.draw_frame();

    let root = wnd.tree().root();
    assert_eq!(wnd.tree().count(root), 3);

    overlay.show_at(Point::new(50., 30.), wnd.0.clone());
    wnd.draw_frame();
    assert_eq!(*r_log.borrow(), &["mounted"]);

    assert_eq!(wnd.layout_info_by_path(&[1, 0]).unwrap().pos, Point::new(50., 30.));

    overlay.close();
    wnd.draw_frame();
    assert_eq!(*r_log.borrow(), &["mounted", "disposed"]);
    assert_eq!(wnd.tree().count(root), 3);
  }
}
