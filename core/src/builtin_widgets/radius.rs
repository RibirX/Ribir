use ribir_painter::Radius;
use wrap_render::WrapRender;

use super::*;

/// A widget that provides a radius for the host widget, applying it to both the
/// background and border of the widget.
///
/// If you set the radius in different `FatObj`, ensure it is set in the
/// outermost `FatObj`. Otherwise, the outer border or background will ignore
/// it.
///
/// For example:
///
/// ```
/// use ribir::prelude::*;
///
/// let _ = fn_widget! {
///   @Background {
///     background: Color::RED,
///     @RadiusWidget {
///       radius: Radius::all(10.),
///       @BorderWidget {
///         border: Border::all(BorderSide::new(1., Color::BLACK.into())),
///         @Container {
///           size: Size::new(100., 100.),
///         }
///       }
///     }
///   }
/// };
/// ```
///
/// This widget will create a border with a radius of 10 and a red box without a
/// radius.
#[derive(Default, Clone)]
pub struct RadiusWidget {
  /// A border to draw above the background
  pub radius: Radius,
}

impl Declare for RadiusWidget {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl WrapRender for RadiusWidget {
  fn paint(&self, host: &dyn Render, ctx: &mut PaintingCtx) {
    let mut provider = Provider::new(self.radius);
    provider.setup(ctx.as_mut());
    host.paint(ctx);
    provider.restore(ctx.as_mut());
  }
}

impl_compose_child_for_wrap_render!(RadiusWidget, DirtyPhase::Paint);
