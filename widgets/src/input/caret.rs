use ribir_core::prelude::*;
#[derive(Declare)]
pub struct Caret {
  pub focused: bool,
  #[declare(default = svgs::TEXT_CARET)]
  pub icon: NamedSvg,
}

impl Compose for Caret {
  fn compose(this: impl StateWriter<Value = Self>) -> impl WidgetBuilder {
    let blink_interval = Duration::from_millis(500);
    fn_widget! {
      let icon = $this.icon;
      let mut caret = @ $icon {
        opacity: 0.,
        box_fit: BoxFit::CoverY,
      };
      let mut _guard = None;
      let u = watch!($this.focused)
        .subscribe(move |focused| {
          if focused {
            $caret.write().opacity = 1.;
            let unsub = interval(blink_interval, AppCtx::scheduler())
              .subscribe(move |idx| $caret.write().opacity = (idx % 2) as f32)
              .unsubscribe_when_dropped();
            _guard = Some(unsub);
          } else {
            $caret.write().opacity = 0.;
            _guard = None;
          }
        });
      @ $caret { on_disposed: move |_| u.unsubscribe() }
    }
  }
}
