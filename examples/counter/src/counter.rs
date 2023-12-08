use ribir::prelude::*;

pub fn counter() -> impl WidgetBuilder {
  fn_widget! {
    let cnt = Stateful::new(0);
    @Row {
      @FilledButton {
        on_tap: move |_| *$cnt.write() += 1,
        @{ Label::new("Inc") }
      }
      @H1 { text: pipe!($cnt.to_string()) }
    }
  }
}
