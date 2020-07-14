use super::{painting_context::PaintingContext, render_tree::*};
use crate::{
  prelude::*,
  widget::{events::dispatch::Dispatcher, widget_tree::*},
};
use canvas::{surface::TextureSurface, Canvas, CanvasRender, DeviceSize, WgpuRender};
use std::{pin::Pin, ptr::NonNull};
use winit::{
  event::WindowEvent,
  event_loop::EventLoop,
  window::WindowId,
  window::{Window as NativeWindow, WindowBuilder},
};

/// Window is the root to represent.
pub struct Window<W = NativeWindow, R: CanvasRender = WgpuRender> {
  render_tree: Pin<Box<RenderTree>>,
  widget_tree: Pin<Box<WidgetTree>>,
  native_window: W,
  canvas: Pin<Box<Canvas>>,
  render: R,
  dispatcher: Dispatcher,
}

pub type HeadlessWindow = Window<(), WgpuRender<TextureSurface>>;
pub type NoRenderWindow = Window<(), MockRender>;

pub struct MockRender;
impl CanvasRender for MockRender {
  fn draw(
    &mut self,
    _: &canvas::RenderData,
    _: &mut canvas::MemTexture<u8>,
    _: &mut canvas::MemTexture<u32>,
  ) {
  }
}

impl<W, R: CanvasRender> Window<W, R> {
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
  fn tree_repair(&mut self) {
    unsafe {
      self
        .widget_tree
        .as_mut()
        .get_unchecked_mut()
        .repair(self.render_tree.as_mut().get_unchecked_mut());
    }
  }

  /// Layout the render tree as needed
  fn layout(&mut self) {
    // let tree = unsafe { self.render_tree.as_mut().get_unchecked_mut() };
    // let mut_ptr = tree as *mut RenderTree;
    // let root = tree.root().unwrap();
    // let mut ctx = RenderCtx::new(tree, &mut self.canvas);
    // unsafe {
    //   root
    //     .get_mut(&mut *mut_ptr)
    //     .map(|node| node.perform_layout(root, &mut ctx));
    // }
  }

  fn mark_dirty(&mut self) {
    // let tree = unsafe { self.render_tree.as_mut().get_unchecked_mut() };
    // let root = tree.root().unwrap();
    // let mut ctx = RenderCtx::new(tree, &mut self.canvas);
    // ctx.mark_layout_dirty(root);
  }

  fn new(root: BoxWidget, wnd: W, canvas: Canvas, render: R) -> Self {
    let render_tree = Box::pin(RenderTree::default());

    let widget_tree = Box::pin(WidgetTree::default());
    let mut wnd = Self {
      native_window: wnd,
      dispatcher: Dispatcher::new(NonNull::from(&*render_tree), NonNull::from(&*widget_tree)),
      render_tree,
      widget_tree,
      canvas: Box::pin(canvas),
      render,
    };

    unsafe {
      wnd
        .widget_tree
        .as_mut()
        .get_unchecked_mut()
        .set_root(root.box_it(), wnd.render_tree.as_mut().get_unchecked_mut());
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

  pub(crate) fn from_event_loop(root: BoxWidget, event_loop: &EventLoop<()>) -> Self {
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
  pub fn headless(root: BoxWidget, size: DeviceSize) -> Self {
    let (canvas, render) =
      futures::executor::block_on(canvas::create_canvas_with_render_headless(size));
    Self::new(root, (), canvas, render)
  }

  #[inline]
  pub fn inner_position(&self) -> DevicePoint { DevicePoint::new(20, 20) }

  #[inline]
  pub fn outer_position(&self) -> DevicePoint { DevicePoint::new(0, 0) }
}

impl NoRenderWindow {
  pub fn without_render(root: BoxWidget, size: DeviceSize) -> Self {
    let canvas = Canvas::new(size);
    let render = MockRender;
    Self::new(root, (), canvas, render)
  }

  #[inline]
  pub fn inner_position(&self) -> DevicePoint { DevicePoint::new(20, 20) }

  #[inline]
  pub fn outer_position(&self) -> DevicePoint { DevicePoint::new(0, 0) }

  #[cfg(test)]
  pub fn render_tree(&mut self) -> Pin<&mut RenderTree> { self.render_tree.as_mut() }

  #[cfg(test)]
  pub fn widget_tree(&mut self) -> Pin<&mut WidgetTree> { self.widget_tree.as_mut() }

  #[cfg(test)]
  pub fn canvas(&mut self) -> Pin<&mut Canvas> { self.canvas.as_mut() }
}
