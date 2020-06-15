use super::{painting_context::PaintingContext, render_tree::*};
use crate::{prelude::*, widget::widget_tree::*};
use canvas::{
  surface::{PhysicSurface, Surface, TextureSurface},
  Canvas, DeviceSize, WgpuRender,
};
use winit::{
  event::WindowEvent,
  event_loop::EventLoop,
  window::WindowId,
  window::{Window as NativeWindow, WindowBuilder},
};

/// Window is the root to represent.
pub struct Window<W = NativeWindow, S: Surface = PhysicSurface> {
  render_tree: RenderTree,
  widget_tree: WidgetTree,
  native_window: W,
  canvas: Canvas,
  render: WgpuRender<S>,
}

pub type HeadlessWindow = Window<(), TextureSurface>;

impl<W, S: Surface> Window<W, S> {
  /// processes native events from this native window
  pub(crate) fn processes_native_event(&mut self, _event: WindowEvent) {
    // todo: should process and dispatch event.
  }

  /// This method ensure render tree is ready to paint, three things it's have
  /// to do:
  /// 1. every need rebuild widgets has rebuild and correspond render tree
  /// construct.
  /// 2. every dirty widget has flush to render tree so render tree's data
  /// represent the latest application state.
  /// 3. every render objet need layout has done, so every render object is in
  /// the correct position.
  pub(crate) fn render_ready(&mut self) -> bool {
    self.tree_repair();
    self.mark_dirty();
    self.layout();
    // Todo: "should return if need repaint."
    true
  }

  /// Draw an image what current render tree represent.
  pub(crate) fn draw_frame(&mut self) {
    if let Some(ctx) = PaintingContext::new(&self.render_tree) {
      let layer = ctx.draw();
      let mut frame = self.canvas.next_frame(&mut self.render);
      frame.compose_2d_layer(layer);
    }
  }

  /// Repair the gaps between widget tree represent and current data state after
  /// some user or device inputs has been processed. The render tree will also
  /// react widget tree's change.
  #[inline]
  fn tree_repair(&mut self) { self.widget_tree.repair(&mut self.render_tree); }

  /// Layout the render tree as needed
  fn layout(&mut self) {
    let mut_ptr = &mut self.render_tree as *mut RenderTree;
    let root = self.render_tree.root().unwrap();
    let mut ctx = RenderCtx::new(&mut self.render_tree, &mut self.canvas);
    unsafe {
      root
        .get_mut(&mut *mut_ptr)
        .map(|node| node.perform_layout(root, &mut ctx));
    }
  }

  fn mark_dirty(&mut self) {
    let root = self.render_tree.root().unwrap();
    let mut ctx = RenderCtx::new(&mut self.render_tree, &mut self.canvas);
    ctx.mark_layout_dirty(root);
  }
}

impl Window {
  #[inline]
  pub fn id(&self) -> WindowId { self.native_window.id() }

  pub(crate) fn new<W: Into<Box<dyn Widget>>>(root: W, event_loop: &EventLoop<()>) -> Self {
    let native_window = WindowBuilder::new().build(event_loop).unwrap();
    let size = native_window.inner_size();
    let (canvas, render) = futures::executor::block_on(canvas::create_canvas_with_render_from_wnd(
      &native_window,
      DeviceSize::new(size.width, size.height),
    ));

    let mut wnd = Window {
      native_window,
      render_tree: Default::default(),
      widget_tree: Default::default(),
      canvas,
      render,
    };

    wnd.widget_tree.set_root(root.into(), &mut wnd.render_tree);

    wnd
  }

  /// Emits a `WindowEvent::RedrawRequested` event in the associated event loop
  /// after all OS events have been processed by the event loop.
  #[inline]
  pub(crate) fn request_redraw(&self) { self.native_window.request_redraw(); }
}

impl HeadlessWindow {
  pub fn headless<W: Into<Box<dyn Widget>>>(root: W, size: DeviceSize) -> Self {
    let (canvas, render) =
      futures::executor::block_on(canvas::create_canvas_with_render_headless(size));
    let mut wnd = HeadlessWindow {
      native_window: (),
      render_tree: Default::default(),
      widget_tree: Default::default(),
      canvas,
      render,
    };

    wnd.widget_tree.set_root(root.into(), &mut wnd.render_tree);

    wnd
  }
}
