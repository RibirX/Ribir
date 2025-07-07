use ribir_core::prelude::*;

use crate::{LayerArea, PressedLayer, md};

/// Widget use to do ripple animate as a visual feedback to user interactive.
/// Usually for touch and mouse.
#[derive(Declare)]
pub struct Ripple {
  /// The radius in pixels of foreground ripples when fully expanded. The
  /// default radius will be the distance from the center of the ripple to the
  /// furthest corner of the host bounding rectangle.
  #[declare(default)]
  pub ripple_radius: Option<f32>,
  /// Whether the ripple always originates from the center of the host bound.
  #[declare(default)]
  pub center: bool,
  #[declare(default)]
  /// If the ripple need boundary in the widget or not.
  pub bounded: bool,
  #[declare(default)]
  launcher: Option<Box<dyn Fn(Option<Point>)>>,
}

impl<'c> ComposeChild<'c> for Ripple {
  type Child = Widget<'c>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    fn_widget! {
      let mut ripple_layer = PressedLayer::new(LayerArea::FullContent);
      init_ripple_launcher(&this, &mut ripple_layer);

      @(ripple_layer) {
        on_pointer_down: move |e| {
          let this = $read(this);
          let pos = (!this.center).then(||e.position());
          this.launch(pos);
          e.stop_propagation();
        },
        on_disposed: move |_| $write(this).launcher = None,
        @ { child }
      }
    }
    .into_widget()
  }
}

impl Ripple {
  /// Manual launch a ripple animate at `pos`.
  pub fn launch(&self, pos: Option<Point>) { self.launcher.as_ref().inspect(|l| l(pos)); }
}

fn init_ripple_launcher(
  this: &impl StateWriter<Value = Ripple>, layer: &mut FatObj<Stateful<PressedLayer>>,
) {
  rdl! {
    let circle_state = LerpFnState::new(
      part_writer!(&mut layer.area),
      move |_, to, factor| {
        let LayerArea::Circle { center, radius, constrain_to_bounds } = *to else { unreachable!() };
        LayerArea::Circle { center, radius: f32::lerp(&0., &radius, factor), constrain_to_bounds }
      }
    );
    let ripple_grow = @Animate {
      state: (circle_state, part_writer!(&mut layer.draw_opacity)),
      transition: EasingTransition {
        easing: md::easing::EMPHASIZED_DECELERATE,
        duration: md::easing::duration::SHORT3,
      }.box_it(),
      from: (LayerArea::FullContent, 0.),
    };

    let fade_out = @Animate {
      state: part_writer!(&mut layer.draw_opacity),
      transition: EasingTransition{
        easing: md::easing::STANDARD_ACCELERATE,
        duration: md::easing::duration::MEDIUM3
      }.box_it(),
      from: PressedLayer::show_opacity(),
    };

    watch!(!$read(ripple_grow).is_running() && !*$read(layer.is_pointer_pressed()))
      .skip(1)
      .distinct_until_changed()
      .filter(|fade| *fade)
      .subscribe(move |_| {
        $write(layer).hide();
        fade_out.run();
      });

    let launcher = move |pos: Option<Point>| {
      let size = *$read(layer.layout_size());
      let center = pos.unwrap_or_else(|| {
        (size / 2.).to_vector().to_point()
      });
      let radius = $read(this).ripple_radius.unwrap_or_else(|| {
        let distance_x = f32::max(center.x , size.width - center.x);
        let distance_y = f32::max(center.y, size.height - center.y);
        (distance_x.powf(2.) + distance_y.powf(2.)).sqrt()
      });
      let constrain_to_bounds = $read(this).bounded;
      {
        let mut layer = $write(layer);
        layer.area = LayerArea::Circle { center, radius, constrain_to_bounds };
        layer.show();
      }
      ripple_grow.run()
    };

    $write(this).launcher = Some(Box::new(launcher));
  }
}
