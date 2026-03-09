use super::{Window, WindowId};
use crate::{context::build_ctx::BuildCtx, prelude::*, widget_tree::widget_id::TrackId};

#[must_use = "if the handle is dropped immediately, the mounted content will be closed. Bind it to \
              a variable or call `retain_until`."]
pub struct MountHandle {
  wnd_id: WindowId,
  entry_id: Option<u64>,
}

#[derive(Default)]
pub(super) struct MountStore {
  next_entry_id: u64,
  entries: Vec<MountedEntry>,
}

struct MountedEntry {
  id: u64,
  generator: Option<GenWidget>,
  track_id: Option<TrackId>,
}

impl Window {
  pub(crate) fn rebuild_mounts(&self) {
    let entries = self
      .mounts
      .borrow()
      .entries
      .iter()
      .filter_map(|entry| {
        entry
          .generator
          .clone()
          .map(|generator| (entry.id, generator))
      })
      .collect::<Vec<_>>();
    for (entry_id, generator) in entries {
      self.remount_generated_entry(entry_id, generator);
    }
  }

  /// Mount an extra widget subtree into this window outside the main content
  /// generator.
  pub fn mount(&self, widget: Widget<'static>) -> MountHandle {
    let entry_id = self.alloc_mount_entry_id();
    let track_id = self.mount_widget(widget);
    self
      .mounts
      .borrow_mut()
      .entries
      .push(MountedEntry { id: entry_id, generator: None, track_id: Some(track_id) });
    MountHandle::new(self.id(), entry_id)
  }

  /// Mount a generator-backed subtree that should be rebuilt alongside the
  /// window's root content.
  pub fn mount_gen(&self, generator: GenWidget) -> MountHandle {
    let entry_id = self.alloc_mount_entry_id();
    let track_id = self.mount_generated_widget(&generator);
    self
      .mounts
      .borrow_mut()
      .entries
      .push(MountedEntry { id: entry_id, generator: Some(generator), track_id: Some(track_id) });
    MountHandle::new(self.id(), entry_id)
  }
  fn mount_widget(&self, widget: Widget<'static>) -> TrackId {
    self.with_mount_build_ctx(|| {
      let mut widget = FatObj::new(widget);
      let track_id = widget.track_id();
      let mounted_root = BuildCtx::get_mut().build(widget.into_widget());
      let tree = self.tree_mut();
      let root = tree.root();
      root.append(mounted_root, tree);
      mounted_root.on_mounted_subtree(tree);
      tree
        .dirty_marker()
        .mark(mounted_root, DirtyPhase::Layout);
      track_id
    })
  }

  fn mount_generated_widget(&self, generator: &GenWidget) -> TrackId {
    self.with_mount_build_ctx(|| self.mount_widget(generator.gen_widget()))
  }

  fn with_mount_build_ctx<R>(&self, f: impl FnOnce() -> R) -> R {
    let root = self.tree().root();
    let _guard = BuildCtx::try_get()
      .is_none()
      .then(|| BuildCtx::init_for(root, self.tree));
    if let Some(ctx) = BuildCtx::try_get() {
      assert_eq!(
        ctx.window().id(),
        self.id(),
        "mount cannot reuse a build context from another window."
      );
    }
    f()
  }

  fn alloc_mount_entry_id(&self) -> u64 {
    let mut mounts = self.mounts.borrow_mut();
    let entry_id = mounts.next_entry_id;
    mounts.next_entry_id += 1;
    entry_id
  }

  fn current_mount_track_id(&self, entry_id: u64) -> Option<TrackId> {
    self
      .mounts
      .borrow()
      .entries
      .iter()
      .find(|entry| entry.id == entry_id)
      .and_then(|entry| entry.track_id.clone())
  }

  fn take_mount_entry(&self, entry_id: u64) -> Option<MountedEntry> {
    let mut mounts = self.mounts.borrow_mut();
    let idx = mounts
      .entries
      .iter()
      .position(|entry| entry.id == entry_id)?;
    Some(mounts.entries.swap_remove(idx))
  }

  fn remount_generated_entry(&self, entry_id: u64, generator: GenWidget) {
    let old_track_id = {
      let mut mounts = self.mounts.borrow_mut();
      let Some(entry) = mounts
        .entries
        .iter_mut()
        .find(|entry| entry.id == entry_id)
      else {
        return;
      };
      entry.track_id.take()
    };

    if let Some(track_id) = old_track_id {
      close_mounted_root(self.id(), &track_id);
    }

    let track_id = self.mount_generated_widget(&generator);
    if let Some(entry) = self
      .mounts
      .borrow_mut()
      .entries
      .iter_mut()
      .find(|entry| entry.id == entry_id)
    {
      entry.track_id = Some(track_id);
    }
  }
}

