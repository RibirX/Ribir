use super::pointers::{PointerEvent, PointerListener};
use crate::{
  prelude::{Point, Size},
  render::render_tree::{RenderId, RenderTree},
  widget::widget_tree::{WidgetId, WidgetTree},
};
use std::{
  cell::{Ref, RefCell},
  rc::Rc,
};
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
        self.bubble_pointer(|w, event| {
          log::info!("Pointer move {:?}", event);
          w.dispatch_pointer_move(event)
        });
      }
      WindowEvent::MouseInput { state, button, .. } => {
        self.button = button;
        match state {
          ElementState::Pressed => {
            self.bubble_pointer(|w, event| {
              log::info!("Pointer down {:?}", event);
              w.dispatch_pointer_down(event)
            });
          }
          ElementState::Released => {
            self.bubble_pointer(|w, event| {
              log::info!("Pointer up {:?}", event);
              w.dispatch_pointer_up(event)
            });
          }
        };
      }
      _ => log::info!("not processed event {:?}", event),
    }
  }

  fn bubble_pointer<D: Fn(&PointerListener, &PointerEvent)>(&mut self, dispatch: D) {
    let mut pointer = None;
    let w_tree = self.widget_tree.borrow();
    self.render_hit_iter().for_each(|(wid, pos)| {
      let w = wid
        .get(&w_tree)
        .and_then(|w| w.as_any().downcast_ref::<PointerListener>());
      if let Some(w) = w {
        let event = pointer.get_or_insert_with(|| {
          PointerEvent::from_mouse(wid, pos, self.cursor_pos, self.modifiers, self.button)
        });
        event.position = pos;
        let common = event.as_mut();
        common.current_target = wid;
        common.composed_path.push(wid);
        dispatch(w, &event);
      }
    })
  }

  fn render_hit_iter<'a>(&'a self) -> impl Iterator<Item = (WidgetId, Point)> + 'a {
    let r_tree = self.render_tree.clone();
    HitRenderIter::new(self.render_tree.borrow(), self.cursor_pos).filter_map(move |(rid, pos)| {
      rid
        .relative_to_widget(&r_tree.borrow())
        .map(|wid| (wid, pos))
    })
  }
}

pub struct HitRenderIter<'a> {
  r_tree: Ref<'a, RenderTree>,
  current: Option<RenderId>,
  pos: Point,
}

impl<'a> HitRenderIter<'a> {
  fn new(r_tree: Ref<'a, RenderTree>, pos: Point) -> Self {
    let mut iter = Self {
      current: r_tree.root(),
      r_tree,
      pos,
    };
    iter.current = iter
      .current
      .filter(|rid| Self::try_down_coordinate(&iter.r_tree, *rid, &mut iter.pos));
    iter
  }

  fn try_down_coordinate(tree: &RenderTree, rid: RenderId, pos: &mut Point) -> bool {
    rid
      .box_place(tree)
      .filter(|rect| rect.contains(*pos))
      .map_or(false, |rect| {
        let offset: Size = rect.min().to_tuple().into();
        *pos -= offset;
        true
      })
  }
}

impl<'a> Iterator for HitRenderIter<'a> {
  type Item = (RenderId, Point);
  fn next(&mut self) -> Option<Self::Item> {
    if let Some(rid) = self.current {
      let value = Some((rid, self.pos));
      let Self { r_tree, pos, .. } = self;
      self.current = rid
        .reverse_children(r_tree)
        .find(|rid| Self::try_down_coordinate(r_tree, *rid, pos));

      return value;
    } else {
    }
    None
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::prelude::*;
  use crate::widget::window::HeadlessWindow;
  use std::{cell::RefCell, rc::Rc};

  fn record_pointer<W: Into<Box<dyn Widget>>>(
    event_stack: Rc<RefCell<Vec<PointerEvent>>>,
    child: W,
  ) -> PointerListener {
    PointerListener::listen_on(child)
      .on_pointer_down({
        let stack = event_stack.clone();
        move |e: &PointerEvent| stack.borrow_mut().push(e.clone())
      })
      .on_pointer_move({
        let stack = event_stack.clone();
        move |e: &PointerEvent| stack.borrow_mut().push(e.clone())
      })
      .on_pointer_up({
        let stack = event_stack.clone();
        move |e: &PointerEvent| stack.borrow_mut().push(e.clone())
      })
      .on_pointer_cancel({
        let stack = event_stack.clone();
        move |e: &PointerEvent| stack.borrow_mut().push(e.clone())
      })
  }

  #[test]
  fn pointer_from_mouse() {
    let event_record = Rc::new(RefCell::new(vec![]));
    let record = record_pointer(event_record.clone(), Text("pointer event test".to_string()));
    let root = record_pointer(event_record.clone(), record);
    let mut wnd = HeadlessWindow::headless(root, DeviceSize::new(100, 100));
    wnd.render_ready();

    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id: unsafe { std::mem::MaybeUninit::uninit().assume_init() },
      position: (1, 1).into(),
      modifiers: ModifiersState::default(),
    });

    {
      let mut records = event_record.borrow_mut();
      assert_eq!(records.len(), 4);
      assert_eq!(records[0].composed_path().len(), 1);
      assert_eq!(records[1].composed_path().len(), 2);
      assert_eq!(records[0].button_num(), 0);
      records.clear();
    }

    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id: unsafe { std::mem::MaybeUninit::uninit().assume_init() },
      state: ElementState::Pressed,
      button: MouseButton::Left,
      modifiers: ModifiersState::default(),
    });

    {
      let mut records = event_record.borrow_mut();
      assert_eq!(records[0].button_num(), 1);
      assert_eq!(records[0].position, (1., 1.).into());
      records.clear();
    }

    todo!("release button and then check, move events button");
  }
}
