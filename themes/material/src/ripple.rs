use ribir_core::prelude::*;
use ribir_widgets::prelude::*;

use super::state_layer::StateRole;

/// Widget use to do ripple animate as a visual feedback to user interactive.
/// Usually for touch and mouse.
#[derive(Debug, Declare)]
pub struct Ripple {
  /// The color of ripples.
  pub color: Color,
  /// The radius in pixels of foreground ripples when fully expanded. The
  /// default radius will be the distance from the center of the ripple to the
  /// furthest corner of the host bounding rectangle.
  #[declare(default)]
  pub radius: Option<f32>,
  /// Whether the ripple always originates from the center of the host bound.
  #[declare(default)]
  pub center: bool,
  #[declare(default=RippleBound::Bounded)]
  /// How ripples show outside of the host widget box.
  pub bounded: RippleBound,
  /// The position of current animate launch start.
  #[declare(default = Stateful::new(None))]
  ripple_at: Stateful<Option<Point>>,
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

impl ComposeChild for Ripple {
  type Child = Widget;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> impl WidgetBuilder {
    fn_widget! {
      let mut container = @Stack { fit: StackFit::Passthrough };
      let ripple_at = $this.ripple_at.clone_writer();

      let ripple_widget = pipe!(*$ripple_at)
        .map(move |launch_at| {
          let launch_at = launch_at?;
          let radius = $this.radius.unwrap_or_else(|| {
            let size = $container.layout_size();
            let distance_x = f32::max(launch_at.x , size.width - launch_at.x);
            let distance_y = f32::max(launch_at.y, size.height - launch_at.y);
            (distance_x.powf(2.) + distance_y.powf(2.)).sqrt()
          });

          let mut ripple = @PathPaintKit {
            brush: pipe!(StateRole::pressed().calc_color($this.color)),
            path: Path::circle(launch_at, radius),
          };

          let ripper_enter = @Animate {
            transition: transitions::LINEAR.of(ctx!()),
            state: LerpFnState::new(
              ripple.map_writer(|w| PartData::from_ref_mut(&mut w.path)),
              move |_, _, rate| {
                let radius = Lerp::lerp(&0., &radius, rate);
                Path::circle(launch_at, radius)
              }
            ),
            from: Path::circle(Point::zero(), 0.)
          };

          watch!(!$container.pointer_pressed() && !$ripper_enter.is_running())
            .filter(|b| *b)
            // the ripple used only once, so we unsubscribe it after the animate finished.
            .take(1)
            .subscribe(move |_| {
              $ripple_at.write().take();
            });

          let ripper_fade_out = ripple
            .get_opacity_widget()
            .map_writer(|w| PartData::from_ref_mut(&mut w.opacity))
            .transition(transitions::EASE_OUT.of(ctx!()), ctx!());

          let bounded = $this.bounded;
          let clipper = (bounded != RippleBound::Unbounded).then(|| {
            let rect = Rect::from_size($container.layout_size());
            let path = match bounded {
              RippleBound::Unbounded => unreachable!(),
              RippleBound::Bounded => Path::rect(&rect),
              RippleBound::Radius(radius) => Path::rect_round(&rect, &radius)
            };
            @Clip { clip: ClipType::Path(path) }
          });

          Some(@IgnorePointer {
            keep_alive: pipe!($ripper_fade_out.is_running()),
            on_disposed: move |_| $ripple.write().opacity = 0.,
            on_mounted: move |_| { ripper_enter.run(); },
            @Container {
              size: $container.layout_size(),
              @$clipper { @ { ripple } }
            }
          })
      });

      @ $container {
        on_pointer_down: move |e| *$ripple_at.write() = if $this.center {
          let center = $container.layout_size() / 2.;
          Some(Point::new(center.width, center.height))
        } else {
          Some(e.position())
        },
        @{ child }
        @{ ripple_widget }
      }
    }
  }
}

impl Ripple {
  /// Manual launch a ripple animate at `pos`.
  pub fn launch_at(&mut self, pos: Point) { *self.ripple_at.write() = Some(pos); }
}
