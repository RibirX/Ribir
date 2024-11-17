use crate::prelude::*;

impl Render for Svg {
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, _: &mut LayoutCtx) -> Size { clamp.clamp(self.size) }

  fn paint(&self, ctx: &mut PaintingCtx) {
    let painter = ctx.painter();
    painter.draw_svg(self);
  }
}

pub mod named_svgs {
  use std::sync::{LazyLock, Mutex};

  pub use super::*;

  const DEFAULT_SVG_KEY: &str = "__RIRBIR_DEFAULT_SVG__";
  static SVGS: LazyLock<Mutex<ahash::AHashMap<&'static str, Svg>>> = LazyLock::new(|| {
    let svg = include_crate_svg!("src/builtin_widgets/default_named.svg");
    let mut set = ahash::AHashMap::new();
    set.insert(DEFAULT_SVG_KEY, svg);
    Mutex::new(set)
  });

  /// Register an SVG with a specific name. You can then use the same `name`
  /// parameter with [`named_svgs::get`](get) to retrieve it.
  ///
  /// To prevent conflicts, it is recommended to add a namespace prefix from
  /// your library or application to the name, such as `ribir::add`.
  pub fn register(name: &'static str, svg: Svg) { SVGS.lock().unwrap().insert(name, svg); }

  /// Retrieve a named SVG that was registered using
  /// [`named_svgs::register`](register).
  pub fn get(name: &str) -> Option<Svg> { SVGS.lock().unwrap().get(name).cloned() }

  /// Functions similarly to [`named_svgs::get`](get), but returns the
  /// default SVG if not found.
  pub fn get_or_default(name: &str) -> Svg {
    get(name).unwrap_or_else(|| get(DEFAULT_SVG_KEY).unwrap())
  }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
  use ribir_dev_helper::*;

  use super::*;

  fn svgs_smoke() -> Painter {
    named_svgs::register(
      "test::add",
      Svg::parse_from_bytes(
        r#"<svg xmlns="http://www.w3.org/2000/svg" height="48" width="48"><path d="M22.5 38V25.5H10v-3h12.5V10h3v12.5H38v3H25.5V38Z"/></svg>"#.as_bytes(),
      ).unwrap(),
    );
    let mut painter = Painter::new(Rect::from_size(Size::new(128., 64.)));
    let add = named_svgs::get("test::add").unwrap();
    let x = named_svgs::get_or_default("x");

    painter
      .draw_svg(&add)
      .translate(64., 0.)
      .draw_svg(&x);

    painter
  }

  painter_backend_eq_image_test!(svgs_smoke, comparison = 0.001);
}
