use super::{painting_context::PaintingContext, render_tree::*};
use crate::{prelude::*, widget::widget_tree::*};
use canvas::{Canvas, DeviceSize};
use winit::{
  event::WindowEvent,
  event_loop::EventLoop,
  window::WindowId,
  window::{Window as NativeWindow, WindowBuilder},
};

/// Window is the root to represent.
pub struct Window<'a> {
  render_tree: RenderTree,
  widget_tree: WidgetTree<'a>,
  native_window: NativeWindow,
  canvas: Canvas,
}

impl<'a> Window<'a> {
  #[inline]
  pub fn id(&self) -> WindowId { self.native_window.id() }

  pub(crate) fn new<W: Into<Box<dyn Widget + 'a>>>(
    root: W,
    event_loop: &EventLoop<()>,
  ) -> Self {
    let native_window = WindowBuilder::new().build(event_loop).unwrap();
    let size = native_window.inner_size();
    let canvas = Canvas::from_window(
      &native_window,
      DeviceSize::new(size.width, size.height),
    );

    let canvas = futures::executor::block_on(canvas);
    let mut wnd = Window {
      native_window,
      render_tree: Default::default(),
      widget_tree: Default::default(),
      canvas,
    };

    wnd.widget_tree.set_root(root.into(), &mut wnd.render_tree);

    wnd
  }

  /// processes native events from this native window
  pub(crate) fn processes_native_event(&mut self, event: WindowEvent) {
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
    self.layout();
    // Todo: "should return if need repaint."
    true
  }

  /// Draw an image what current render tree represent.
  pub(crate) fn draw_frame(&mut self) {
    if let Some(root) = self.render_tree.root() {
      let painting_context =
        PaintingContext::new(&mut self.canvas, root, &self.render_tree);
      root
        .get(&self.render_tree)
        .expect("Root render object should exists when root id exists in tree.")
        .paint(painting_context);
    }
  }

  /// Emits a `WindowEvent::RedrawRequested` event in the associated event loop
  /// after all OS events have been processed by the event loop.
  #[inline]
  pub(crate) fn request_redraw(&self) { self.native_window.request_redraw(); }

  /// Repair the gaps between widget tree represent and current data state after
  /// some user or device inputs has been processed. The render tree will also
  /// react widget tree's change.
  #[inline]
  fn tree_repair(&mut self) { self.widget_tree.repair(&mut self.render_tree); }

  /// Layout the render tree as needed
  fn layout(&mut self) {
    // todo: layout the tree from window to leaf.
  }
}
