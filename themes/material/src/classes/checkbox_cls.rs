use easing::CubicBezierEasing;
use ribir_core::prelude::*;
use ribir_widgets::checkbox::*;

use crate::*;

pub(super) fn init(classes: &mut Classes) {
  classes.insert(
    CHECKBOX_UNCHECKED,
    style_class! { foreground: Palette::of(BuildCtx::get()).on_surface_variant() },
  );
  classes.insert(CHECKBOX_CHECKED, style_class! { foreground: BuildCtx::color() });
  classes.insert(CHECKBOX_INDETERMINATE, style_class! { foreground: BuildCtx::color() });

  classes.insert(CHECKBOX, |w| {
    let margin = if Provider::of::<DisableInteractiveLayer>(BuildCtx::get()).is_some() {
      EdgeInsets::all(3.)
    } else {
      md::EDGES_4
    };
    interactive_layers! {
      ripple_radius: 20.,
      center: true,
      ring_outer_offset: 2.,

      margin,
      radius: md::RADIUS_20,
      clamp: BoxClamp::fixed_size(md::SIZE_40),
      text_line_height: 18.,
      cursor: CursorIcon::Pointer,
      @ { w }
    }
    .into_widget()
  });

  const ICON_TRANS: EasingTransition<CubicBezierEasing> = EasingTransition {
    duration: md::easing::duration::SHORT3,
    easing: md::easing::EMPHASIZED_DECELERATE,
  };

  fn checked_icon_container<'w>(icon: Widget<'w>) -> Widget<'w> {
    container! {
      size: md::SIZE_18,
      background: BuildCtx::color(),
      radius: md::RADIUS_2,
      @ { icon }
    }
    .into_widget()
  }

  classes.insert(CHECKBOX_CHECKED_ICON, |w| {
    let mut builder = Path::builder();
    builder
      .begin_path(Point::new(3.5, 8.5))
      .line_to(Point::new(7., 12.))
      .line_to(Point::new(14.5, 4.5))
      .end_path(false);
    let check = Stateful::new(Resource::new(builder.build()));
    let sampler = check.read().sampler();
    let empty_path = Resource::new(sampler.normalized_sub_path(0f32..0f32));

    fn_widget! {
      let enter = @Animate {
        state: LerpFnState::new(check.clone_writer(), move |_, _, rate| {
          let sub_path = sampler.normalized_sub_path(0f32..rate);
          Resource::new(sub_path)
        }),
        transition: ICON_TRANS,
        from: empty_path
      };
      @Stack {
        on_mounted: move |_| enter.run(),
        foreground: BuildCtx::color().on_this_color(BuildCtx::get()),
        painting_style: PaintingStyle::Stroke(StrokeOptions {
          width: 2.,
          ..Default::default()
        }),
        @checked_icon_container(check.into_widget())
        @ { w }
      }
    }
    .into_widget()
  });
  classes.insert(CHECKBOX_INDETERMINATE_ICON, |w| {
    let icon = rdl! {
      let mut icon = @Container{
        size: Size::new(12., 2.),
        x: AnchorX::center(),
        y: AnchorY::center(),
        background: BuildCtx::color().on_this_color(BuildCtx::get()),
      };
      let enter = @Animate {
        state: (icon.width(), icon.height()),
        transition: ICON_TRANS,
        from: (Dimension::Fixed(0_f32.px()), Dimension::Fixed(2_f32.px())),
      };
      @(icon) {
        on_mounted: move |_| enter.run(),
        @ { w }
      }
    };
    checked_icon_container(icon.into_widget())
  });

  classes.insert(
    CHECKBOX_UNCHECKED_ICON,
    style_class! {
      border: md::border_2_surface_color(),
      radius: md::RADIUS_2,
      clamp: BoxClamp::fixed_size(md::SIZE_18),
    },
  );
}
