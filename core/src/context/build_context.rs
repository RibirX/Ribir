use crate::{prelude::*, ticker::Ticker};
use ::text::FontFamily;
use std::rc::Rc;

thread_local!(static DEFAULT_THEME: Rc<Theme> =
  Rc::new(  widget::material::light(Box::new([FontFamily::Name(std::borrow::Cow::Borrowed("Roboto"))])))
);

pub struct BuildCtx<'a> {
  parent: Option<WidgetId>,
  default_theme: Option<Rc<Theme>>,
  ctx: &'a mut Context,
}

impl<'a> BuildCtx<'a> {
  /// The data from the closest Theme instance that encloses this context.
  pub fn theme(&mut self) -> &Theme {
    let tree = &*self.ctx.widget_tree;
    self
      .parent
      .as_ref()
      .and_then(|p| {
        p.ancestors(tree).find_map(|id| {
          let mut theme: Option<&Theme> = None;
          id.assert_get(tree)
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
  pub(crate) fn new(parent: Option<WidgetId>, ctx: &'a mut Context) -> Self {
    Self { parent, default_theme: None, ctx }
  }

  pub fn ticker(&mut self) -> Ticker { self.ctx.ticker_provider.ticker() }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::{cell::RefCell, rc::Rc};

  #[test]
  #[should_panic(expected = "Get a default theme from context")]
  fn always_have_default_theme() {
    struct T;
    impl Compose for T {
      fn compose(_: Stateful<Self>, ctx: &mut BuildCtx) -> Widget {
        let _ = ctx.theme();
        panic!("Get a default theme from context");
      }
    }
    // should panic when construct the context
    Context::new(T.into_widget(), 1.);
  }

  #[derive(Debug, Declare)]
  struct ThemeTrack {
    themes: Rc<RefCell<Vec<Theme>>>,
  }

  impl Compose for ThemeTrack {
    fn compose(this: Stateful<Self>, ctx: &mut BuildCtx) -> Widget {
      this
        .shallow_ref()
        .themes
        .borrow_mut()
        .push(ctx.theme().clone());
      SizedBox { size: Size::zero() }.into_widget()
    }
  }

  #[test]
  fn nearest_theme() {
    #[derive(Default, Clone)]
    struct DarkLightThemes(Rc<RefCell<Vec<Theme>>>);

    impl Compose for DarkLightThemes {
      fn compose(this: Stateful<Self>, _: &mut BuildCtx) -> Widget {
        let family = Box::new([FontFamily::Name(std::borrow::Cow::Borrowed("serif"))]);
        let dark = material::dark(family.clone());
        let light = material::light(family);

        widget! {
          track { this }
          SizedBox {
            size: SizedBox::expanded_size(),
            theme: dark.clone(),
            SizedBox {
              size: SizedBox::shrink_size(),
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
    wnd.render_ready();
    assert_eq!(track_themes.borrow().len(), 1);
    assert_eq!(
      track_themes.borrow()[0].brightness,
      widget::Brightness::Light
    );

    #[derive(Default, Clone)]
    struct LightDarkThemes(Rc<RefCell<Vec<Theme>>>);

    impl Compose for LightDarkThemes {
      fn compose(this: Stateful<Self>, _: &mut BuildCtx) -> Widget {
        let family = Box::new([FontFamily::Name(std::borrow::Cow::Borrowed("serif"))]);
        let dark = material::dark(family.clone());
        let light = material::light(family);

        widget! {
          track { this }
          SizedBox {
            size: SizedBox::expanded_size(),
            theme: light,
            SizedBox {
              size: SizedBox::shrink_size(),
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
    wnd.render_ready();
    assert_eq!(track_themes.borrow().len(), 1);
    assert_eq!(
      track_themes.borrow()[0].brightness,
      widget::Brightness::Dark
    );
  }
}
