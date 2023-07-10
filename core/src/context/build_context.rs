use crate::prelude::*;
use std::{ops::Deref, rc::Rc};

pub struct BuildCtx<'a> {
  themes: &'a mut Vec<Rc<Theme>>,
  wnd_ctx: &'a mut WindowCtx,
}

impl<'a> BuildCtx<'a> {
  #[inline]
  pub fn wnd_ctx(&self) -> &WindowCtx { self.wnd_ctx }

  #[inline]
  pub(crate) fn new(themes: &'a mut Vec<Rc<Theme>>, wnd_ctx: &'a mut WindowCtx) -> Self {
    Self { themes, wnd_ctx }
  }

  pub(crate) fn find_cfg<T>(&self, f: impl Fn(&Theme) -> Option<&T>) -> Option<&T> {
    for t in self.themes.iter().rev() {
      let v = f(t);
      if v.is_some() {
        return v;
      } else if matches!(t.deref(), Theme::Full(_)) {
        return None;
      }
    }
    f(AppCtx::app_theme())
  }

  #[inline]
  // todo: should &mut self here, but we need to remove `init ctx =>` first
  pub(crate) fn push_theme(&self, theme: Rc<Theme>) {
    #[allow(clippy::cast_ref_to_mut)]
    let this = unsafe { &mut *(self as *const Self as *mut Self) };
    this.themes.push(theme);
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
        MockBox {
          size: INFINITY_SIZE,
          ThemeWidget {
            theme: Rc::new(Theme::Inherit(InheritTheme {
              palette: Some(Rc::new(dark_palette)),
              ..<_>::default()
            })),
            MockBox {
              size: ZERO_SIZE,
              DynWidget {
                dyns: move |ctx: &BuildCtx| {
                  *themes = ctx.themes.clone();
                  Void
                }
              }
            }
          }
        }
      }
    };

    let mut wnd = TestWindow::new(light_dark);
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
