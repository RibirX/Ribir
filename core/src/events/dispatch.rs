use super::pointers::{PointerEvent, PointerListener};
use crate::{
  prelude::{Point, Size},
  render::render_tree::{RenderId, RenderTree},
  widget::widget_tree::{WidgetId, WidgetTree},
};
use std::{cell::RefCell, rc::Rc};
use winit::event::{ElementState, ModifiersState, MouseButton, WindowEvent};

pub(crate) struct Dispatcher {
  render_tree: Rc<RefCell<RenderTree>>,
  widget_tree: Rc<RefCell<WidgetTree>>,
  cursor_pos: Point,
  button: MouseButton,
  modifiers: ModifiersState,
}

impl Dispatcher {
  pub fn new(render_tree: Rc<RefCell<RenderTree>>, widget_tree: Rc<RefCell<WidgetTree>>) -> Self {
    Self {
      render_tree,
      widget_tree,
      cursor_pos: Point::zero(),
      modifiers: <_>::default(),
      button: MouseButton::Other(0),
    }
  }

  pub fn dispatch(&mut self, event: WindowEvent) {
    log::info!("Dispatch winit event {:?}", event);
    match event {
      WindowEvent::ModifiersChanged(s) => self.modifiers = s,
      WindowEvent::CursorMoved { position, .. } => {
        self.cursor_pos = Point::new(position.x as f32, position.y as f32);
        let mut pointer = PointerEvent::from_mouse();
        self.bubble_callback(self.cursor_pos, |w: &PointerListener, pos| {
          pointer.pos = pos;
          w.dispatch_pointer_move(&pointer)
        })
      }
      WindowEvent::MouseInput { state, button, .. } => {
        self.button = button;
        match state {
          ElementState::Pressed => {
            let mut pointer = None;
            self.bubble_callback(self.cursor_pos, |w: &PointerListener, wid, pos| {
              let mut event = if let Some(ev) = pointer {
                ev
              } else {
                PointerEvent::from_mouse(wid, pos, self.cursor_pos, self.modifiers, self.button)
              };
              event.pos = pos;
              event.common().current_target = wid;
              w.dispatch_pointer_down(&event)
            })
          }
          ElementState::Released => {
            self.bubble_callback(self.cursor_pos, |w: &PointerListener, pos| {
              pointer.pos = pos;
              w.dispatch_pointer_up(&pointer)
            })
          }
        };
      }
      _ => unimplemented!(),
    }
  }

  fn bubble_callback<T: 'static, F: FnMut(&T, WidgetId, Point)>(&self, pos: Point, mut cb: F) {
    self.hit_render_bubble_path(pos, |rid, pos| {
      let r_tree = self.render_tree.borrow();
      let w_tree = self.widget_tree.borrow();
      let widget = rid.relative_to_widget(&r_tree).and_then(|wid| {
        wid
          .get(&w_tree)
          .and_then(|w| w.as_any().downcast_ref::<T>().map(|w| (w, wid)))
      });
      if let Some((w, wid)) = widget {
        cb(w, wid, pos)
      }
    });
  }

  fn hit_render_bubble_path<F: FnMut(RenderId, Point)>(&self, mut pos: Point, mut cb: F) {
    let r_tree = self.render_tree.borrow();
    let mut rid = r_tree.root();

    while let Some(r) = rid {
      let rect = r.box_place(&r_tree);
      let offset: Size = rect.min().to_tuple().into();
      pos += offset;
      if rect.contains(pos) {
        cb(r, pos - offset);
        rid = r
          .reverse_children(&r_tree)
          .find(|r| r.box_place(&r_tree).contains(pos));
      } else {
        break;
      }
    }
  }
}
