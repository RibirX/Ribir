use std::cell::RefCell;

use crate::prelude::*;

class_names! {
  #[doc = "Class name for the tooltips"]
  TOOLTIPS,
}
/// Add attributes of tooltips to Widget Declarer.
///
/// ### Example:
/// ```no_run
/// use ribir::prelude::*;
///
/// let w = text! {
///     text: "hover to show tooltips!",
///     tooltips: "this is tooltips",
///   }
/// };
/// App::run(w);
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
  fn tooltips(&self) -> &CowArc<str> { &self.tooltips }

  pub fn show(&self, wnd: Sc<Window>) {
    if let Some(overlay) = self.overlay.borrow().clone() {
      if !overlay.is_showing() {
        overlay.show(wnd);
      }
    }
  }

  pub fn hidden(&self) {
    if let Some(overlay) = self.overlay.borrow().clone() {
      if overlay.is_showing() {
        overlay.close();
      }
    }
  }
}

impl<'c> ComposeChild<'c> for Tooltips {
  type Child = Widget<'c>;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    fn_widget! {
      let mut child = FatObj::new(child);
      *$this.overlay.borrow_mut() = Some(Overlay::new(
        move || {
          let w = @Text {
            text: pipe!($this.tooltips().clone()),
            class: TOOLTIPS,
          };

          @ $w {
            global_anchor_x: pipe!(
              GlobalAnchorX::center_align_to(
                $child.track_id(), 0.
              ).always_follow()
            ),
            global_anchor_y: pipe!(
              GlobalAnchorY::bottom_align_to(
                $child.track_id(), $child.layout_size().height
              ).always_follow()
            ),
          }.into_widget()
        },  OverlayStyle {
          auto_close_policy: AutoClosePolicy::NOT_AUTO_CLOSE,
          mask: None,
        }
      ));

      let wnd = BuildCtx::get().window();
      let u = watch!($child.is_hover())
        .delay(Duration::from_millis(150), AppCtx::scheduler())
        .distinct_until_changed()
        .subscribe(move |_| {
          if $child.is_hover() {
            $this.show(wnd.clone());
          } else {
            $this.hidden();
          }
        });

      @ $child {
        on_disposed: move|_| {
          u.unsubscribe();
          $this.hidden();
        },
      }
    }
    .into_widget()
  }
}
