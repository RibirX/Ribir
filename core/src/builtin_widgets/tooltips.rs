use std::cell::RefCell;

use crate::prelude::*;

class_names! {
  #[doc = "Class name for the tooltips"]
  TOOLTIPS,
}
/// Adds tooltip behavior to a widget declarer.
///
/// This built-in `FatObj` field attaches a `Tooltips` overlay that shows
/// when the host is hovered.
///
/// # Example
///
/// Hover the text to show a tooltip.
///
/// ```rust no_run
/// use ribir::prelude::*;
///
/// text! {
///   text: "Hover me",
///   tooltips: "I'm a tooltip!",
/// };
/// ```
#[derive(Default)]
pub struct Tooltips {
  pub tooltips: CowArc<str>,

  overlay: RefCell<Option<Overlay>>,
}

impl Declare for Tooltips {
  type Builder = FatObj<()>;
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl Tooltips {
  pub fn show(&self, wnd: Rc<Window>) {
    if let Some(overlay) = self.overlay.borrow().clone()
      && !overlay.is_showing()
    {
      overlay.show(wnd);
    }
  }

  pub fn hidden(&self) {
    if let Some(overlay) = self.overlay.borrow().clone()
      && overlay.is_showing()
    {
      overlay.close();
    }
  }
}

impl<'c> ComposeChild<'c> for Tooltips {
  type Child = Widget<'c>;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    let mut child = FatObj::new(child);

    let content = text! {
      text: pipe!($read(this).tooltips.clone()),
      class: TOOLTIPS,
      global_anchor_x: {
        let track_id = $clone(child.track_id());
        GlobalAnchorX::center_align_to(track_id).always_follow()
      },
      global_anchor_y: {
        let track_id = $clone(child.track_id());
        let height = *$read(child.layout_height());
        GlobalAnchorY::bottom_align_to(track_id).offset(height).always_follow()
      },
    };
    *this.read().overlay.borrow_mut() = Some(Overlay::new(
      content,
      OverlayStyle { auto_close_policy: AutoClosePolicy::NOT_AUTO_CLOSE, mask: None },
    ));

    fn_widget! {
      let wnd = BuildCtx::get().window();
      let u = watch!(*$read(child.is_hovered()))
        .delay(Duration::from_millis(50))
        .distinct_until_changed()
        .subscribe(move |_| {
          if *$read(child.is_hovered()) {
            $read(this).show(wnd.clone());
          } else {
            $read(this).hidden();
          }
        });

      child.on_disposed(move |_| {
        u.unsubscribe();
        $read(this).hidden();
      });

      child
    }
    .into_widget()
  }
}
