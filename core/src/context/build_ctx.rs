use std::{
  cell::{OnceCell, RefCell},
  rc::Rc,
};

use ribir_algo::Sc;

use crate::{prelude::*, window::WindowId};

/// A context provide during build the widget tree.
pub struct BuildCtx<'a> {
  pub(crate) themes: OnceCell<Vec<Sc<Theme>>>,
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
  pub fn ctx_from(&self) -> WidgetId {
    self
      .ctx_from
      .unwrap_or_else(|| self.tree.borrow().root())
  }

  /// Create a handle of this `BuildCtx` which support `Clone`, `Copy` and
  /// convert back to this `BuildCtx`. This let you can store the `BuildCtx`.
  pub fn handle(&self) -> BuildCtxHandle {
    BuildCtxHandle { wnd_id: self.window().id(), ctx_from: self.ctx_from }
  }

  #[inline]
  pub(crate) fn new(from: Option<WidgetId>, tree: &'a RefCell<WidgetTree>) -> Self {
    Self { themes: OnceCell::new(), ctx_from: from, tree }
  }

  pub(crate) fn new_with_data(
    from: Option<WidgetId>, tree: &'a RefCell<WidgetTree>, data: Vec<Sc<Theme>>,
  ) -> Self {
    let themes: OnceCell<Vec<Sc<Theme>>> = OnceCell::new();
    // Safety: we just create the `OnceCell` and it's empty.
    unsafe { themes.set(data).unwrap_unchecked() };

    Self { themes, ctx_from: from, tree }
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
      let Some(p) = self.ctx_from else {
        return themes;
      };

      let tree = &self.tree.borrow();
      p.ancestors(tree).any(|p| {
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
