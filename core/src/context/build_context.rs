use crate::{
  prelude::*,
  widget::{widget_id::new_node, WidgetTree},
  window::{DelayEvent, WindowId},
};
use std::{
  cell::{Ref, RefCell, UnsafeCell},
  ops::Deref,
  rc::Rc,
};

/// A context provide during build the widget tree.
pub struct BuildCtx<'a> {
  // tmp `UnsafeCell` before use `BuildCtx` as mutable reference.
  pub(crate) themes: UnsafeCell<Option<Vec<Rc<Theme>>>>,
  /// The widget which this `BuildCtx` is created from. It's not means this
  /// is the parent of the widget which is builded by this `BuildCtx`.
  ctx_from: Option<WidgetId>,
  pub(crate) tree: &'a RefCell<WidgetTree>,
}

/// A handle of `BuildCtx` that you can store it and access the `BuildCtx` later
/// in anywhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BuildCtxHandle {
  ctx_from: Option<WidgetId>,
  wnd_id: WindowId,
}

impl<'a> BuildCtx<'a> {
  /// Return the window of this context is created from.
  pub fn window(&self) -> Rc<Window> { self.tree.borrow().window() }

  /// Get the widget which this `BuildCtx` is created from.
  pub fn ctx_from(&self) -> WidgetId { self.ctx_from.unwrap_or_else(|| self.tree.borrow().root()) }

  /// Create a handle of this `BuildCtx` which support `Clone`, `Copy` and
  /// convert back to this `BuildCtx`. This let you can store the `BuildCtx`.
  pub fn handle(&self) -> BuildCtxHandle {
    BuildCtxHandle {
      wnd_id: self.window().id(),
      ctx_from: self.ctx_from,
    }
  }

  pub fn reset_ctx_from(&mut self, reset: Option<WidgetId>) -> Option<WidgetId> {
    std::mem::replace(&mut self.ctx_from, reset)
  }

  #[inline]
  pub(crate) fn new(from: Option<WidgetId>, tree: &'a RefCell<WidgetTree>) -> Self {
    Self {
      themes: UnsafeCell::new(None),
      ctx_from: from,
      tree,
    }
  }

  pub(crate) fn find_cfg<T>(&self, f: impl Fn(&Theme) -> Option<&T>) -> Option<&T> {
    for t in self.themes().iter().rev() {
      let v = f(t);
      if v.is_some() {
        return v;
      } else if matches!(t.deref(), Theme::Full(_)) {
        return None;
      }
    }
    f(AppCtx::app_theme())
  }

  /// Get the widget back of `id`, panic if not exist.
  pub(crate) fn assert_get(&self, id: WidgetId) -> Ref<dyn Render> {
    Ref::map(self.tree.borrow(), |tree| id.assert_get(&tree.arena))
  }

  pub(crate) fn alloc_widget(&self, widget: Box<dyn Render>) -> WidgetId {
    new_node(&mut self.tree.borrow_mut().arena, widget)
  }

  pub(crate) fn append_child(&mut self, parent: WidgetId, child: WidgetId) {
    parent.append(child, &mut self.tree.borrow_mut().arena);
  }

  /// Insert `next` after `prev`
  pub(crate) fn insert_after(&mut self, prev: WidgetId, next: WidgetId) {
    prev.insert_after(next, &mut self.tree.borrow_mut().arena);
  }

  /// After insert new subtree to the widget tree, call this to watch the
  /// subtree and fire mount events.
  pub(crate) fn on_subtree_mounted(&self, id: WidgetId) {
    id.descendants(&self.tree.borrow().arena)
      .for_each(|w| self.on_widget_mounted(w));
    self.tree.borrow_mut().mark_dirty(id);
  }

  /// After insert new widget to the widget tree, call this to watch the widget
  /// and fire mount events.
  pub(crate) fn on_widget_mounted(&self, id: WidgetId) {
    self.assert_get(id).query_all_type(
      |notifier: &Notifier| {
        let state_changed = self.tree.borrow().dirty_set.clone();
        notifier
          .raw_modifies()
          .filter(|b| b.contains(ModifyScope::FRAMEWORK))
          .subscribe(move |_| {
            state_changed.borrow_mut().insert(id);
          });
        true
      },
      QueryOrder::OutsideFirst,
    );

    self.window().add_delay_event(DelayEvent::Mounted(id));
  }

