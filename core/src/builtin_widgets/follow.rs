use std::cell::RefCell;

use rxrust::subscription::{BoxedSubscription, SubscriptionGuard};

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

impl Follow {
  fn sync_position(wnd: &Window, self_track: &TrackId, follow: &Self) {
    let Some(tid) = follow.target.get() else {
      tracing::trace!("follow skipped position sync because target is not mounted");
      return;
    };

    let Some(id) = self_track.get() else {
      tracing::trace!(target = ?tid, "follow skipped position sync because follower is not mounted");
      return;
    };

    let Some(self_size) = wnd.widget_size(id) else {
      tracing::trace!(follower = ?id, "follow skipped position sync because follower has no layout yet");
      return;
    };

    let target_size = wnd
      .widget_size(tid)
      .expect("Follow target should have layout info when syncing position.");
    let target_pos = wnd
      .widget_pos(tid)
      .expect("Follow target should have a layout position when syncing position.");
    let global_pos = wnd
      .parent(tid)
      .map(|p| wnd.map_to_global(target_pos, p))
      .unwrap_or(target_pos);

    let x = global_pos.x
      + follow
        .x_align
        .calculate(target_size.width, self_size.width);
    let y = global_pos.y
      + follow
        .y_align
        .calculate(target_size.height, self_size.height);

    let parent_global_pos = wnd
      .parent(id)
      .map(|p| wnd.map_to_global(Point::zero(), p))
      .unwrap_or_default();

    let local_x = x - parent_global_pos.x;
    let local_y = y - parent_global_pos.y;

    tracing::trace!(
      follower = ?id,
      target = ?tid,
      x = local_x,
      y = local_y,
      "follow synced position"
    );
    wnd.update_widget_position(id, Point::new(local_x, local_y));
  }
}

impl<'c> ComposeChild<'c> for Follow {
  type Child = Widget<'c>;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    let wnd = BuildCtx::get().window();
    let sub = Rc::new(RefCell::new(None::<SubscriptionGuard<BoxedSubscription>>));
    let ticker = wnd.frame_tick_stream();
    let u_this_finish = this.clone_watcher();
    let u_this_tick = this.clone_watcher();
    let mut host = FatObj::new(child);
    let self_track = host.track_id();

    host.on_mounted({
      let self_track_finish = self_track.clone();
      let self_track_tick = self_track.clone();
      let u_wnd = wnd.clone();
      let sub = sub.clone();
      move |_| {
        u_wnd.once_frame_finished({
          let u_wnd = u_wnd.clone();
          move || {
            let this = u_this_finish.read();
            Follow::sync_position(&u_wnd, &self_track_finish, &this);
          }
        });

        let subscription = ticker
          .filter(|msg| matches!(msg, FrameMsg::LayoutReady(_)))
          .subscribe(move |_| {
            let this = u_this_tick.read();
            Follow::sync_position(&u_wnd, &self_track_tick, &this);
          });
        *sub.borrow_mut() = Some(SubscriptionGuard::new(BoxedSubscription::new(subscription)));
      }
    });

    host.on_disposed(move |_| {
      sub.borrow_mut().take();
    });

    host.into_widget()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{reset_test_env, test_helper::*};

  #[test]
  fn mounted_follow_syncs_position_after_extra_mount_layout() {
    reset_test_env!();

    let (tracked_id, tracked_id_writer) = split_value(<Option<TrackId>>::None);

    let wnd = TestWindow::new_with_size(
      fn_widget! {
        let mut target = @MockBox {
          size: Size::new(40., 20.),
          x: 100.,
          y: 80.,
        };

        @(target) {
          on_mounted: move |_| *$write(tracked_id_writer) = Some($clone(target.track_id()))
        }
      },
      Size::new(240., 200.),
    );
    wnd.draw_frame();

    let target_track = tracked_id
      .read()
      .clone()
      .expect("target track id should be captured after first layout");
    let _mount = wnd.mount(
      follow! {
        target: target_track,
        x_align: AnchorX::center(),
        y_align: AnchorY::under(),
        @MockBox { size: Size::new(20., 10.) }
      }
      .into_widget(),
    );
    wnd.draw_frame();

    let mounted_follow = wnd
      .children(wnd.root())
      .last()
      .expect("mounted follow should be appended to root");
    assert_eq!(wnd.widget_pos(mounted_follow), Some(Point::new(110., 100.)));
  }

  #[test]
  fn reused_follow_resyncs_after_remount() {
    reset_test_env!();

    let visible = Stateful::new(true);
    let wnd = TestWindow::new_with_size(
      fn_widget! {
        let mut target = @MockBox {
          size: Size::new(40., 20.),
          x: 100.,
          y: 80.,
        };
        let target_track = target.track_id();
        let reusable = Reusable::new(
          follow! {
            target: $clone(target_track),
            x_align: AnchorX::center(),
            y_align: AnchorY::above(),
            @MockBox { size: Size::new(20., 10.) }
          }
          .into_widget(),
        );

        @MockMulti {
          @ { target }
          @ { pipe!(*$read(visible)).map(move |show| show.then(|| reusable.get_widget())) }
        }
      },
      Size::new(240., 200.),
    );
    wnd.draw_frame();

    let positioned = || {
      let bubble = wnd
        .children(wnd.root())
        .last()
        .expect("follow bubble should be present");
      wnd
        .widget_pos(bubble)
        .expect("follow bubble should have layout position")
    };

    assert_eq!(positioned(), Point::new(110., 70.));

    *visible.write() = false;
    wnd.draw_frame();
    *visible.write() = true;
    wnd.draw_frame();

    assert_eq!(positioned(), Point::new(110., 70.));
  }
}
