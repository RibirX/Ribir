use std::{cell::RefCell, mem::replace, rc::Rc};

use crate::prelude::*;

#[derive(Clone)]
pub struct OverlayStyle {
  pub close_policy: ClosePolicy,
  pub mask_brush: Option<Brush>,
}

bitflags! {
  #[derive(Clone, Copy)]
  pub struct ClosePolicy: u8 {
    const NONE = 0b0000;
    const ESC = 0b0001;
    const TAP_OUTSIDE = 0b0010;
  }
}

impl CustomStyle for OverlayStyle {
  fn default_style(_: &BuildCtx) -> Self {
    Self {
      close_policy: ClosePolicy::ESC | ClosePolicy::TAP_OUTSIDE,
      mask_brush: Some(Color::from_f32_rgba(0.3, 0.3, 0.3, 0.3).into()),
    }
  }
}

/// A handle to close the overlay
#[derive(Clone)]
pub struct OverlayCloseHandle(OverlayState);
impl OverlayCloseHandle {
  pub fn close(&self) { self.0.close() }
}

type Builder = Box<dyn FnMut(OverlayCloseHandle, &BuildCtx) -> Widget<'static>>;
struct OverlayData {
  builder: Builder,
  style: Option<OverlayStyle>,
  state: OverlayState,
}

#[derive(Clone)]
pub struct Overlay(Rc<RefCell<OverlayData>>);

impl Overlay {
  /// Create overlay from Clone able widget.
  ///
  /// ### Example
  ///  ``` no_run
  ///  use ribir::prelude::*;
  ///  let w = fn_widget! {
  ///  let overlay = Overlay::new(
  ///     fn_widget! {
  ///       @Text {
  ///         h_align: HAlign::Center,
  ///         v_align: VAlign::Center,
  ///         text: "Hello"
  ///       }
  ///     }
  ///   );
  ///   @FilledButton{
  ///     on_tap: move |e| overlay.show(e.window()),
  ///     @{ Label::new("Click to show overlay") }
  ///   }
  ///  };
  ///  App::run(w);
  /// ```
  pub fn new(gen: impl Into<GenWidget>) -> Self {
    let mut gen = gen.into();
    Self::inner_new(Box::new(move |_, ctx| gen.gen_widget(ctx)))
  }

  /// Create overlay from a builder with a close_handle
  ///
  /// ### Example
  /// popup a widget of a button which will close when clicked.
  /// ``` no_run
  /// use ribir::prelude::*;
  /// let w = fn_widget! {
  ///   let overlay = Overlay::new_with_handle(
  ///     move |ctrl: OverlayCloseHandle| {
  ///       let ctrl = ctrl.clone();
  ///       fn_widget! {
  ///         @FilledButton {
  ///           h_align: HAlign::Center,
  ///           v_align: VAlign::Center,
  ///           on_tap: move |_| ctrl.close(),
  ///           @{ Label::new("Click to close") }
  ///         }
  ///       }.into_widget()
  ///     }
  ///   );
  ///   @FilledButton {
  ///     on_tap: move |e| overlay.show(e.window()),
  ///     @{ Label::new("Click to show overlay") }
  ///   }
  /// };
  ///
  /// App::run(w).with_size(Size::new(200., 200.));
  /// ```
  pub fn new_with_handle(
    mut builder: impl FnMut(OverlayCloseHandle) -> Widget<'static> + 'static,
  ) -> Self {
    Self::inner_new(Box::new(move |ctrl, _| builder(ctrl)))
  }

  /// Overlay will show with the given style, if the overlay have not been set
  /// with style, the default style will be get from the theme.
  pub fn with_style(&self, style: OverlayStyle) { self.0.borrow_mut().style = Some(style); }

  /// the Overlay widget will be show at the top level of all widget.
  /// if the overlay is showing, nothing will happen.
  pub fn show(&self, wnd: Rc<Window>) {
    if self.is_show() {
      return;
    }
    let ctx = BuildCtx::new(None, &wnd.widget_tree);
    let mut inner = self.0.borrow_mut();
    let handle = inner.state.close_handle();
    let w = (inner.builder)(handle, &ctx);
    let style = inner.style.clone();
    inner.state.show(w, style, wnd);
  }

