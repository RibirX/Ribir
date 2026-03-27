#[cfg(feature = "debug")]
use ribir_core::prelude::{BoxFuture, ColorFormat, PixelImage};
use ribir_core::prelude::{
  Color, DeviceRect, DeviceSize, GlyphRasterSource, PaintCommand, PainterBackend, Transform,
};
use ribir_gpu::Surface;
#[cfg(feature = "debug")]
use ribir_gpu::{GPUBackendImpl, Texture, WgpuTexture};

use crate::winit_shell_wnd::WinitBackend;

pub struct WgpuBackend<'a> {
  surface: Surface<'a>,
  backend: ribir_gpu::GPUBackend<ribir_gpu::WgpuImpl>,
  #[cfg(feature = "debug")]
  offscreen_texture: Option<WgpuTexture>,
}

impl<'a> WinitBackend<'a> for WgpuBackend<'a> {
  async fn new(window: &'a winit::window::Window) -> WgpuBackend<'a> {
    let (wgpu, surface) = ribir_gpu::WgpuImpl::new(window).await;
    let size = window.inner_size();
    let size = DeviceSize::new(size.width as i32, size.height as i32);

    let mut wgpu = WgpuBackend {
      surface,
      backend: ribir_gpu::GPUBackend::new(wgpu),
      #[cfg(feature = "debug")]
      offscreen_texture: None,
    };
    wgpu.on_resize(size);

    wgpu
  }

  fn on_resize(&mut self, size: DeviceSize) {
    if size != self.surface.size() {
      self.surface.resize(size, self.backend.get_impl());
      #[cfg(feature = "debug")]
      {
        self.offscreen_texture = None;
      }
    }
  }

  fn begin_frame(&mut self, surface_color: Color) { self.backend.begin_frame(surface_color); }

  fn draw_commands(
    &mut self, viewport: DeviceRect, global_matrix: &Transform, commands: &[PaintCommand],
    glyph_provider: &dyn GlyphRasterSource,
  ) {
    let Some(texture) = self
      .surface
      .get_current_texture(self.backend.get_impl())
    else {
      return;
    };
    self
      .backend
      .draw_commands(viewport, commands, global_matrix, texture, glyph_provider);
  }

  fn end_frame(&mut self) {
    self.backend.end_frame();
    self.surface.present();
  }

  #[cfg(feature = "debug")]
  fn capture_screenshot(
    &mut self, viewport: DeviceRect, global_matrix: &Transform, commands: &[PaintCommand],
    glyph_provider: &dyn GlyphRasterSource,
  ) -> Option<BoxFuture<'static, Option<PixelImage>>> {
    let (surface, backend, cached_offscreen_texture) =
      (&mut self.surface, &mut self.backend, &mut self.offscreen_texture);
    let size = surface.size();
    if size.is_empty() {
      return None;
    }

    let texture = match surface.get_current_texture(backend.get_impl()) {
      Some(texture) => texture,
      None => {
        let texture = ensure_offscreen_texture(size, backend, cached_offscreen_texture);
        backend.draw_commands(viewport, commands, global_matrix, texture, glyph_provider);
        texture
      }
    };
    let rect = DeviceRect::from_size(size);

    let img_future =
      <WgpuTexture as Texture>::copy_as_image(texture, &rect, backend.get_impl_mut());

    Some(Box::pin(async move { img_future.await.ok() }))
  }
}

#[cfg(feature = "debug")]
fn ensure_offscreen_texture<'a>(
  size: DeviceSize, backend: &mut ribir_gpu::GPUBackend<ribir_gpu::WgpuImpl>,
  offscreen_texture: &'a mut Option<WgpuTexture>,
) -> &'a mut WgpuTexture {
  let needs_resize = offscreen_texture
    .as_ref()
    .is_none_or(|texture| texture.size() != size);
  if needs_resize {
    *offscreen_texture = Some(
      backend
        .get_impl_mut()
        .new_texture(size, ColorFormat::Rgba8),
    );
  }

  offscreen_texture.as_mut().unwrap()
}
