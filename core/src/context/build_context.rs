use crate::{
  animation::TickerAnimationCtrl,
  dynamic_widget::GenerateInfo,
  prelude::{widget_tree::WidgetTree, *},
};
use ::text::FontFamily;
use std::{cell::RefCell, rc::Rc, time::Duration};

use super::generator_store::GeneratorStore;

thread_local!(static DEFAULT_THEME: Rc<Theme> =
  Rc::new(  widget::material::light(Box::new([FontFamily::Name(std::borrow::Cow::Borrowed("Roboto"))])))
);

pub struct BuildCtx<'a> {
  pub(crate) parent: Option<Parent<'a>>,
  default_theme: Option<Rc<Theme>>,
  animation_ticker: Option<Rc<RefCell<Box<dyn TickerProvider>>>>,
  generator_store: &'a GeneratorStore,
}

pub(crate) struct Parent<'a> {
  id: WidgetId,
  tree: &'a WidgetTree,
}

impl<'a> BuildCtx<'a> {
  /// The data from the closest Theme instance that encloses this context.
  pub fn theme(&mut self) -> &Theme {
    self
      .parent
      .and_then(|p| {
        p.id
          .ancestors(p.tree)
          .find_map(|id| id.assert_get(p.tree).get_theme())
      })
      .unwrap_or_else(|| {
        self
          .default_theme
          .get_or_insert_with(|| DEFAULT_THEME.with(|f| f.clone()))
      })
  }

  #[inline]
  pub(crate) fn new(
    parent: Option<Parent<'a>>,
    animation_ticker: Option<Rc<RefCell<Box<dyn TickerProvider>>>>,
    generator_store: &'a GeneratorStore,
  ) -> Self {
    Self {
      parent,
      default_theme: None,
      animation_ticker,
      generator_store,
    }
  }

  pub fn ticker_ctrl(&self, duration: Duration) -> Option<Box<dyn TickerAnimationCtrl>> {
    self
      .animation_ticker
      .as_ref()
      .map(|ticker| ticker.borrow_mut().ticker_ctrl(duration))
  }

  #[inline]
  pub(crate) fn new_generator_info(&self) -> GenerateInfo {
    self.generator_store.new_generator_info()
  }
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
      fn compose(&self, ctx: &mut BuildCtx) -> BoxedWidget {
        let _ = ctx.theme();
        panic!("Get a default theme from context");
      }
    }
    // should panic when construct the context
    Context::new(T.box_it(), 1., None);
  }

  #[derive(Debug, Declare)]
  struct ThemeTrack {
    themes: Rc<RefCell<Vec<Theme>>>,
  }

  impl Compose for ThemeTrack {
    fn compose(&self, ctx: &mut BuildCtx) -> BoxedWidget {
      self.themes.borrow_mut().push(ctx.theme().clone());
      SizedBox { size: Size::zero() }.box_it()
    }
  }

  #[test]
  fn nearest_theme() {
    #[derive(Default, Clone)]
    struct DarkLightThemes(Rc<RefCell<Vec<Theme>>>);

    impl Compose for DarkLightThemes {
      #[widget]
      fn compose(&self, ctx: &mut BuildCtx) -> BoxedWidget {
        let family = Box::new([FontFamily::Name(std::borrow::Cow::Borrowed("serif"))]);
        let dark = material::dark(family.clone());
        let light = material::light(family);

        widget! {
          declare SizedBox {
            size: SizedBox::expanded_size(),
            theme: dark.clone(),
            SizedBox {
              size: SizedBox::shrink_size(),
              theme: light.clone(),
              ThemeTrack { themes: self.0.clone() }
            }
          }
        }
      }
    }

    let dark_light = DarkLightThemes::default();
    let track_themes = dark_light.0.clone();
    let mut wnd = Window::without_render(dark_light.box_it(), Size::zero());
    wnd.render_ready();
    assert_eq!(track_themes.borrow().len(), 1);
    assert_eq!(
      track_themes.borrow()[0].brightness,
      widget::Brightness::Light
    );

    #[derive(Default, Clone)]
    struct LightDarkThemes(Rc<RefCell<Vec<Theme>>>);

    impl Compose for LightDarkThemes {
      #[widget]
      fn compose(&self, ctx: &mut BuildCtx) -> BoxedWidget {
        let family = Box::new([FontFamily::Name(std::borrow::Cow::Borrowed("serif"))]);
        let dark = material::dark(family.clone());
        let light = material::light(family);

        widget! {
          declare SizedBox {
            size: SizedBox::expanded_size(),
            theme: light,
            SizedBox {
              size: SizedBox::shrink_size(),
              theme: dark,
              ThemeTrack { themes: self.0.clone() }
            }
          }
        }
      }
    }

    let light_dark = LightDarkThemes::default();
    let track_themes = light_dark.0.clone();
    let mut wnd = Window::without_render(light_dark.box_it(), Size::zero());
    wnd.render_ready();
    assert_eq!(track_themes.borrow().len(), 1);
    assert_eq!(
      track_themes.borrow()[0].brightness,
      widget::Brightness::Dark
    );
  }
}