  /// User can make transform before the widget show at the top level of all
  /// widget. if the overlay is showing, nothing will happen.
  ///
  /// ### Example
  /// Overlay widget which auto align horizontal position to the src button even
  /// when window's size changed
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
  ///       overlay.show_map(
  ///         move |w, _| {
  ///           let wid = wid.clone();
  ///           fn_widget! {
  ///             let mut w = @$w {};
  ///             w.left_align_to(&wid, 0., ctx!());
  ///             w
  ///           }
  ///         },
  ///         e.window()
  ///       );
  ///     },
  ///     @{ Label::new("Click to show overlay") }
  ///   }
  /// };
  /// App::run(w);
  /// ```
  pub fn show_map<F, O>(&self, f: F, wnd: Rc<Window>)
  where
    F: FnOnce(Widget<'static>, OverlayCloseHandle) -> O + 'static,
    O: IntoWidget<'static, FN> + 'static,
  {
    if self.is_show() {
      return;
    }

    let ctx = BuildCtx::new(None, &wnd.widget_tree);
    let mut inner = self.0.borrow_mut();
    let close_handle = inner.state.close_handle();
    let overlay = (inner.builder)(close_handle.clone(), &ctx);
    let overlay = f(overlay, close_handle);
    let style = inner.style.clone();
    inner
      .state
      .show(overlay.into_widget(), style, wnd);
  }

  /// Show the widget at the give position.
  /// if the overlay is showing, nothing will happen.
  pub fn show_at(&self, pos: Point, wnd: Rc<Window>) {
    if self.is_show() {
      return;
    }
    self.show_map(
      move |w, _| {
        fn_widget! {
          @$w { anchor: Anchor::from_point(pos) }
        }
      },
      wnd,
    );
  }

  /// return whether the overlay is show.
  pub fn is_show(&self) -> bool { self.0.borrow().state.is_show() }

  /// remove the showing overlay.
  pub fn close(&self) { self.0.borrow().state.close() }

  fn inner_new(builder: Builder) -> Self {
    Self(Rc::new(RefCell::new(OverlayData {
      builder,
      style: None,
      state: OverlayState::default(),
    })))
  }
}

enum OverlayInnerState {
  ToShow(Instant, Rc<Window>),
  Showing(WidgetId, Rc<Window>),
  Hided,
}

#[derive(Clone)]
struct OverlayState(Rc<RefCell<OverlayInnerState>>);
impl Default for OverlayState {
  fn default() -> Self { OverlayState(Rc::new(RefCell::new(OverlayInnerState::Hided))) }
}

impl OverlayState {
  fn close(&self) {
    let state = replace(&mut *self.0.borrow_mut(), OverlayInnerState::Hided);
    if let OverlayInnerState::Showing(wid, wnd) = state {
      let _ = AppCtx::spawn_local(async move {
        let root = wnd.widget_tree.borrow().root();
        wid.dispose_subtree(&mut wnd.widget_tree.borrow_mut());
        wnd.widget_tree.borrow_mut().mark_dirty(root);
      });
    }
  }

  fn is_show(&self) -> bool { !matches!(*self.0.borrow(), OverlayInnerState::Hided) }

  fn show(&self, w: Widget<'static>, style: Option<OverlayStyle>, wnd: Rc<Window>) {
    if self.is_show() {
      return;
    }
    let this = self.clone();
    let instant = Instant::now();
    *this.0.borrow_mut() = OverlayInnerState::ToShow(instant, wnd);
    let _ = AppCtx::spawn_local(async move {
      let wnd = match (instant, &*this.0.borrow()) {
        (instant, OverlayInnerState::ToShow(crate_at, wnd)) if &instant == crate_at => wnd.clone(),
        _ => return,
      };
      let build_ctx = BuildCtx::new(None, &wnd.widget_tree);
      let style = style.unwrap_or_else(|| OverlayStyle::of(&build_ctx));
      let wid = this
        .wrap_style(w, style)
        .into_widget()
        .build(&build_ctx);
      *this.0.borrow_mut() = OverlayInnerState::Showing(wid, wnd.clone());
      let mut tree = wnd.widget_tree.borrow_mut();
      tree.root().append(wid, &mut tree);
      wid.on_mounted_subtree(&tree);
      tree.mark_dirty(wid);
    });
  }

  fn wrap_style(&self, w: Widget<'static>, style: OverlayStyle) -> impl IntoWidget<'static, FN> {
    let this = self.clone();
    fn_widget! {
      let OverlayStyle { close_policy, mask_brush } = style;
      let this2 = this.clone();
      @Container {
        size: Size::new(f32::INFINITY, f32::INFINITY),
        background: mask_brush.unwrap_or_else(|| Color::from_u32(0).into()),
        on_tap: move |e| {
          if close_policy.contains(ClosePolicy::TAP_OUTSIDE)
            && e.target() == e.current_target() {
            this.close();
          }
        },
        on_key_down: move |e| {
          if close_policy.contains(ClosePolicy::ESC)
            && *e.key() == VirtualKey::Named(NamedKey::Escape) {
            this2.close();
          }
        },
        @ { w }
      }
    }
  }

  fn close_handle(&self) -> OverlayCloseHandle { OverlayCloseHandle(self.clone()) }
}

pub(crate) struct OverlayRoot {}

impl Render for OverlayRoot {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let mut size = ZERO_SIZE;
    let mut layouter = ctx.first_child_layouter();
    while let Some(mut l) = layouter {
      let child_size = l.perform_widget_layout(clamp);
      size = size.max(child_size);
      layouter = l.into_next_sibling();
    }
    size
  }

  fn paint(&self, _: &mut PaintingCtx) {}
}

#[cfg(test)]
mod tests {
  use std::{cell::RefCell, rc::Rc};

  use ribir_dev_helper::assert_layout_result_by_path;

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

    let root = wnd.widget_tree.borrow().root();
    assert_eq!(wnd.widget_tree.borrow().count(root), 3);

    overlay.show(wnd.0.clone());
    overlay.close();
    overlay.show_at(Point::new(50., 30.), wnd.0.clone());
    wnd.draw_frame();
    assert_eq!(*r_log.borrow(), &["mounted"]);
    // the path [1, 0, 0, 0] is from root to anchor,
    // OverlayRoot -> BoxDecoration-> Container -> Anchor
    assert_layout_result_by_path!(wnd, {path = [1, 0, 0, 0], x == 50., y == 30.,});

    overlay.close();
    wnd.draw_frame();
    assert_eq!(*r_log.borrow(), &["mounted", "disposed"]);
    assert_eq!(wnd.widget_tree.borrow().count(root), 3);
  }
}
