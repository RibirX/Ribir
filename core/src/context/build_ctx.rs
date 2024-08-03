use std::{cell::OnceCell, ptr::NonNull, rc::Rc};

use ribir_algo::Sc;
use smallvec::SmallVec;

use crate::{prelude::*, window::WindowId};

/// A context provide during build the widget tree.
pub struct BuildCtx {
  pub(crate) themes: OnceCell<Vec<Sc<Theme>>>,
  /// The widget ID from which the context originates is typically the parent of
  /// the upcoming node to be created, or the ID of the node itself if its
  /// parent is a provider.
  pub(crate) startup: WidgetId,
  /// Widgets from the current widget up to the root widget supply data that the
  /// descendants can access.
  pub(crate) providers: SmallVec<[WidgetId; 1]>,
  /// A node ID has already been allocated for the current building node.
  pub(crate) pre_alloc_id: Option<WidgetId>,
  pub(crate) tree: NonNull<WidgetTree>,
}

/// A handle of `BuildCtx` that you can store it and access the `BuildCtx` later
/// in anywhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BuildCtxHandle {
  startup: WidgetId,
  wnd_id: WindowId,
}

impl BuildCtx {
  /// Return the window of this context is created from.
  pub fn window(&self) -> Rc<Window> { self.tree().window() }

  /// Create a handle of this `BuildCtx` which support `Clone`, `Copy` and
  /// convert back to this `BuildCtx`. This let you can store the `BuildCtx`.
  pub fn handle(&self) -> BuildCtxHandle {
    BuildCtxHandle { wnd_id: self.window().id(), startup: self.startup }
  }

  #[inline]
  pub(crate) fn new(startup: WidgetId, tree: NonNull<WidgetTree>) -> Self {
    // Safety: caller guarantee.
    let t = unsafe { tree.as_ref() };
    let providers = startup
      .ancestors(t)
      .filter(|id| id.queryable(t))
      .collect();
    Self { themes: OnceCell::new(), startup, tree, providers, pre_alloc_id: None }
  }

  pub(crate) fn new_with_data(
    startup: WidgetId, tree: NonNull<WidgetTree>, data: Vec<Sc<Theme>>,
  ) -> Self {
    // Safety: caller guarantee.
    let t = unsafe { tree.as_ref() };
    let providers = startup
      .ancestors(t)
      .filter(|id| id.queryable(t))
      .collect();
    let themes: OnceCell<Vec<Sc<Theme>>> = OnceCell::new();
    // Safety: we just create the `OnceCell` and it's empty.
    unsafe { themes.set(data).unwrap_unchecked() };

    Self { themes, startup, providers, tree, pre_alloc_id: None }
  }

  pub(crate) fn find_cfg<T>(&self, f: impl Fn(&Theme) -> Option<&T>) -> Option<&T> {
    for t in self.themes().iter() {
      let v = f(t);
      if v.is_some() {
        return v;
      } else if matches!(t.deref(), Theme::Full(_)) {
        return None;
      }
    }
    f(AppCtx::app_theme())
  }

  pub(crate) fn themes(&self) -> &Vec<Sc<Theme>> {
    self.themes.get_or_init(|| {
      let mut themes = vec![];
      let tree = self.tree();
      self.startup.ancestors(tree).any(|p| {
        for t in p.query_all_iter::<Sc<Theme>>(tree) {
          themes.push(t.clone());
          if matches!(&**t, Theme::Full(_)) {
            break;
          }
        }

        matches!(themes.last().map(Sc::deref), Some(Theme::Full(_)))
      });
      themes
    })
  }

  pub(crate) fn tree(&self) -> &WidgetTree {
    // Safety: Please refer to the comments in `WidgetTree::tree_mut` for more
    // information.
    unsafe { self.tree.as_ref() }
  }

  pub(crate) fn tree_mut(&mut self) -> &mut WidgetTree {
    let mut tree = self.tree;
    // Safety:
    // The widget tree is only used for building the widget tree. Even if there are
    // multiple mutable references, they are only involved in constructing specific
    // parts of the tree.
    unsafe { tree.as_mut() }
  }
}

impl BuildCtxHandle {
  /// Acquires a reference to the `BuildCtx` in this handle, maybe not exist if
  /// the window is closed or widget is removed.
  pub fn with_ctx<R>(self, f: impl FnOnce(&mut BuildCtx) -> R) -> Option<R> {
    AppCtx::get_window(self.wnd_id).map(|wnd: Rc<Window>| {
      let mut ctx = BuildCtx::new(self.startup, wnd.tree);
      f(&mut ctx)
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{reset_test_env, test_helper::*};

  #[test]
  fn themes() {
    reset_test_env!();

    let themes: Stateful<Vec<Sc<Theme>>> = Stateful::new(vec![]);
    let c_themes = themes.clone_writer();

    let light_dark = fn_widget! {
      let light_palette = Palette {
        brightness: Brightness::Light,
        ..Default::default()
      };
      @ThemeWidget {
        theme: Sc::new(Theme::Inherit(InheritTheme {
          palette: Some(Rc::new(light_palette)),
          ..<_>::default()
        })),
        @ {
          Box::new(fn_widget!{
            let c_themes = c_themes.clone_writer();
            let dark_palette = Palette {
              brightness: Brightness::Dark,
              ..Default::default()
            };
            @MockBox {
              size: INFINITY_SIZE,
              @ThemeWidget {
                theme: Sc::new(Theme::Inherit(InheritTheme {
                  palette: Some(Rc::new(dark_palette)),
                  ..<_>::default()
                })),
                @ {
                  Box::new(fn_widget!{
                    @MockBox {
                      size: ZERO_SIZE,
                      @ {
                        Clone::clone_from(&mut *$c_themes.write(), ctx!().themes());
                        Void
                      }
                    }
                  })
                }
              }
            }
          })
        }
      }
    };

    let wnd = TestWindow::new(light_dark);
    wnd.layout();
    let themes = themes.read();
    assert_eq!(themes.len(), 2);
    let mut iter = themes.iter().filter_map(|t| match t.deref() {
      Theme::Full(t) => Some(t.palette.brightness),
      Theme::Inherit(i) => i
        .palette
        .as_ref()
        .map(|palette| palette.brightness),
    });

    assert_eq!(iter.next(), Some(Brightness::Light));
    assert_eq!(iter.next(), Some(Brightness::Dark));
  }
}
