use easing::CubicBezierEasing;
use ribir_core::prelude::*;
use ribir_widgets::checkbox::*;

use crate::*;

pub(super) fn init(classes: &mut Classes) {
  classes.insert(CHECKBOX, |w| {
    let hover_layer = HoverLayer::tracked(LayerArea::WidgetCover(md::RADIUS_20));
    ripple! {
      cursor: CursorIcon::Pointer,
      ripple_radius: 20.,
      center: true,
      @ $hover_layer {
        clamp: BoxClamp::fixed_size(md::SIZE_40),
        @ { w }
      }
    }
    .into_widget()
  });

  fn icon_with_ripple<'w>(
    icon: Widget<'w>, ripple: Widget<'w>, foreground: Variant<Color>,
  ) -> Widget<'w> {
    stack! {
      margin: md::EDGES_4,
      foreground,
      @Icon {
        clamp: BoxClamp::fixed_size(md::SIZE_40),
        text_line_height: 18.,
        @ { icon }
      }
      @{ ripple }
    }
    .into_widget()
  }

  const ICON_TRANS: EasingTransition<CubicBezierEasing> = EasingTransition {
    duration: md::easing::duration::SHORT3,
    easing: md::easing::EMPHASIZED_DECELERATE,
  };

  fn check_icon_with_ripple<'w>(icon: Widget<'w>, ripple: Widget<'w>) -> Widget<'w> {
    let icon = container! {
      size: md::SIZE_18,
      background: BuildCtx::color(),
      radius: md::RADIUS_2,
      @ { icon }
    };
    icon_with_ripple(icon.into_widget(), ripple, BuildCtx::color())
  }

  classes.insert(CHECKBOX_CHECKED, |w| {
    let icon = rdl! {
      let mut builder = Path::builder();
      builder
        .begin_path(Point::new(3.5, 8.5))
        .line_to(Point::new(7., 12.))
        .line_to(Point::new(14.5, 4.5))
        .end_path(false);
      let check = Stateful::new(Resource::new(builder.build()));
      let sampler = check.read().sampler();
      let empty_path = Resource::new(sampler.normalized_sub_path(0f32..0f32));

      let enter = @Animate {
        state: LerpFnState::new(check.clone_writer(), move |_, _, rate| {
          let sub_path = sampler.normalized_sub_path(0f32..rate);
          Resource::new(sub_path)
        }),
        transition: ICON_TRANS.box_it(),
        from: empty_path
      };
      @FatObj {
        on_mounted: move |_| enter.run(),
        foreground: BuildCtx::color().on_this_color(BuildCtx::get()),
        painting_style: PaintingStyle::Stroke(StrokeOptions {
          width: 2.,
          ..Default::default()
        }),
        @ { check }
      }
    }
    .into_widget();

    check_icon_with_ripple(icon, w)
  });
  classes.insert(CHECKBOX_INDETERMINATE, |w| {
    let icon = rdl! {
      let icon = @Container{
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
      @ $icon { on_mounted: move |_| enter.run() }
    };
    check_icon_with_ripple(icon.into_widget(), w)
  });

  classes.insert(CHECKBOX_UNCHECKED, |w| {
    let icon = container! {
      size: md::SIZE_18,
      border: md::border_2_surface_color(),
      radius: md::RADIUS_2,
      clamp: BoxClamp::fixed_size(md::SIZE_18),
    }
    .into_widget();

    let foreground = Palette::of(BuildCtx::get()).on_surface_variant();
    icon_with_ripple(icon, w, Variant::Value(foreground))
  });
}
