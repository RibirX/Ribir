use ribir_core::{prelude::*, wrap_render::WrapRender};
use ribir_widgets::{layout::Stack, path::PathPaintKit, prelude::StackFit};

use crate::md;

const HOVER_OPACITY: u8 = 8;
const PRESSED_OPACITY: u8 = 10;

pub type HoverLayer = StateLayer<HOVER_OPACITY>;
pub type PressedLayer = StateLayer<PRESSED_OPACITY>;

/// Widget that as an visual indicator of material design used to present the
/// interactive status of its child.
#[derive(Declare)]
pub(crate) struct StateLayerOld {
  pub color: Color,
  pub path: Path,
  pub role: StateRole,
}
/// Widget that as visual indicator of material design used to communicate the
/// status of interactive widget, its visual state will reactive to its child
/// interactive state.
#[derive(Declare)]
pub(crate) struct InteractiveLayer {
  /// the color of the state layer, will apply a fixed opacity in different
  /// state.
  pub color: Color,
  /// The border radii
  pub border_radii: Radius,
}

impl Compose for StateLayerOld {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    fn_widget! {
      @PathPaintKit {
        path: pipe!($this.path.clone()),
        foreground: pipe!($this.role.calc_color($this.color)),
      }
    }
    .into_widget()
  }
}

impl<'c> ComposeChild<'c> for InteractiveLayer {
  type Child = Widget<'c>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    fn_widget! {
      let mut host = FatObj::new(child);
      let layer = @IgnorePointer {
        @Container {
          size: pipe!($host.layout_size()),
          @StateLayerOld {
            color: pipe!($this.color),
            path: pipe!(Path::rect_round(&$host.layout_rect(), &$this.border_radii)),
            role: pipe!(if $host.is_pointer_pressed() {
              StateRole::pressed()
            } else if $host.has_focus() {
              StateRole::focus()
            } else if $host.is_hover() {
              StateRole::hover()
            } else {
              // todo: not support drag & drop now
              StateRole::custom(0.)
            })
          }
        }
      };

      @Stack {
        fit: StackFit::Passthrough,
        @{ host }
        @{ layer }
      }
    }
    .into_widget()
  }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct StateRole(f32);

impl StateRole {
  pub const fn hover() -> Self { Self(0.08) }

  pub const fn focus() -> Self { Self(0.12) }

  pub const fn pressed() -> Self { Self(0.12) }

  pub const fn dragged() -> Self { Self(0.16) }

  pub const fn custom(opacity: f32) -> Self { Self(opacity) }

  #[inline]
  pub fn calc_color(self, color: Color) -> Color { color.with_alpha(self.0) }
}

#[derive(Debug, Clone)]
pub struct StateLayer<const M: u8> {
  pub area: LayerArea,
  pub draw_opacity: f32,
}

impl PressedLayer {
  /// Create a pressed state layer in a hidden state. This layer is only a
  /// visual effect and not track the interactive state to control to show or
  /// hide.
  pub fn new(path: impl Into<LayerArea>) -> FatObj<Stateful<PressedLayer>> {
    FatObj::new(Stateful::new(Self { area: path.into(), draw_opacity: 0. }))
  }
}

impl HoverLayer {
  /// Create a hover state layer displaying only when the pointer is hovering
  /// this widget.
  pub fn tracked(path: impl Into<LayerArea>) -> FatObj<Stateful<HoverLayer>> {
    let layer = Stateful::new(Self { area: path.into(), draw_opacity: 0. });
    part_writer!(&mut layer.draw_opacity).transition(EasingTransition {
      easing: md::easing::STANDARD,
      duration: md::easing::duration::SHORT1,
    });
    let mut layer = FatObj::new(layer);
    let layer2 = layer.clone_writer();

    let hover = layer.get_mix_flags_widget().clone_reader();
    let u = watch!($layer.is_hover())
      // Delay hover effects to prevent displaying this layer while scrolling.
      .delay(Duration::from_millis(100), AppCtx::scheduler())
      .subscribe(move |_| {
        layer2
          .write()
          .set_visible_state(hover.read().is_hover());
      });
    layer.on_disposed(move |_| u.unsubscribe())
  }
}

impl<const M: u8> StateLayer<M> {
  pub fn show(&mut self) { self.set_visible_state(true); }

  pub fn hide(&mut self) { self.set_visible_state(false); }

  pub fn show_opacity() -> f32 { M as f32 / 100. }

  fn set_visible_state(&mut self, visible: bool) {
    if visible {
      self.draw_opacity = Self::show_opacity();
    } else {
      self.draw_opacity = 0.;
    }
  }
}

impl<'c, const M: u8> ComposeChild<'c> for StateLayer<M> {
  type Child = Widget<'c>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    WrapRender::combine_child(this, child)
  }
}

impl<const M: u8> WrapRender for StateLayer<M> {
  fn perform_layout(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx) -> Size {
    host.perform_layout(clamp, ctx)
  }

  fn paint(&self, host: &dyn Render, ctx: &mut PaintingCtx) {
    host.paint(ctx);
    if self.draw_opacity > 0. {
      let opacity = ctx.painter().alpha();
      ctx.painter().apply_alpha(self.draw_opacity);
      self.area.draw(ctx);
      ctx.painter().set_alpha(opacity);
    }
  }
}

/// The path of a state layer to fill can either be a radius that fills the
/// widget box with a radius or a specific path.
///
/// This path will not affect the widget layout; it is purely for visual effect.
#[derive(Debug, Clone, PartialEq)]
pub enum LayerArea {
  Circle { center: Point, radius: f32, clip: Option<Radius> },
  WidgetCover(Radius),
}

impl LayerArea {
  fn draw(&self, ctx: &mut PaintingCtx) {
    let size = ctx.box_size().unwrap();
    let rect = Rect::from_size(size);
    let painter = ctx.painter();
    match self {
      LayerArea::Circle { center, radius, clip } => {
        if let Some(clip) = clip {
          painter.clip(Path::rect_round(&rect, clip).into());
        }
        painter.circle(*center, *radius).fill()
      }
      LayerArea::WidgetCover(radius) => painter.rect_round(&rect, radius).fill(),
    };
  }
}
