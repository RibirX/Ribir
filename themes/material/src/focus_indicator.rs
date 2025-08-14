use ribir_core::{prelude::*, wrap_render::WrapRender};

use crate::{md, state_layer::*};

const RING_STROKE_WIDTH: f32 = 3.0;

/// A provider used to hint the `FocusIndicator` should show a focus ring or
/// focus layer.
///
/// As default, the `FocusIndicator` will show the focus ring in web platform,
/// and a focus layer in the native platform.
///
/// Note: `FocusIndicator` treat it as a static provider, and will not react to
/// its dynamic changes.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ShowFocusRing(pub bool);

/// A widget that shows a focus ring or focus layer when the widget is focused
/// by keyboard.
#[derive(Declare, Clone, Copy, PartialEq, Debug)]
pub struct FocusIndicator {
  /// The outer offset of the focus ring.
  #[declare(default)]
  pub ring_outer_offset: f32,
}

impl<'c> ComposeChild<'c> for FocusIndicator {
  type Child = Widget<'c>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    let this = this
      .try_into_value()
      .unwrap_or_else(|_| panic!("FocusIndicator should be a stateless widget"));

    FocusIndicator::combine_in(this, FatObj::new(child))
  }
}

impl FocusIndicator {
  pub fn combine_in(self, mut child: FatObj<Widget>) -> Widget {
    let lazy = move || {
      let show_focus_ring = Provider::of::<ShowFocusRing>(BuildCtx::get())
        .map_or(ShowFocusRing(cfg!(target_arch = "wasm32")), |r| *r);

      if show_focus_ring.0 {
        let ring = FocusRing::create_for(self.ring_outer_offset, &mut child);
        ring.with_child(child).into_widget()
      } else {
        let layer = FocusLayer::create_for(&mut child);
        layer.with_child(child).into_widget()
      }
    };

    lazy.into_widget()
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FocusRing {
  outer_offset: f32,
  stroke_width: f32,
}

impl_compose_child_for_wrap_render!(FocusRing);

impl FocusRing {
  pub fn create_for(outer_offset: f32, host: &mut FatObj<Widget>) -> Stateful<Self> {
    let ring = Stateful::new(Self { outer_offset, stroke_width: 0. });

    let mut animate = Animate::declarer();
    animate
      .with_from(0.)
      .with_state(part_writer!(&mut ring.stroke_width))
      .with_transition(EasingTransition {
        duration: md::easing::duration::MEDIUM2,
        easing: easing::CubicBezierEasing::new(0.05, 1., 0.4, 3.),
      });
    let animate = animate.finish();

    let u = watch! {
      *$read(host.is_focused()) &&
      *$read(host.focus_changed_reason()) == FocusReason::Keyboard
    }
    .subscribe({
      let ring = ring.clone_writer();
      move |has_focus| {
        ring.write().set_stroke_width(has_focus);
        if has_focus {
          animate.run();
        }
      }
    });

    host.on_disposed(move |_| u.unsubscribe());

    ring
  }

  fn set_stroke_width(&mut self, has_focus: bool) {
    self.stroke_width = if has_focus { RING_STROKE_WIDTH } else { 0. };
  }
}

impl WrapRender for FocusRing {
  fn paint(&self, host: &dyn Render, ctx: &mut PaintingCtx) {
    if self.stroke_width <= 0.0 {
      return host.paint(ctx);
    }

    let outer_offset = self.outer_offset + self.stroke_width / 2.;
    let rect = Rect::from_size(ctx.box_size().unwrap())
      .outer_rect(SideOffsets2D::new_all_same(outer_offset));
    let widget_radius = Provider::of::<Radius>(ctx).map(|r| r.add_to_all(outer_offset));
    let color = *Provider::of::<Color>(ctx).unwrap();

    let painter = ctx.painter();
    painter
      .set_style(PathStyle::Stroke)
      .set_stroke_brush(color)
      .set_line_width(self.stroke_width);

    if let Some(radius) = widget_radius {
      painter.rect_round(&rect, &radius);
    } else {
      painter.rect(&rect);
    }

    painter.stroke();
    host.paint(ctx);
  }

  #[inline]
  fn wrapper_dirty_phase(&self) -> DirtyPhase { DirtyPhase::Paint }
}
