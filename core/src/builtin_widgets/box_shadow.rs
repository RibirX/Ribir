use crate::{prelude::*, wrap_render::WrapRender};

#[derive(Clone, Copy)]
pub struct BoxShadow {
  pub offset: Point,
  pub blur_radius: f32,
  pub spread_radius: f32,
  pub color: Color,
}

impl Default for BoxShadow {
  fn default() -> Self {
    Self {
      color: Color::TRANSPARENT,
      blur_radius: 0.,
      offset: Point::new(0., 0.),
      spread_radius: 0.0,
    }
  }
}

impl BoxShadow {
  pub fn new(offset: Point, blur_radius: f32, spread_radius: f32, color: Color) -> Self {
    Self { color, blur_radius, offset, spread_radius }
  }

  pub fn is_empty(&self) -> bool {
    self.color.alpha == 0
      || (self.blur_radius == 0. && self.offset == Point::new(0., 0.) && self.spread_radius == 0.)
  }
  pub fn shadow_rect(&self, size: Size, blur_radius: f32) -> Rect {
    let spread_radius = self.spread_radius;
    let offset = self.offset;

    let x = offset.x - blur_radius - spread_radius;
    let y = offset.y - blur_radius - spread_radius;
    let width = size.width + (spread_radius + blur_radius) * 2.0;
    let height = size.height + (spread_radius + blur_radius) * 2.0;

    Rect::new(Point::new(x, y), Size::new(width, height))
  }
}

/// A widget that renders a box shadow for its host by drawing a colored area
/// and applying an optional blur.
///
/// This is a built-in `FatObj` field. Setting the `box_shadow` field attaches
/// a `BoxShadowWidget` to the host to render shadow effects.
///
/// # Example
///
/// Apply a red box shadow to a container.
///
/// ```rust
/// use ribir::prelude::*;
///
/// container! {
///   size: Size::new(100., 100.),
///   radius: Radius::all(50.),
///   box_shadow: BoxShadow::new(Point::new(45., 15.), 20., 12., Color::RED),
/// };
/// ```
#[derive(Default, Clone)]
pub struct BoxShadowWidget {
  pub box_shadow: BoxShadow,
}

impl Declare for BoxShadowWidget {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl WrapRender for BoxShadowWidget {
  fn paint(&self, host: &dyn Render, ctx: &mut PaintingCtx) {
    let size = ctx.box_size().unwrap();

    if size.is_empty() || self.box_shadow.is_empty() {
      host.paint(ctx);
      return;
    }
    let box_shadow = self.box_shadow;
    let blur_radius = box_shadow.blur_radius.round().max(0.);

    let shadow_rect = box_shadow.shadow_rect(size, blur_radius);
    let shadow_size = shadow_rect.size;

    // Check if there's a radius defined for rounded corners
    let radius: Option<Radius> = {
      let (provider_ctx, _) = ctx.provider_ctx_and_box_painter();
      Provider::of::<Radius>(provider_ctx).map(|r| *r)
    };

    // build clip path, exclude the original box
    let mut builder = path_builder::PathBuilder::default();
    builder.rect(&shadow_rect, true); // positive winding for outer rectangle
    if let Some(radius) = &radius {
      builder.rect_round(&Rect::from_size(size), radius, false); // negative winding to exclude inner rounded rect
    } else {
      builder.rect(&Rect::from_size(size), false); // negative winding to exclude inner rectangle
    }

    ctx.painter().save();
    let mut painter = ctx.painter().fork();
    ctx.painter().clip(builder.build().into());

    let actual_shadow_size =
      Size::new(shadow_size.width - blur_radius * 2.0, shadow_size.height - blur_radius * 2.0);
    let scale_x = actual_shadow_size.width / size.width;
    let scale_y = actual_shadow_size.height / size.height;

    painter.translate(
      box_shadow.offset.x - box_shadow.spread_radius,
      box_shadow.offset.y - box_shadow.spread_radius,
    );

    // Apply blur filter if blur radius is specified
    if blur_radius > 0. {
      painter.filter(Filter::blur(blur_radius));
    }

    painter.scale(scale_x, scale_y);

    // Use rounded rectangle if radius is provided, otherwise use regular rectangle
    if let Some(radius) = radius {
      painter.rect_round(&Rect::from_size(size), &radius, true);
    } else {
      painter.rect(&Rect::from_size(size), true);
    }

    painter.set_fill_brush(box_shadow.color).fill();

    ctx.painter().merge(&mut painter);
    ctx.painter().restore();
    // Now paint the actual content on top
    host.paint(ctx);
  }

  fn visual_box(&self, host: &dyn Render, ctx: &mut VisualCtx) -> Option<Rect> {
    let size = ctx.box_size().unwrap();
    let box_shadow = self.box_shadow;
    let blur_radius = box_shadow.blur_radius.round();

    let shadow_rect = box_shadow.shadow_rect(size, blur_radius);
    Some(
      host
        .visual_box(ctx)
        .map(|rect| rect.union(&shadow_rect))
        .unwrap_or(shadow_rect),
    )
  }

  #[inline]
  fn wrapper_dirty_phase(&self) -> DirtyPhase { DirtyPhase::Paint }
}

impl_compose_child_for_wrap_render!(BoxShadowWidget);

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
  use ribir::{core::test_helper::*, material as ribir_material, prelude::*};
  use ribir_dev_helper::*;

  #[cfg(feature = "png")]
  widget_image_tests!(
    box_shadow_basic,
    WidgetTester::new(fn_widget! {
      @Container {
        size: Size::new(100., 100.),
        radius: Radius::all(50.),
        box_shadow: BoxShadow::new(Point::new(45., 15.), 20., 12., Color::RED.with_alpha(0.6)),
      }
    })
    .with_comparison(0.00005)
    .with_wnd_size(Size::new(200., 200.))
  );
}
