use ribir::prelude::*;
mod app;

fn main() { App::run(app::gallery).with_app_theme(material::purple::light); }
