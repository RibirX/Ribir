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
  fn widget_layout_global_pos(wnd: &Window, id: WidgetId) -> Option<Point> {
    wnd.widget_pos(id).map(|pos| {
      wnd
        .parent(id)
        .map_or(pos, |p| wnd.map_to_global(pos, p))
    })
  }

  fn sync_position(&self, wnd: &Window, self_track: &TrackId) {
    let Some(target_id) = self.target.get() else { return };
    let Some(id) = self_track.get() else { return };

    let (Some(self_size), Some(target_size)) = (wnd.widget_size(id), wnd.widget_size(target_id))
    else {
      return;
    };

    // 1. Calculate the anchor position of the target in the global coordinate
    // system
    let Some(target_global) = Self::widget_layout_global_pos(wnd, target_id) else {
      return;
    };

    let anchor_pos = target_global
      + Vector::new(
        self
          .x_align
          .calculate(target_size.width, self_size.width),
        self
          .y_align
          .calculate(target_size.height, self_size.height),
      );

    // 2. Convert the global anchor position to parent local coordinates
    let parent_global = wnd
      .parent(id)
      .and_then(|p| Self::widget_layout_global_pos(wnd, p))
      .unwrap_or_default();
    let local_pos = anchor_pos - parent_global.to_vector();

    tracing::trace!(target = ?target_id, follower = ?id, ?local_pos, "follow synced position");
    wnd.update_widget_position(id, local_pos);
  }
}

impl<'c> ComposeChild<'c> for Follow {
  type Child = Widget<'c>;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    fn_widget! {
      let wnd = BuildCtx::get().window();
      let mut host = FatObj::new(child);
      let track_id = host.track_id();
      let u = wnd
        .frame_tick_stream()
        .filter(|msg| matches!(msg, FrameMsg::LayoutReady(_)))
        .subscribe(move |_| {
          $read(this).sync_position(&wnd, &track_id);
        });

        @FatObj {
          on_disposed: move |_| u.unsubscribe(),
          @ { host }
        }
    }
    .into_widget()
  }
}

#[cfg(test)]
mod tests {
  use super::Follow;
  use crate::{prelude::*, reset_test_env, test_helper::*, wrap_render::WrapRender};

  #[derive(Declare, Clone)]
  struct Shifted {
    offset: Point,
  }

  impl WrapRender for Shifted {
    fn adjust_position(&self, host: &dyn Render, pos: Point, ctx: &mut PlaceCtx) -> Point {
      host.adjust_position(pos + self.offset.to_vector(), ctx)
    }

    fn wrapper_dirty_phase(&self) -> DirtyPhase { DirtyPhase::Position }
  }

  impl_compose_child_for_wrap_render!(Shifted);

  #[test]
  fn mounted_follow_syncs_position_after_extra_mount_layout() {
    reset_test_env!();

    let (tracked_id, tracked_id_writer) = split_value(<Option<TrackId>>::None);
    let (bubble_track, bubble_track_writer) = split_value(<Option<TrackId>>::None);

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
      fn_widget! {
        let mut bubble = @MockBox { size: Size::new(20., 10.) };
        let bubble_track_id = bubble.track_id();
        let track_id = $clone(bubble_track_id);
        bubble.on_mounted(move |_| *$write(bubble_track_writer) = Some(track_id));

        @Follow {
          target: target_track,
          x_align: AnchorX::center(),
          y_align: AnchorY::under(),
          @ { bubble }
        }
      }
      .into_widget(),
    );
    wnd.draw_frame();

