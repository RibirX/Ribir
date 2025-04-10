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
    let mut w = FatObj::new(w);
    w.text_line_height(18.)
      .cursor(CursorIcon::Pointer);

    if DisabledRipple::get(BuildCtx::get()) {
      // 24x24 if no ripple
      w.margin(EdgeInsets::all(3.));
      return w.into_widget();
    }

    interactive_layers! {
      clamp: BoxClamp::fixed_size(md::SIZE_40),
      margin: md::EDGES_4,
      radius: md::RADIUS_20,
      ripple_radius: 20.,
      center: true,
      ring_outer_offset: 2.,
      @ { w}
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
        transition: ICON_TRANS.box_it(),
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
        h_align: HAlign::Center,
        v_align: VAlign::Center,
        background: BuildCtx::color().on_this_color(BuildCtx::get()),
      };
      let enter = @Animate {
        state: part_writer!(&mut icon.size),
        transition: ICON_TRANS.box_it(),
        from: Size::new(0., 2.),
      };
      @ $icon {
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
