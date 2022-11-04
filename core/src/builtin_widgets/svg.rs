use std::error::Error;

use crate::{impl_query_self_only, prelude::*};

#[derive(Declare)]
pub struct Svg {
  bytes: Vec<u8>,
}

impl Compose for Svg {
  fn compose(this: StateWidget<Self>) -> Widget {
    widget_try_track! {
      try_track { this }
      env { let theme = ctx.theme().clone(); }
      DynWidget {
        dyns: match SvgRender::parse_from_bytes(&this.bytes) {
          Ok(reader) => reader.into_widget(),
          Err(err) =>  {
            log::warn!("Parse svg failed: {err}");
            IconTheme::of(&theme).miss_icon.clone().into_widget()
          }
        }
      }
    }
  }
}

/// Widget paint the svg.
#[derive(Debug)]
pub struct SvgRender(pub SvgPaths);

impl SvgRender {
  #[inline]
  pub fn parse_from_bytes(svg_data: &[u8]) -> Result<Self, Box<dyn Error>> {
    SvgPaths::parse_from_bytes(svg_data).map(Self)
  }

  #[inline]
  pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Self, Box<dyn Error>> {
    SvgPaths::open(path).map(Self)
  }
}
impl Render for SvgRender {
  #[inline]
  fn perform_layout(&self, _: BoxClamp, _: &mut LayoutCtx) -> Size { self.0.size }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) {
    let painter = ctx.painter();
    self.0.paths.iter().for_each(|c| {
      if let Some(b) = c.brush.as_ref() {
        painter.set_brush(b.clone());
      }
      painter
        .apply_transform(&c.transform)
        .paint_path(c.path.clone());
    });
  }
}

impl Query for SvgRender {
  impl_query_self_only!();
}
