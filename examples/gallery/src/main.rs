use ribir::prelude::*;
mod app;
mod sections;
mod styles;

fn main() { App::run(app::gallery).with_app_theme(material::purple::light); }