    let bubble_id = bubble_track
      .read()
      .clone()
      .and_then(|track| track.get())
      .expect("follow bubble should mount and expose its track id");
    assert_eq!(wnd.map_to_global(Point::zero(), bubble_id), Point::new(110., 100.));
  }

  #[test]
  fn reused_follow_resyncs_after_remount() {
    reset_test_env!();

    let visible = Stateful::new(true);
    let (bubble_track, bubble_track_writer) = split_value(<Option<TrackId>>::None);
    let wnd = TestWindow::new_with_size(
      fn_widget! {
        let mut target = @MockBox {
          size: Size::new(40., 20.),
          x: 100.,
          y: 80.,
        };
        let target_track = target.track_id();
        let reusable = Reusable::new(
          fn_widget! {
            let mut bubble = @MockBox { size: Size::new(20., 10.) };
            let bubble_track_id = bubble.track_id();
            let track_id = $clone(bubble_track_id);
            bubble.on_mounted(move |_| *$write(bubble_track_writer) = Some(track_id));

            @Follow {
              target: $clone(target_track),
              x_align: AnchorX::center(),
              y_align: AnchorY::above(),
              @ { bubble }
            }
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
      let bubble = bubble_track
        .read()
        .clone()
        .and_then(|track| track.get())
        .expect("follow bubble should be present");
      wnd.map_to_global(Point::zero(), bubble)
    };

    assert_eq!(positioned(), Point::new(110., 70.));

    *visible.write() = false;
    wnd.draw_frame();
    *visible.write() = true;
    wnd.draw_frame();

    assert_eq!(positioned(), Point::new(110., 70.));
  }

  #[test]
  fn follow_stays_correct_under_shifted_ancestor() {
    reset_test_env!();

    let (tracked_id, tracked_id_writer) = split_value(<Option<TrackId>>::None);
    let (bubble_track, bubble_track_writer) = split_value(<Option<TrackId>>::None);

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
      fn_widget! {
        @Shifted {
          offset: Point::new(90., 56.),
          @ {
            fn_widget! {
              let mut bubble = @MockBox { size: Size::new(60., 24.) };
              let bubble_track_id = bubble.track_id();
              let track_id = $clone(bubble_track_id);
              bubble.on_mounted(move |_| *$write(bubble_track_writer) = Some(track_id));

              @Follow {
                target: target_track,
                x_align: AnchorX::center(),
                y_align: AnchorY::above(),
                @ { bubble }
              }
            }
          }
        }
      }
      .into_widget(),
    );
    wnd.draw_frame();

    let bubble_id = bubble_track
      .read()
      .clone()
      .and_then(|track| track.get())
      .expect("follow bubble should mount and expose its track id");
    assert_eq!(wnd.map_to_global(Point::zero(), bubble_id), Point::new(90., 56.));
  }

  #[test]
  fn follow_centers_using_target_layout_box_instead_of_padded_content_origin() {
    reset_test_env!();

    let (tracked_id, tracked_id_writer) = split_value(<Option<TrackId>>::None);
    let (bubble_track, bubble_track_writer) = split_value(<Option<TrackId>>::None);

    let wnd = TestWindow::new_with_size(
      fn_widget! {
        let mut target = @MockBox {
          size: Size::new(61., 40.),
          padding: EdgeInsets::symmetrical(0., 24.),
          x: 120.,
          y: 100.,
        };

        @(target) {
          on_mounted: move |_| *$write(tracked_id_writer) = Some($clone(target.track_id()))
        }
      },
      Size::new(320., 240.),
    );
    wnd.draw_frame();

    let target_track = tracked_id
      .read()
      .clone()
      .expect("target track id should be captured after first layout");

    let _mount = wnd.mount(
      fn_widget! {
        let mut bubble = @MockBox { size: Size::new(20., 10.) };
        let bubble_track_id = bubble.track_id();
        let track_id = $clone(bubble_track_id);
        bubble.on_mounted(move |_| *$write(bubble_track_writer) = Some(track_id));

        @Follow {
          target: target_track,
          x_align: AnchorX::center(),
          y_align: AnchorY::above(),
          @ { bubble }
        }
      }
      .into_widget(),
    );
    wnd.draw_frame();

    let bubble_id = bubble_track
      .read()
      .clone()
      .and_then(|track| track.get())
      .expect("follow bubble should mount and expose its track id");
    assert_eq!(wnd.map_to_global(Point::zero(), bubble_id), Point::new(164.5, 90.));
  }
}
