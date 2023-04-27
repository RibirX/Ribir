use crate::prelude::*;
use std::{
  cell::{Ref, RefCell},
  ops::Deref,
  rc::Rc,
};

#[derive(Clone)]
pub struct BuildCtx<'a> {
  themes: &'a RefCell<Vec<Rc<Theme>>>,
  wnd_ctx: &'a WindowCtx,
}

impl<'a> BuildCtx<'a> {
  #[inline]
  pub fn wnd_ctx(&self) -> &WindowCtx { self.wnd_ctx }

  #[inline]
  pub(crate) fn new(themes: &'a RefCell<Vec<Rc<Theme>>>, wnd_ctx: &'a WindowCtx) -> Self {
    Self { themes, wnd_ctx }
  }

  pub(crate) fn find_cfg<T>(&self, f: impl Fn(&Theme) -> Option<&T>) -> Option<Ref<'_, T>> {
    let themes = self.themes.borrow();
    Ref::filter_map(themes, |themes| {
      let mut v = None;
      for t in themes.iter().rev() {
        v = f(t);
        if v.is_some() || matches!(t.deref(), Theme::Full(_)) {
          break;
        }
      }
      v
    })
    .ok()
  }

  #[inline]
  pub fn app_ctx(&self) -> &AppContext { &self.wnd_ctx.app_ctx }

  pub(crate) fn push_theme(&self, theme: Rc<Theme>) { self.themes.borrow_mut().push(theme); }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::borrow::Borrow;
  use std::{cell::RefCell, rc::Rc};

  #[test]
  #[should_panic(expected = "Get a default theme from context")]
  fn always_have_default_theme() {
    let w = widget! {
      DynWidget {
        dyns: move |ctx: &BuildCtx| {
          assert!(ctx.themes.borrow().len() > 0);
          panic!("Get a default theme from context");
          #[allow(unreachable_code)]
          Void {}
        }
      }
    };
    // should panic when construct widget tree
    Window::default_mock(w, None);
  }

  #[test]
  fn themes() {
    #[derive(Default, Clone)]
    struct LightDarkThemes(Rc<RefCell<Vec<Theme>>>);

    let themes: Stateful<Vec<Rc<Theme>>> = Stateful::new(vec![]);
    let light_palette = Palette {
      brightness: Brightness::Light,
      ..Default::default()
    };
    let dark_palette = Palette {
      brightness: Brightness::Dark,
      ..Default::default()
    };
    let light_dark = widget! {
      states { themes: themes.clone() }
      ThemeWidget {
        theme: Rc::new(Theme::Inherit(InheritTheme {
          palette: Some(Rc::new(light_palette)),
          ..<_>::default()

        })),
        Container {
          size: INFINITY_SIZE,
          ThemeWidget {
            theme: Rc::new(Theme::Inherit(InheritTheme {
              palette: Some(Rc::new(dark_palette)),
              ..<_>::default()
            })),
            Container {
              size: ZERO_SIZE,
              DynWidget {
                dyns: move |ctx: &BuildCtx| {
                  *themes = ctx.themes.borrow().clone();
                  Void
                }
              }
            }
          }
        }
      }
    };

    let mut wnd = Window::default_mock(light_dark, None);
    wnd.layout();
    let themes = themes.state_ref();
    assert_eq!(themes.borrow().len(), 3);
    let mut iter = themes.borrow().iter().filter_map(|t| match t.deref() {
      Theme::Full(t) => Some(t.palette.brightness),
      Theme::Inherit(i) => i.palette.as_ref().map(|palette| palette.brightness),
    });

    assert_eq!(iter.next(), Some(Brightness::Light));
    assert_eq!(iter.next(), Some(Brightness::Light));
    assert_eq!(iter.next(), Some(Brightness::Dark));
  }
}
