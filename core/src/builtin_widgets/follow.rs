use std::cell::RefCell;

use crate::{prelude::*, ticker::FrameMsg};

/// A widget that allows its child to follow the position of another widget.
/// This replaces the removed `Global` positioning in `PosX`/`PosY`.
#[derive(Declare, Clone)]
pub struct Follow {
  pub target: TrackId,
  #[declare(default)]
  pub x_align: AnchorX,
  #[declare(default)]
  pub y_align: AnchorY,
}

impl<'c> ComposeChild<'c> for Follow {
  type Child = Widget<'c>;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    fn_widget! {
      let wnd = BuildCtx::get().window();
      let sub = Rc::new(RefCell::new(None));

      @FatObj {
        on_mounted: {
          let ticker = wnd.frame_tick_stream();
          let u_this = this.clone_watcher();
          let u_wnd = wnd.clone();
          move |e| {
            let id = e.current_target();
            let subscription = ticker
              .filter(|msg| matches!(msg, FrameMsg::LayoutReady(_)))
              .subscribe(move |_| {
                let this = $read(u_this);
                if let Some(tid) = this.target.get() {
                  let target_size = u_wnd.widget_size(tid).unwrap_or_default();
                  let target_pos = u_wnd.widget_pos(tid).unwrap_or_default();
                  // Get target's global position
                  let global_pos = tid.parent(u_wnd.tree())
                    .map(|p| u_wnd.map_to_global(target_pos, p))
                    .unwrap_or(target_pos);

                  let self_size = u_wnd.widget_size(id).unwrap_or_default();

                  // Calculate position using alignment
                  let x = global_pos.x + this.x_align.calculate(target_size.width, self_size.width);
                  let y =
                    global_pos.y + this.y_align.calculate(target_size.height, self_size.height);

                  // Convert global position to parent's coordinate system
                  let parent_global_pos = id.parent(u_wnd.tree())
                    .map(|p| u_wnd.map_to_global(Point::zero(), p))
                    .unwrap_or_default();

                  let local_x = x - parent_global_pos.x;
                  let local_y = y - parent_global_pos.y;

                  // Directly update layout_info pos
                  u_wnd.update_widget_position(id, Point::new(local_x, local_y));
                }
              });
            *$clone(sub).borrow_mut() = Some(subscription);
          }
        },
        on_disposed: move |_| if let Some(s) = sub.borrow_mut().take() { s.unsubscribe(); },
        @ {child}
      }
    }
    .into_widget()
  }
}
