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
  #[declare(default=RippleBound::Unbounded)]
  /// How ripples show outside of the host widget box.
  pub bounded: RippleBound,
  #[declare(default)]
  launcher: Option<Box<dyn Fn(Option<Point>)>>,
}

/// Config how ripples show outside of the host widget box.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum RippleBound {
  /// Ripples visible outside of the host widget.
  Unbounded,
  /// Ripples only visible in the host widget box.
  Bounded,
  /// Ripples only visible in the host widget box with a border radius.
  Radius(Radius),
}

impl<'c> ComposeChild<'c> for Ripple {
  type Child = Widget<'c>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    fn_widget! {
      let mut ripple_layer = PressedLayer::new(LayerArea::WidgetCover(Radius::all(0.)));
      init_ripple_launcher(&this, &mut ripple_layer);

      @ $ripple_layer {
        on_pointer_down: move |e| {
          let pos = (!$this.center).then(||e.position());
          $this.launch(pos);
        },
        on_disposed: move |_| $this.write().launcher = None,
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
    let ripple_grow = @Animate {
      state: LerpFnState::new(
        part_writer!(&mut layer.area),
        move |_, to, factor| {
          let LayerArea::Circle { center, radius, clip } = *to else { unreachable!() };
          LayerArea::Circle { center, radius: f32::lerp(&0., &radius, factor), clip }
        }
      ),
      transition: EasingTransition {
        easing: md::easing::EMPHASIZED_DECELERATE,
        duration: md::easing::duration::SHORT3,
      }.box_it(),
      from: LayerArea::WidgetCover(Radius::all(0.)),
    };

    let fade_out = @Animate {
      state: part_writer!(&mut layer.draw_opacity),
      transition: EasingTransition{
        easing: md::easing::STANDARD_ACCELERATE,
        duration: md::easing::duration::MEDIUM3
      }.box_it(),
      from: PressedLayer::show_opacity(),
    };

    watch!(!$ripple_grow.is_running() && !$layer.is_pointer_pressed())
      .skip(1)
      .distinct_until_changed()
      .filter(|fade| *fade)
      .subscribe(move |_| {
        $layer.write().hide();
        fade_out.run();
      });

    let launcher = move |pos: Option<Point>| {
      let size = $layer.layout_size();
      let center = pos.unwrap_or_else(|| {
        (size / 2.).to_vector().to_point()
      });
      let radius = $this.ripple_radius.unwrap_or_else(|| {
        let distance_x = f32::max(center.x , size.width - center.x);
        let distance_y = f32::max(center.y, size.height - center.y);
        (distance_x.powf(2.) + distance_y.powf(2.)).sqrt()
      });
      let clip = match $this.bounded {
        RippleBound::Unbounded => None,
        RippleBound::Bounded => Some(Radius::all(0.)),
        RippleBound::Radius(radius) => Some(radius),
      };
      {
        let mut layer = $layer.write();
        layer.area = LayerArea::Circle { center, radius, clip };
        layer.show();
      }
      ripple_grow.run()
    };

    $this.write().launcher = Some(Box::new(launcher));
  }
}
