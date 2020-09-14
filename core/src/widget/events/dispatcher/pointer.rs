use super::{focus::Focus, CommonDispatcher, FocusManager};
use crate::{prelude::*, render::render_tree::RenderTree};
use rxrust::prelude::*;
use winit::event::{DeviceId, ElementState, MouseButton};

#[derive(Default)]
pub(crate) struct PointerDispatcher {
  cursor_pos: Point,
  last_pointer_widget: Option<WidgetId>,
  mouse_button: (Option<DeviceId>, MouseButtons),
  pointer_down_uid: Option<WidgetId>,
}

impl PointerDispatcher {
  pub fn cursor_move_to(&mut self, position: Point, common: &CommonDispatcher) {
    self.cursor_pos = position;
    self.pointer_enter_leave_dispatch(common);
    self.bubble_pointer(PointerEventType::Move, common);
  }

  pub fn on_cursor_left(&mut self, common: &CommonDispatcher) {
    self.cursor_pos = Point::new(-1., -1.);
    self.pointer_enter_leave_dispatch(common);
  }

  pub fn dispatch_mouse_input(
    &mut self,
    device_id: DeviceId,
    state: ElementState,
    button: MouseButton,
    common: &CommonDispatcher,
    focus_mgr: &mut FocusManager,
  ) {
    // A mouse press/release emit during another mouse's press will ignored.
    if self.mouse_button.0.get_or_insert(device_id) == &device_id {
      let path = self.widget_hit_path(common);
      match state {
        ElementState::Pressed => {
          self.mouse_button.1 |= button.into();
          // only the first button press emit event.
          if self.mouse_button.1 == button.into() {
            self.pointer_down_uid = path.last().map(|(id, _)| *id);

            self.bubble_mouse_down(common, focus_mgr);
          }
        }
        ElementState::Released => {
          self.mouse_button.1.remove(button.into());
          // only the last button release emit event.
          if self.mouse_button.1.is_empty() {
            self.mouse_button.0 = None;
            self.bubble_pointer(PointerEventType::Up, common);

            let release_on = path.last().map(|(id, _)| *id);
            let common_ancestor = self.pointer_down_uid.take().and_then(|down| {
              release_on
                .and_then(|release| down.common_ancestor_of(release, common.widget_tree_ref()))
            });
            if let Some(from) = common_ancestor {
              let iter = path.iter().rev().skip_while(|w| w.0 != from);
              self.bubble_pointer_by_path(PointerEventType::Tap, iter, common);
            }
          }
        }
      };
    }
  }

  fn bubble_mouse_down(&self, common: &CommonDispatcher, focus_mgr: &mut FocusManager) {
    let tree = common.widget_tree_ref();
    let nearest_focus = self.pointer_down_uid.and_then(|wid| {
      wid.ancestors(tree).find(|id| {
        id.get(tree)
          .and_then(|widget| Widget::dynamic_cast_ref::<Focus>(widget))
          .is_some()
      })
    });
    if let Some(focus_id) = nearest_focus {
      focus_mgr.focus(focus_id, common);
    } else {
      focus_mgr.blur(common);
    }
    self.bubble_pointer(PointerEventType::Down, common);
  }

  fn bubble_pointer(
    &self,
    event_type: PointerEventType,
    common: &CommonDispatcher,
  ) -> PointerEvent {
    self.bubble_pointer_by_path(
      event_type,
      self.widget_hit_path(common).iter().rev(),
      common,
    )
  }

  fn bubble_pointer_by_path<'r>(
    &self,
    event_type: PointerEventType,
    mut path: impl Iterator<Item = &'r (WidgetId, Point)>,
    common: &CommonDispatcher,
  ) -> PointerEvent {
    let event = self.mouse_pointer_without_target(common);
    let mut init_target = false;
    let res = path.try_fold(event, |mut event, (wid, pos)| {
      if !init_target {
        event.as_mut().target = *wid;
        init_target = true;
      }
      event.position = *pos;
      event = self.dispatch_pointer(*wid, event_type, event, common);
      CommonDispatcher::ok_bubble(event)
    });
    match res {
      Ok(event) => event,
      Err(event) => event,
    }
  }

  fn dispatch_pointer(
    &self,
    wid: WidgetId,
    pointer_type: PointerEventType,
    event: PointerEvent,
    common: &CommonDispatcher,
  ) -> PointerEvent {
    log::info!("{:?} {:?}", pointer_type, event);
    common.dispatch_to(
      wid,
      &mut |widget: &PointerListener, e| widget.pointer_observable().next((pointer_type, e)),
      event,
    )
  }

  fn pointer_enter_leave_dispatch(&mut self, common: &CommonDispatcher) {
    let mut event = self.mouse_pointer_without_target(common);
    let mut old_path = if let Some(last) = self.last_pointer_widget {
      last.ancestors(common.widget_tree_ref()).collect::<Vec<_>>()
    } else {
      vec![]
    };
    let mut new_path = self.widget_hit_path(common);
    // Remove the common ancestors of `old_path` and `new_path`
    while !old_path.is_empty() && old_path.last() == new_path.first().map(|(wid, _)| wid) {
      old_path.pop();
      new_path.remove(0);
    }

    event = old_path.iter().fold(event, |mut event, wid| {
      event.position = self.widget_relative_point(*wid, common);
      self.dispatch_pointer(*wid, PointerEventType::Leave, event, common)
    });

    new_path.iter().fold(event, |mut event, (wid, pos)| {
      event.position = *pos;
      self.dispatch_pointer(*wid, PointerEventType::Enter, event, common)
    });
    self.last_pointer_widget = new_path.last().map(|(wid, _)| *wid);
  }

  /// collect the render widget hit path.
  fn widget_hit_path(&self, common: &CommonDispatcher) -> Vec<(WidgetId, Point)> {
    fn down_coordinate_to(
      id: RenderId,
      pos: Point,
      tree: &RenderTree,
    ) -> Option<(RenderId, Point)> {
      id.layout_box_rect(tree)
        .filter(|rect| rect.contains(pos))
        .map(|rect| {
          let offset: Size = rect.min().to_tuple().into();
          (id, pos - offset)
        })
    }

    let r_tree = common.render_tree_ref();
    let mut current = r_tree
      .root()
      .and_then(|id| down_coordinate_to(id, self.cursor_pos, &r_tree));

    let mut path = vec![];
    while let Some((rid, pos)) = current {
      path.push((
        rid
          .relative_to_widget(&r_tree)
          .expect("Render object 's owner widget is not exist."),
        pos,
      ));
      current = rid
        .reverse_children(&r_tree)
        .find_map(|rid| down_coordinate_to(rid, pos, &r_tree));
    }

    path
  }

  fn widget_relative_point(&self, wid: WidgetId, common: &CommonDispatcher) -> Point {
    let r_tree = common.render_tree_ref();
    if let Some(rid) = wid.relative_to_render(common.widget_tree_ref()) {
      rid
        .ancestors(r_tree)
        .map(|r| {
          r.layout_box_rect(r_tree)
            .map_or_else(Point::zero, |rect| rect.origin)
        })
        .fold(Point::zero(), |sum, pos| sum + pos.to_vector())
    } else {
      unreachable!("");
    }
  }

  fn mouse_pointer_without_target(&self, common: &CommonDispatcher) -> PointerEvent {
    unsafe {
      PointerEvent::from_mouse_with_dummy_target(
        self.cursor_pos,
        common.modifiers,
        self.mouse_button.1,
        common.window.clone(),
      )
    }
  }
}
