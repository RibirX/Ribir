#![feature(
  decl_macro,
  test,
  const_fn,
  slice_fill,
  const_fn_floating_point_arithmetic
)]
mod atlas;
mod canvas;
pub mod color;
pub mod error;
pub mod layer;
mod mem_texture;
mod text_brush;
pub mod wgpu_render;

pub use crate::canvas::*;
pub use color::Color;
pub use layer::*;
pub use mem_texture::MemTexture;
pub use text_brush::*;
pub use wgpu_render::*;

/// The tag for device unit system to prevent mixing values from different
/// system.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PhysicUnit;

/// The tag for logic unit system to prevent mixing values from different
/// system.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LogicUnit;

pub type Rect = euclid::Rect<f32, LogicUnit>;
pub type Point = euclid::Point2D<f32, LogicUnit>;
pub type Size = euclid::Size2D<f32, LogicUnit>;
pub type Transform = euclid::Transform2D<f32, LogicUnit, LogicUnit>;
pub type Vector = euclid::Vector2D<f32, LogicUnit>;
pub type Angle = euclid::Angle<f32>;

pub type DeviceRect = euclid::Rect<u32, PhysicUnit>;
pub type DevicePoint = euclid::Point2D<u32, PhysicUnit>;
pub type DeviceSize = euclid::Size2D<u32, PhysicUnit>;

pub async fn create_canvas_with_render_from_wnd<W: raw_window_handle::HasRawWindowHandle>(
  window: &W,
  size: DeviceSize,
) -> (Canvas, WgpuRender) {
  let mut canvas = Canvas::new(None);
  let render = WgpuRender::wnd_render(
    window,
    size,
    canvas.text_brush().texture().size(),
    canvas.atlas().texture().size(),
    AntiAliasing::MSAA4X,
  )
  .await;

  (canvas, render)
}

pub async fn create_canvas_with_render_headless(
  size: DeviceSize,
) -> (Canvas, WgpuRender<surface::TextureSurface>) {
  let mut canvas = Canvas::new(None);
  let render = WgpuRender::headless_render(
    size,
    canvas.text_brush().texture().size(),
    canvas.atlas().texture().size(),
  )
  .await;

  (canvas, render)
}
