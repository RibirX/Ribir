use crate::prelude::{widget_tree::WidgetTree, *};
use std::{cell::RefCell, rc::Rc};

thread_local!(static DEFAULT_THEME: Rc<Theme> =
  Rc::new(widget::material::purple::light())
);

pub struct BuildCtx<'a> {
  parent: Option<WidgetId>,
  // todo: use as store current theme?
  default_theme: Option<Rc<Theme>>,
  pub(crate) tree: &'a mut WidgetTree,
}

impl<'a> BuildCtx<'a> {
  /// The data from the closest Theme instance that encloses this context.
  pub fn theme(&mut self) -> &Theme {
    self
      .parent
      .as_ref()
      .and_then(|p| {
        p.ancestors(self.tree).find_map(|id| {
          let mut theme: Option<&Theme> = None;
          id.assert_get(self.tree)
            .query_on_first_type(QueryOrder::InnerFirst, |t: &Theme| {
              // Safety: we known the theme in the widget node should always live longer than
              // the `BuildCtx`
              theme = unsafe { Some(std::mem::transmute(t)) };
            });
          theme
        })
      })
      .unwrap_or_else(|| {
        self
          .default_theme
          .get_or_insert_with(|| DEFAULT_THEME.with(|f| f.clone()))
      })
  }

  #[inline]
  pub fn app_ctx(&self) -> &Rc<RefCell<AppContext>> { self.tree.app_ctx() }

  #[inline]
  pub(crate) fn new(parent: Option<WidgetId>, tree: &'a mut WidgetTree) -> Self {
    Self { parent, default_theme: None, tree }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
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
    WidgetTree::new(w, <_>::default());
  }

  #[derive(Debug, Declare)]
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
        let dark = material::purple::dark();
        let light = material::purple::light();

        widget! {
          track { this: this.into_stateful() }
          SizedBox {
            size: INFINITY_SIZE,
            theme: dark.clone(),
            SizedBox {
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
    let mut wnd = Window::without_render(dark_light.into_widget(), Size::zero());
    wnd.draw_frame();
    assert_eq!(track_themes.borrow().len(), 1);
    assert_eq!(
      track_themes.borrow()[0].brightness,
      widget::Brightness::Light
    );

    #[derive(Default, Clone)]
    struct LightDarkThemes(Rc<RefCell<Vec<Theme>>>);

    impl Compose for LightDarkThemes {
      fn compose(this: StateWidget<Self>) -> Widget {
        let dark = material::purple::dark();
        let light = material::purple::light();

        widget! {
          track { this: this.into_stateful() }
          SizedBox {
            size: INFINITY_SIZE,
            theme: light,
            SizedBox {
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
    let mut wnd = Window::without_render(light_dark.into_widget(), Size::zero());
    wnd.draw_frame();
    assert_eq!(track_themes.borrow().len(), 1);
    assert_eq!(
      track_themes.borrow()[0].brightness,
      widget::Brightness::Dark
    );
  }
}
