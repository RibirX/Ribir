use crate::prelude::*;
use ::text::FontFamily;
use std::{pin::Pin, rc::Rc};

thread_local!(static DEFAULT_THEME: Rc<Theme> =
  Rc::new(  widget::material::light(Box::new([FontFamily::Name(std::borrow::Cow::Borrowed("Roboto"))])))
);

pub struct BuildCtx<'a> {
  pub(crate) tree: Pin<&'a widget_tree::WidgetTree>,
  wid: WidgetId,
  default_theme: Option<Rc<Theme>>,
}

impl<'a> BuildCtx<'a> {
  /// The data from the closest Theme instance that encloses this context.
  pub fn theme(&mut self) -> &Theme {
    let tree = &*self.tree;
    self
      .wid
      .ancestors(tree)
      .find_map(|id| {
        id.get(tree)
          .and_then(|w| w.get_attrs())
          .and_then(Attributes::find)
      })
      .unwrap_or_else(|| {
        self
          .default_theme
          .get_or_insert_with(|| DEFAULT_THEME.with(|f| f.clone()))
      })
  }

  #[inline]
  pub(crate) fn new(tree: Pin<&'a widget_tree::WidgetTree>, widget: WidgetId) -> Self {
    Self {
      tree,
      wid: widget,
      default_theme: None,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::{cell::RefCell, rc::Rc};

  #[test]
  fn default_theme() {
    let win_size = Size::zero();
    let sized = widget::SizedBox { size: win_size };
    let mut wnd = window::Window::without_render(sized.box_it(), win_size);
    wnd.render_ready();
    let tree = wnd.widget_tree();

    let ctx = BuildCtx::new(tree.as_ref(), tree.root().unwrap());
    ctx.theme();
  }

  #[derive(Debug, Declare)]
  struct ThemeTrack {
    themes: Rc<RefCell<Vec<Theme>>>,
  }

  impl CombinationWidget for ThemeTrack {
    fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
      self.themes.borrow_mut().push(ctx.theme().clone());
      SizedBox { size: Size::zero() }.box_it()
    }
  }

  #[test]
  fn nearest_theme() {
    let track_themes: Rc<RefCell<Vec<Theme>>> = <_>::default();
    let family = Box::new([FontFamily::Name(CowRc::borrowed("serif"))]);
    let dark = material::dark(family.clone());
    let light = material::light(family);

    let dark_light_theme = declare! {
      SizedBox {
        size: SizedBox::expanded_size(),
        theme: dark.clone(),
        SizedBox {
          size: SizedBox::shrink_size(),
          theme: light.clone(),
          ThemeTrack { themes: track_themes.clone() }
        }
      }
    };

    let mut wnd = window::Window::without_render(dark_light_theme.box_it(), Size::zero());
    wnd.render_ready();
    assert_eq!(track_themes.borrow().len(), 1);
    assert_eq!(
      track_themes.borrow()[0].brightness,
      widget::theme::Brightness::Light
    );

    let light_dark_theme = declare! {
      SizedBox {
        size: SizedBox::expanded_size(),
        theme: light,
        SizedBox {
          size: SizedBox::shrink_size(),
          theme: dark,
          ThemeTrack { themes: track_themes.clone() }
        }
      }
    };

    let mut wnd = window::Window::without_render(light_dark_theme.box_it(), Size::zero());
    wnd.render_ready();
    assert_eq!(track_themes.borrow().len(), 2);
    assert_eq!(
      track_themes.borrow()[1].brightness,
      widget::theme::Brightness::Dark
    );
  }
}
