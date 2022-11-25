use super::state_layer::StateRole;
use crate::prelude::*;
use ribir_core::prelude::*;

/// Widget use to do ripple animate as a visual feedback to user interactive.
/// Usually for touch and mouse.
#[derive(Declare, Debug)]
pub struct Ripple {
  /// The color of ripples.
  pub color: Color,
  /// The radius in pixels of foreground ripples when fully expanded. The
  /// default radius will be the distance from the center of the ripple to the
  /// furthest corner of the host bounding rectangle.
  #[declare(default, convert=strip_option)]
  pub radius: Option<f32>,
  /// Whether the ripple always originates from the center of the host bound.
  #[declare(default)]
  pub center: bool,
  #[declare(default=RippleBound::Bounded)]
  /// How ripples show outside of the host widget box.
  pub bounded: RippleBound,
  /// The position of current animate launch start.
  #[declare(skip)]
  launch_pos: Option<Point>,
}

/// Config how ripples show outside of the host widget box.
#[derive(Debug, PartialEq)]
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

  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget {
    widget! {
      track { this: this.into_stateful() }
      Stack {
        id: container,
        DynWidget { dyns: child }
        DynWidget {
          dyns: {
            this.launch_pos.clone().map(|launch_at| {
              let radius = this.radius.unwrap_or_else(|| {
                let rect = container.layout_rect();
                let distance_x = f32::max( launch_at.x - rect.min_x(), rect.max_x() - launch_at.x);
                let distance_y = f32::max(launch_at.y - rect.min_y(), rect.max_y() - launch_at.y);
                (distance_x.powf(2.) + distance_y.powf(2.)).sqrt()
              });
              widget!{
                DynWidget {
                  dyns: (this.bounded != RippleBound::Unbounded).then(|| {
                    let rect = container.layout_rect();
                    let path = match this.bounded {
                      RippleBound::Unbounded => unreachable!(),
                      RippleBound::Bounded => Path::rect(&rect, PathStyle::Fill),
                      RippleBound::Radius(radius) => {
                        Path::rect_round(&rect, &radius, PathStyle::Fill)
                      }
                    };
                    Clip { clip: ClipType::Path(path) }
                  }),
                  PathPaintKit {
                    id: ripple_path,
                    brush: StateRole::pressed().calc_color(this.color),
                    path: Path::circle(launch_at, radius, PathStyle::Fill),
                  }
                }
                Animate {
                  id: ripper_enter,
                  from: State {
                    ripple_path.path: Path::circle(Point::zero(), 0., PathStyle::Fill)
                  },
                  transition: transitions::LINEAR.of(ctx),
                  lerp_fn: move |_, _, rate| {
                    let radius = Lerp::lerp(&0., &radius, rate);
                    let center = this.launch_pos.clone().unwrap();
                    Path::circle(center, radius, PathStyle::Fill)
                  }
                }
                on container.pointer_pressed() || ripper_enter.is_running() {
                  change: move |(before, after)| if (before, after) == (true, false) {
                    this.launch_pos.take();
                  }
                }
                // todo: support disposed animate
                // Animate {
                //   id: ripper_fade_out,
                //   from: State { ripple_path.opacity: 1.},
                //   transition: transitions::EASE_OUT.get_from_or_default(ctx.theme()),
                // }
                on ripple_path {
                  mounted: move |_| { ripper_enter.run(); }
                }
              }
            })
          }
        }
      }
      on container {
        pointer_down: move |e| this.launch_pos = if this.center {
          Some(container.layout_rect().center())
        } else {
          Some(e.position())
        }
      }
    }
  }
}

impl Ripple {
  /// Manual launch a ripple animate at `pos`.
  pub fn launch_at(&mut self, pos: Point) { self.launch_pos = Some(pos); }
}
