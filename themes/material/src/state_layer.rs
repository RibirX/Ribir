use ribir_core::{prelude::*, rxrust::ops::throttle::ThrottleEdge, wrap_render::WrapRender};

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
  pub fn new(path: LayerArea) -> FatObj<Stateful<PressedLayer>> {
    FatObj::new(Stateful::new(Self { area: path, draw_opacity: 0. }))
  }
}

impl HoverLayer {
  /// Create a hover layer displaying only when the pointer is hovering
  /// this widget.
  pub fn created_for(area: LayerArea, host: &mut FatObj<Widget>) -> Stateful<HoverLayer> {
    let layer = Stateful::new(Self { area, draw_opacity: 0. });
    part_writer!(&mut layer.draw_opacity).transition(EasingTransition {
      easing: md::easing::STANDARD,
      duration: md::easing::duration::SHORT2,
    });

    let u = watch!(*$read(host.is_hovered()))
      // Delay hover effects to prevent displaying this layer while scrolling.
      .throttle_time(Duration::from_millis(100), ThrottleEdge::tailing(), AppCtx::scheduler())
      .distinct_until_changed()
      .subscribe({
        let layer = layer.clone_writer();
        move |visible| layer.write().set_visible_state(visible)
      });
    host.on_disposed(move |_| u.unsubscribe());

    layer
  }
}

impl FocusLayer {
  /// Create a focused layer displaying only when the widget is focused.
  pub fn create_for(host: &mut FatObj<Widget>) -> Stateful<FocusLayer> {
    let layer = Stateful::new(Self { area: LayerArea::FullContent, draw_opacity: 0. });
    part_writer!(&mut layer.draw_opacity).transition(EasingTransition {
      easing: md::easing::STANDARD,
      duration: md::easing::duration::SHORT2,
    });

    let u = watch! {
      *$read(host.is_focused())
      && *$read(host.focus_changed_reason()) == FocusReason::Keyboard
    }
    .distinct_until_changed()
    .subscribe({
      let layer = layer.clone_writer();
      move |v| layer.write().set_visible_state(v)
    });
    host.on_disposed(move |_| u.unsubscribe());

    layer
  }
}

impl PressedLayer {
  /// Create a pressed state layer in a hidden state. This layer is only a
  /// visual effect and not track the interactive state to control to show or
  /// hide.
  pub fn pressed(area: LayerArea) -> FatObj<Stateful<PressedLayer>> {
    FatObj::new(Stateful::new(StateLayer { area, draw_opacity: 0. }))
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
  fn paint(&self, host: &dyn Render, ctx: &mut PaintingCtx) {
    let StateLayer { area, draw_opacity } = self;
    if *draw_opacity <= 0. {
      return host.paint(ctx);
    }

    // Fork a painter to create an overlay without affecting the main painter's
    // state
    let mut layer = ctx.painter().fork();
    host.paint(ctx);

    layer.apply_alpha(*draw_opacity);
    match area {
      LayerArea::Circle { center, radius, constrain_to_bounds } => {
        if *constrain_to_bounds {
          layer.clip(widget_boundary(ctx).into());
        }
        layer.circle(*center, *radius).fill()
      }
      LayerArea::FullContent => layer.fill_path(widget_boundary(ctx).into()),
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

  #[inline]
  fn wrapper_dirty_phase(&self) -> DirtyPhase { DirtyPhase::Paint }
}

/// Defines a visual layer region that doesn't participate in layout
/// calculations.
///
/// These areas are used exclusively for post-layout visual effects.
#[derive(Debug, Clone, PartialEq)]
pub enum LayerArea {
  /// Circular clipping/masking region with optional boundary constraint
  ///
  /// - `center`: Local coordinates relative to widget origin
  /// - `radius`: Radius in logical pixels
  /// - `constrain_to_bounds`: When true, automatically clamps the circle to
  ///   stay within the widget's content rectangle
  Circle { center: Point, radius: f32, constrain_to_bounds: bool },

  /// Full coverage of widget's layout frame including padding
  ///
  /// Uses the final calculated layout rect after padding has been applied,
  /// matching the widget's visible content area.
  FullContent,
}

fn widget_boundary(ctx: &PaintingCtx) -> Path {
  let rect = Rect::from_size(ctx.box_size().unwrap());
  let widget_radius = Provider::of::<Radius>(ctx);
  if let Some(radius) = widget_radius {
    Path::rect_round(&rect, &radius)
  } else {
    Path::rect(&rect)
  }
}