impl MountHandle {
  fn new(wnd_id: WindowId, entry_id: u64) -> Self { Self { wnd_id, entry_id: Some(entry_id) } }

  pub fn close(mut self) { self.close_inner(); }

  pub fn retain_until<W>(mut self, running: W)
  where
    W: StateWatcher<Value = bool> + 'static,
  {
    let Some(entry_id) = self.entry_id.take() else { return };
    let wnd_id = self.wnd_id;
    let running: Box<dyn StateWatcher<Value = bool>> = Box::new(running);

    if !*running.read() {
      close_mounted_entry(wnd_id, entry_id);
      return;
    }

    let root = AppCtx::get_window(wnd_id).and_then(|wnd| wnd.current_mount_track_id(entry_id));
    let root = root.and_then(|track_id| track_id.get());
    let running_ref = running.clone_boxed_watcher();
    let sub = running.modifies().subscribe(move |_| {
      if !*running_ref.read() {
        close_mounted_entry(wnd_id, entry_id);
      }
    });

    if let Some(wnd) = AppCtx::get_window(wnd_id)
      && let Some(root) = root
      && !root.is_dropped(wnd.tree())
    {
      root.attach_anonymous_data(sub.unsubscribe_when_dropped(), wnd.tree_mut());
    }
  }

  fn close_inner(&mut self) {
    if let Some(entry_id) = self.entry_id.take() {
      close_mounted_entry(self.wnd_id, entry_id);
    }
  }
}

impl Drop for MountHandle {
  fn drop(&mut self) { self.close_inner(); }
}

fn close_mounted_root(wnd_id: WindowId, track_id: &TrackId) {
  let Some(wnd) = AppCtx::get_window(wnd_id) else { return };
  let Some(root) = ({
    let tree = wnd.tree();
    track_id
      .get()
      .filter(|root| !root.is_dropped(tree))
  }) else {
    return;
  };

  let tree = wnd.tree_mut();
  let mount_root = tree.root();
  if root != mount_root {
    root.dispose_subtree(tree);
    tree
      .dirty_marker()
      .mark(mount_root, DirtyPhase::Layout);
  }
}

fn close_mounted_entry(wnd_id: WindowId, entry_id: u64) {
  let Some(wnd) = AppCtx::get_window(wnd_id) else { return };
  let Some(entry) = wnd.take_mount_entry(entry_id) else {
    return;
  };

  if let Some(track_id) = entry.track_id {
    close_mounted_root(wnd_id, &track_id);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{reset_test_env, test_helper::*};

  #[test]
  fn mount_drops_on_handle_drop() {
    reset_test_env!();

    let mounted = Stateful::new(0);
    let disposed = Stateful::new(0);
    let mounted_reader = mounted.clone_reader();
    let disposed_reader = disposed.clone_reader();
    let wnd = TestWindow::new_with_size(
      fn_widget! { @MockBox { size: Size::zero() } },
      Size::new(100., 100.),
    );

    let handle = wnd.mount(
      fn_widget! {
        @MockBox {
          size: Size::new(10., 10.),
          on_mounted: move |_| *$write(mounted) += 1,
          on_disposed: move |_| *$write(disposed) += 1,
        }
      }
      .into_widget(),
    );
    wnd.draw_frame();
    assert_eq!(*mounted_reader.read(), 1);
    assert_eq!(*disposed_reader.read(), 0);

    drop(handle);
    wnd.draw_frame();
    assert_eq!(*disposed_reader.read(), 1);
  }

  #[test]
  fn mount_retain_until_closes_automatically() {
    reset_test_env!();

    let running = Stateful::new(true);
    let disposed = Stateful::new(0);
    let disposed_reader = disposed.clone_reader();
    let wnd = TestWindow::new_with_size(
      fn_widget! { @MockBox { size: Size::zero() } },
      Size::new(100., 100.),
    );

    wnd
      .mount(
        fn_widget! {
          @MockBox {
            size: Size::new(10., 10.),
            on_disposed: move |_| *$write(disposed) += 1,
          }
        }
        .into_widget(),
      )
      .retain_until(running.clone_watcher());
    wnd.draw_frame();
    assert_eq!(*disposed_reader.read(), 0);

    *running.write() = false;
    wnd.draw_frame();
    assert_eq!(*disposed_reader.read(), 1);
  }
}
