use crate::{prelude::*, widget_tree::WidgetTree};
use std::rc::Rc;

pub struct BuildCtx<'a> {
  pub(crate) theme: Rc<Theme>,
  pub(crate) tree: &'a WidgetTree,
}

impl<'a> BuildCtx<'a> {
  /// The data from the closest Theme instance that encloses this context.
  pub fn theme(&self) -> &Theme { &self.theme }

  #[inline]
  pub fn app_ctx(&self) -> &AppContext { self.tree.app_ctx() }

  #[inline]
  pub(crate) fn new(theme: Rc<Theme>, tree: &'a WidgetTree) -> Self { Self { theme, tree } }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::*;
  use std::{cell::RefCell, rc::Rc};

  #[test]
  #[should_panic(expected = "Get a default theme from context")]
  fn always_have_default_theme() {
    let w = widget! {
      ExprWidget {
        expr: {
          let _ = ctx.theme();
          panic!("Get a default theme from context");
          #[allow(unreachable_code)]
          Void {}
        }
      }
    };
    // should panic when construct widget tree
    Window::default_mock(w, None);
  }

  #[derive(Declare)]
  struct ThemeTrack {
    themes: Rc<RefCell<Vec<Theme>>>,
  }

  impl Compose for ThemeTrack {
    fn compose(this: StateWidget<Self>) -> Widget {
      widget_try_track! {
        try_track { this }
        ExprWidget {
          expr: {
            this
            .themes
            .borrow_mut()
            .push(ctx.theme().clone());
            Void
          }
        }
      }
    }
  }

  #[test]
  fn nearest_theme() {
    #[derive(Default, Clone)]
    struct DarkLightThemes(Rc<RefCell<Vec<Theme>>>);

    impl Compose for DarkLightThemes {
      fn compose(this: StateWidget<Self>) -> Widget {
        let mut theme = Theme::default();
        let light = Rc::new(theme.clone());
        theme.brightness = Brightness::Dark;
        let dark = Rc::new(theme.clone());

        widget! {
          track { this: this.into_stateful() }
          MockBox {
            size: INFINITY_SIZE,
            theme: dark.clone(),
            MockBox {
              size: ZERO_SIZE,
              theme: light.clone(),
              ThemeTrack { themes: this.0.clone() }
            }
          }
        }
      }
    }

    let dark_light = DarkLightThemes::default();
    let track_themes = dark_light.0.clone();
    let mut wnd = Window::default_mock(dark_light.into_widget(), None);
    wnd.draw_frame();
    assert_eq!(track_themes.borrow().len(), 1);
    assert_eq!(track_themes.borrow()[0].brightness, Brightness::Light);

    #[derive(Default, Clone)]
    struct LightDarkThemes(Rc<RefCell<Vec<Theme>>>);

    impl Compose for LightDarkThemes {
      fn compose(this: StateWidget<Self>) -> Widget {
        let mut theme = Theme::default();
        let light = Rc::new(theme.clone());
        theme.brightness = Brightness::Dark;
        let dark = Rc::new(theme);

        widget! {
          track { this: this.into_stateful() }
          MockBox {
            size: INFINITY_SIZE,
            theme: light,
            MockBox {
              size: ZERO_SIZE,
              theme: dark,
              ThemeTrack { themes: this.0.clone() }
            }
          }
        }
      }
    }

    let light_dark = LightDarkThemes::default();
    let track_themes = light_dark.0.clone();
    let mut wnd = Window::default_mock(light_dark.into_widget(), None);
    wnd.draw_frame();
    assert_eq!(track_themes.borrow().len(), 1);
    assert_eq!(track_themes.borrow()[0].brightness, Brightness::Dark);
  }
}