  /// Dispose the whole subtree of `id`, include `id` itself.
  pub(crate) fn dispose_subtree(&self, id: WidgetId) {
    let mut tree = self.tree.borrow_mut();
    let parent = id.parent(&tree.arena);
    tree.detach(id);
    tree
      .window()
      .add_delay_event(DelayEvent::Disposed { id, parent });
  }

  pub(crate) fn mark_dirty(&mut self, id: WidgetId) { self.tree.borrow_mut().mark_dirty(id); }

  #[inline]
  pub(crate) fn push_theme(&self, theme: Rc<Theme>) { self.themes().push(theme); }

  #[inline]
  pub(crate) fn pop_theme(&self) { self.themes().pop(); }

  /// todo: tmp code
  /// because we use `BuildCtx` as reference now, but we need to use it as
  /// mutable reference. Do a unsafe cast here, and remove it when we use
  /// `BuildCtx` as mutable reference in `Widget`
  #[allow(clippy::mut_from_ref)]
  pub(crate) fn force_as_mut(&self) -> &mut Self {
    #[allow(clippy::cast_ref_to_mut)]
    unsafe {
      &mut *(self as *const Self as *mut Self)
    }
  }

  #[allow(clippy::mut_from_ref)]
  fn themes(&self) -> &mut Vec<Rc<Theme>> {
    let this = self.force_as_mut();
    unsafe { &mut *this.themes.get() }.get_or_insert_with(|| {
      let mut themes = vec![];
      let Some(p) = self.ctx_from else { return themes };

      let arena = &this.tree.borrow().arena;
      p.ancestors(arena).any(|p| {
        p.assert_get(arena).query_all_type(
          |t: &Rc<Theme>| {
            themes.push(t.clone());
            matches!(t.deref(), Theme::Inherit(_))
          },
          QueryOrder::InnerFirst,
        );
        matches!(themes.last().map(Rc::deref), Some(Theme::Full(_)))
      });
      themes
    })
  }
}

impl BuildCtxHandle {
  /// Acquires a reference to the `BuildCtx` in this handle, maybe not exist if
  /// the window is closed or widget is removed.
  pub fn with_ctx<R>(self, f: impl FnOnce(&BuildCtx) -> R) -> Option<R> {
    AppCtx::get_window(self.wnd_id).map(|wnd| {
      let mut ctx = BuildCtx::new(self.ctx_from, &wnd.widget_tree);
      f(&mut ctx)
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_helper::*;
  use std::{cell::RefCell, rc::Rc};

  #[test]
  fn themes() {
    let _guard = unsafe { AppCtx::new_lock_scope() };

    #[derive(Default, Clone)]
    struct LightDarkThemes(Rc<RefCell<Vec<Theme>>>);

    let themes: Stateful<Vec<Rc<Theme>>> = Stateful::new(vec![]);
    let c_themes = themes.clone_writer();
    let light_palette = Palette {
      brightness: Brightness::Light,
      ..Default::default()
    };
    let dark_palette = Palette {
      brightness: Brightness::Dark,
      ..Default::default()
    };
    let light_dark = fn_widget! {
      @ThemeWidget {
        theme: Rc::new(Theme::Inherit(InheritTheme {
          palette: Some(Rc::new(light_palette)),
          ..<_>::default()

        })),
        @MockBox {
          size: INFINITY_SIZE,
          @ThemeWidget {
            theme: Rc::new(Theme::Inherit(InheritTheme {
              palette: Some(Rc::new(dark_palette)),
              ..<_>::default()
            })),
            @MockBox {
              size: ZERO_SIZE,
              @ {
                FnWidget::new(move |ctx: &BuildCtx| {
                  *$c_themes.write() = ctx.themes().clone();
                  Void
                })
              }

            }
          }
        }
      }
    };

    let wnd = TestWindow::new(light_dark);
    wnd.layout();
    let themes = themes.state_ref();
    assert_eq!(themes.len(), 2);
    let mut iter = themes.iter().filter_map(|t| match t.deref() {
      Theme::Full(t) => Some(t.palette.brightness),
      Theme::Inherit(i) => i.palette.as_ref().map(|palette| palette.brightness),
    });

    assert_eq!(iter.next(), Some(Brightness::Light));
    assert_eq!(iter.next(), Some(Brightness::Dark));
  }
}
