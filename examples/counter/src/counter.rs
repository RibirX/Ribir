use ribir::prelude::*;

pub fn counter() -> impl WidgetBuilder {
  fn_widget! {
    let cnt = Stateful::new(0);

    @Column {
      h_align: HAlign::Center,
      align_items: Align::Center,
      @FilledButton {
        on_tap: move |_: &mut _| *$cnt.write() += 1,
        @{ Label::new("Add") }
      }
      @H1 { text: pipe!($cnt.to_string())  }
      @FilledButton {
        on_tap: move |_: &mut _| *$cnt.write() += -1,
        @{ Label::new("Sub") }
      }
    }
  }
}
