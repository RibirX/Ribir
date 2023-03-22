#[cfg(any(feature = "crossterm", feature = "winit"))]
fn main() {
  use ribir::Application;
  use ribir_core::window::WindowConfig;

  use ribir::prelude::{svgs, *};

  const WINDOW_SIZE: f32 = 800.;

  let widgets = widget! {
    init ctx => {
      let title_style = TypographyTheme::of(ctx).display_large.text.clone();
      let foreground = Palette::of(ctx).on_surface_variant().into();
    }
    Column {
      margin: EdgeInsets::horizontal(20.),
      Column {
        margin: EdgeInsets::only_top(20.),
        Text::new("Button", &foreground, title_style.clone())
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
        Divider { extent: 30., end_indent: 150. }
        Row {
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
        Divider { extent: 30., end_indent: 150. }
        Row {
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
        Divider { extent: 30., end_indent: 150. }
        Row {
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

  let mut app = Application::new(material::purple::light());

  let window_builder = app.window_builder(
    widgets,
    WindowConfig {
      inner_size: Some((WINDOW_SIZE, WINDOW_SIZE).into()),
      title: Some("StoryBook".to_owned()),
      ..Default::default()
    },
  );

  let window_id = app.build_window(window_builder);
  app.exec(window_id);
}

#[cfg(not(any(feature = "crossterm", feature = "winit")))]
fn main() {
  println!("Chose a platform to run:");
  println!("  cargo run --example widget_examples -F winit,wgpu_gl");
  println!("  cargo run --example widget_examples -F crossterm");
}
