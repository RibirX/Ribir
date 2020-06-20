use super::{painting_context::PaintingContext, render_tree::*};
use crate::{events::dispatch::Dispatcher, prelude::*, widget::widget_tree::*};
use canvas::{
  surface::{PhysicSurface, Surface, TextureSurface},
  Canvas, DeviceSize, WgpuRender,
};
use std::{cell::RefCell, rc::Rc};
use winit::{
  event::WindowEvent,
  event_loop::EventLoop,
  window::WindowId,
  window::{Window as NativeWindow, WindowBuilder},
};

/// Window is the root to represent.
pub struct Window<W = NativeWindow, S: Surface = PhysicSurface> {
  render_tree: Rc<RefCell<RenderTree>>,
  widget_tree: Rc<RefCell<WidgetTree>>,
  native_window: W,
  canvas: Canvas,
  render: WgpuRender<S>,
  dispatcher: Dispatcher,
}

pub type HeadlessWindow = Window<(), TextureSurface>;

impl<W, S: Surface> Window<W, S> {
  /// processes native events from this native window
  #[inline]
  pub(crate) fn processes_native_event(&mut self, event: WindowEvent) {
    self.dispatcher.dispatch(event);
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
    let render_tree = self.render_tree.borrow();
    if let Some(ctx) = PaintingContext::new(&render_tree) {
      let layer = ctx.draw();
      let mut frame = self.canvas.next_frame(&mut self.render);
      frame.compose_2d_layer(layer);
    }
  }

  /// Repair the gaps between widget tree represent and current data state after
  /// some user or device inputs has been processed. The render tree will also
  /// react widget tree's change.
  fn tree_repair(&mut self) {
    let mut render_tree = self.render_tree.borrow_mut();
    self.widget_tree.borrow_mut().repair(&mut render_tree);
  }

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

  fn new<R: Into<Box<dyn Widget>>>(root: R, wnd: W, canvas: Canvas, render: WgpuRender<S>) -> Self {
    let render_tree: Rc<RefCell<RenderTree>> = <_>::default();
    let widget_tree: Rc<RefCell<WidgetTree>> = <_>::default();
    let wnd = Self {
      native_window: wnd,
      dispatcher: Dispatcher::new(render_tree.clone(), widget_tree.clone()),
      render_tree,
      widget_tree,
      canvas,
      render,
    };

    {
      let mut render_tree = wnd.render_tree.borrow_mut();
      wnd
        .widget_tree
        .borrow_mut()
        .set_root(root.into(), &mut render_tree);
    }

    wnd
  }
}

impl Window {
  #[inline]
  pub fn id(&self) -> WindowId { self.native_window.id() }

  /// Returns the position of the top-left hand corner of the window's client
  /// area relative to the top-left hand corner of the desktop.
  pub fn inner_position(&self) -> DevicePoint {
    let pos = self
      .native_window
      .inner_position()
      .expect(" Can only be called on the main thread");
    DevicePoint::new(pos.x as u32, pos.y as u32)
  }

  /// Returns the position of the top-left hand corner of the window relative to
  /// the  top-left hand corner of the desktop.
  pub fn outer_position(&self) -> DevicePoint {
    let pos = self
      .native_window
      .outer_position()
      .expect(" Can only be called on the main thread");
    DevicePoint::new(pos.x as u32, pos.y as u32)
  }

  pub(crate) fn from_event_loop<W: Into<Box<dyn Widget>>>(
    root: W,
    event_loop: &EventLoop<()>,
  ) -> Self {
    let native_window = WindowBuilder::new().build(event_loop).unwrap();
    let size = native_window.inner_size();
    let (canvas, render) = futures::executor::block_on(canvas::create_canvas_with_render_from_wnd(
      &native_window,
      DeviceSize::new(size.width, size.height),
    ));

    Self::new(root, native_window, canvas, render)
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
    Self::new(root, (), canvas, render)
  }

  /// Returns the position of the top-left hand corner of the window's client
  /// area relative to the top-left hand corner of the desktop.
  #[inline]
  pub fn inner_position(&self) -> DevicePoint { DevicePoint::new(20, 20) }

  /// Returns the position of the top-left hand corner of the window relative to
  /// the  top-left hand corner of the desktop.
  #[inline]
  pub fn outer_position(&self) -> DevicePoint { DevicePoint::new(0, 0) }
}
