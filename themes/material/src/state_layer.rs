use ribir_core::{prelude::*, wrap_render::WrapRender};

use crate::md;

const HOVER_OPACITY: u8 = 8;
const PRESSED_OPACITY: u8 = 10;
const FOCUS_OPACITY: u8 = 10;

pub type HoverLayer = StateLayer<HOVER_OPACITY>;
pub type PressedLayer = StateLayer<PRESSED_OPACITY>;
pub type FocusLayer = StateLayer<FOCUS_OPACITY>;

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
    let u = watch!($layer.is_hovered())
      // Delay hover effects to prevent displaying this layer while scrolling.
      .delay(Duration::from_millis(50), AppCtx::scheduler())
      .subscribe(move |_| {
        layer2
          .write()
          .set_visible_state(hover.read().is_hovered());
      });
    layer.on_disposed(move |_| u.unsubscribe());
    layer
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
    WrapRender::combine_child(this, child, DirtyPhase::Paint)
  }
}

impl<const M: u8> WrapRender for StateLayer<M> {
  fn paint(&self, host: &dyn Render, ctx: &mut PaintingCtx) {
    let StateLayer { area, draw_opacity } = self;
    if *draw_opacity <= 0. {
      return host.paint(ctx);
    }

    // Fork a painter to create an overlay without affecting the main painter's
    // state
    let mut layer = ctx.painter().fork();
    host.paint(ctx);

    let rect = Rect::from_size(ctx.box_size().unwrap());
    layer.apply_alpha(*draw_opacity);
    match area {
      LayerArea::Circle { center, radius, clip } => {
        if let Some(clip) = clip {
          layer.clip(Path::rect_round(&rect, clip).into());
        }
        layer.circle(*center, *radius).fill()
      }
      LayerArea::WidgetCover(radius) => layer.rect_round(&rect, radius).fill(),
    };
    ctx.painter().merge(&mut layer);
  }

  fn visual_box(&self, host: &dyn Render, ctx: &mut VisualCtx) -> Option<Rect> {
    if let LayerArea::Circle { radius, .. } = self.area {
      if self.draw_opacity > 0. {
        let rect = Rect::from_size(Size::splat(radius * 2.));
        let union = host
          .visual_box(ctx)
          .map_or(rect, |v| v.union(&rect));
        return Some(union);
      }
    }

    host.visual_box(ctx)
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
