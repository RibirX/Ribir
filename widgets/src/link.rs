use log::warn;
use ribir_core::prelude::*;
use webbrowser::{Browser, open_browser as open};

#[derive(Declare)]
pub struct UrlLink {
  /// Want to open url
  url: CowArc<str>,
  /// Select the browser software you expect to open
  #[declare(default=Browser::Default)]
  browser: Browser,
}

impl<'c> ComposeChild<'c> for UrlLink {
  type Child = Widget<'c>;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    FatObj::new(child)
      .on_tap(move |_| {
        let this = this.read();
        if open(this.browser, &this.url).is_err() {
          warn!("Open link fail");
        }
      })
      .into_widget()
  }
}
