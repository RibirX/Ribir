use log::warn;
use ribir_core::prelude::*;
use webbrowser::{open_browser as open, Browser};

#[derive(Declare, Declare2)]
pub struct Link {
  /// Want to open url
  #[declare(convert=into)]
  url: CowArc<str>,
  /// Select the browser software you expect to open
  #[declare(default=Browser::Default)]
  browser: Browser,
}

impl ComposeChild for Link {
  type Child = Widget;
  fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
    fn_widget! {
      @ $child {
        on_tap: move |_| {
          let this = $this;
          if open(this.browser, &this.url).is_err() {
            warn!("Open link fail");
          }
        },
      }
    }
    .into()
  }
}
