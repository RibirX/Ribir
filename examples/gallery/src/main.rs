use ribir::prelude::*;
mod app;
mod sections;
mod styles;

const GALLERY_WINDOW_SIZE: Size = Size::new(1080., 840.);

fn main() {
  App::run(app::gallery)
    .with_app_theme(material::purple::light)
    .with_size(GALLERY_WINDOW_SIZE)
    .with_title("Gallery");
}
