use ribir::prelude::{svgs, *};

const WINDOW_SIZE: f32 = 800.;

fn main() {
  let widgets = widget! {
    init ctx => {
      let label_style = TypographyTheme::of(ctx).display_large.text.clone();
      let foreground = Palette::of(ctx).on_surface_variant().into();
    }
    Column {
      margin: EdgeInsets::horizontal(20.),
      Column {
        margin: EdgeInsets::only_top(20.),
        Text { text: "Button", foreground, label_style }
        Row {
          margin: EdgeInsets::only_top(20.),
          FilledButton { svgs::ADD }
          SizedBox { size: Size::new(20., 0.) }
          FilledButton { Label::new("Filled button") }
          SizedBox { size: Size::new(20., 0.) }
          FilledButton {
            color: Color::RED,
            svgs::ADD
            Label::new("Filled button")
          }
          SizedBox { size: Size::new(20., 0.) }
          FilledButton {
            color: Color::GREEN,
            svgs::ADD
            Label::new("Filled button")
          }
        }
        Row {
          margin: EdgeInsets::only_top(20.),
          OutlinedButton { svgs::ADD }
          SizedBox { size: Size::new(20., 0.) }
          OutlinedButton { Label::new("Outlined button") }
          SizedBox { size: Size::new(20., 0.) }
          OutlinedButton {
            color: Color::RED,
            svgs::ADD
            Label::new("Outlined button")
          }
          SizedBox { size: Size::new(20., 0.) }
          OutlinedButton {
            color: Color::GREEN,
            svgs::ADD
            Label::new("Outlined button")
          }
        }
        Row {
          margin: EdgeInsets::only_top(20.),
          Button { svgs::ADD }
          SizedBox { size: Size::new(20., 0.) }
          Button { Label::new("Raw button") }
          SizedBox { size: Size::new(20., 0.) }
          Button {
            color: Color::RED,
            Label::new("Raw button")
          }
          SizedBox { size: Size::new(20., 0.) }
          Button {
            color: Color::GREEN,
            Label::new("Raw button")
          }
        }
        Row {
          margin: EdgeInsets::only_top(20.),
          FabButton { svgs::ADD }
          SizedBox { size: Size::new(20., 0.) }
          FabButton { Label::new("Fab button") }
          SizedBox { size: Size::new(20., 0.) }
          FabButton {
            color: Color::RED,
            svgs::ADD
            Label::new("Fab button")
          }
          SizedBox { size: Size::new(20., 0.) }
          FabButton {
            color: Color::GREEN,
            svgs::ADD
            Label::new("Fab button")
          }
        }
      }
    }
  };
  let app = Application::new(material::purple::light());
  let wnd = Window::builder(widgets)
    .with_inner_size(Size::new(WINDOW_SIZE, WINDOW_SIZE))
    .with_title("StoryBook")
    .build(&app);
  app::run_with_window(app, wnd);
}
