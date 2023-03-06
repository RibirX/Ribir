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

  fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
    widget! {
      states { this: this.into_writable() }
      init ctx => {
        let linear_transition = transitions::LINEAR.of(ctx);
      }
      Stack {
        id: container,
        on_pointer_down: move |e| this.launch_pos = if this.center {
          let center = container.layout_size() / 2.;
          Some(Point::new(center.width, center.height))
        } else {
          Some(e.position())
        },
        identify(child)
        Option::map(this.launch_pos, |launch_at| {
          let radius = this.radius.unwrap_or_else(|| {
            let size = container.layout_size();
            let distance_x = f32::max(launch_at.x , size.width - launch_at.x);
            let distance_y = f32::max(launch_at.y, size.height - launch_at.y);
            (distance_x.powf(2.) + distance_y.powf(2.)).sqrt()
          });
          let linear_transition = linear_transition.clone();
          widget!{
            IgnorePointer {
              DynWidget {
                dyns: (this.bounded != RippleBound::Unbounded).then(|| {
                  let rect = Rect::from_size(container.layout_size());
                  let path = match this.bounded {
                    RippleBound::Unbounded => unreachable!(),
                    RippleBound::Bounded => Path::rect(&rect, PathStyle::Fill),
                    RippleBound::Radius(radius) => {
                      Path::rect_round(&rect, &radius, PathStyle::Fill)
                    }
                  };
                  Clip { clip: ClipType::Path(path) }
                }),
                Container {
                  size: container.layout_size(),
                  PathPaintKit {
                    id: ripple_path,
                    brush: StateRole::pressed().calc_color(this.color),
                    path: Path::circle(launch_at, radius, PathStyle::Fill),
                    on_mounted: move |_| { ripper_enter.run(); }
                  }
                }
              }
            }
            Animate {
              id: ripper_enter,
              transition: linear_transition,
              prop: prop!(ripple_path.path, move |_, _, rate| {
                let radius = Lerp::lerp(&0., &radius, rate);
                let center = this.launch_pos.clone().unwrap();
                Path::circle(center, radius, PathStyle::Fill)
              }),
              from: Path::circle(Point::zero(), 0., PathStyle::Fill)
            }
            finally {
              let_watch!(!container.pointer_pressed() && !ripper_enter.is_running())
                .filter(|b| *b)
                .subscribe(move |_| {
                  this.launch_pos.take();
                });
            }
            // todo: support disposed animate
            // Animate {
            //   id: ripper_fade_out,
            //   from: State { ripple_path.opacity: 1.},
            //   transition: transitions::EASE_OUT.get_from_or_default(ctx.theme()),
            // }
          }
        })
      }
    }
  }
}

impl Ripple {
  /// Manual launch a ripple animate at `pos`.
  pub fn launch_at(&mut self, pos: Point) { self.launch_pos = Some(pos); }
}
