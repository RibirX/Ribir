use crate::{prelude::*, render::render_tree::RenderTree, widget::widget_tree::WidgetTree};
mod focus_mgr;
pub(crate) use focus_mgr::FocusManager;
mod pointer;
pub(crate) use pointer::PointerDispatcher;
mod common;
pub(crate) use common::CommonDispatcher;
use std::{cell::RefCell, ptr::NonNull, rc::Rc};
pub use window::RawWindow;
use winit::event::WindowEvent;

pub(crate) struct Dispatcher {
  pub(crate) common: CommonDispatcher,
  pub(crate) pointer: PointerDispatcher,
  pub(crate) focus_mgr: FocusManager,
}

impl Dispatcher {
  pub fn new(
    render_tree: NonNull<RenderTree>,
    widget_tree: NonNull<WidgetTree>,
    window: Rc<RefCell<Box<dyn RawWindow>>>,
  ) -> Self {
    Self {
      common: CommonDispatcher::new(render_tree, widget_tree, window),
      pointer: PointerDispatcher::default(),
      focus_mgr: FocusManager::default(),
    }
  }

  pub fn dispatch(&mut self, event: WindowEvent) {
    log::info!("Dispatch winit event {:?}", event);
    match event {
      WindowEvent::ModifiersChanged(s) => self.common.modifiers_change(s),
      WindowEvent::CursorMoved { position, .. } => self.pointer.cursor_move_to(
        Point::new(position.x as f32, position.y as f32),
        &self.common,
      ),
      WindowEvent::CursorLeft { .. } => self.pointer.on_cursor_left(&self.common),
      WindowEvent::MouseInput {
        state,
        button,
        device_id,
        ..
      } => {
        self.pointer.dispatch_mouse_input(
          device_id,
          state,
          button,
          &self.common,
          &mut self.focus_mgr,
        );
      }
      _ => log::info!("not processed event {:?}", event),
    }
  }
}
