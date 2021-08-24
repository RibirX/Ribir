use crate::prelude::*;
use std::pin::Pin;

lazy_static::lazy_static! {
  static ref DEFAULT_THEME: Theme =  widget::material::light("Roboto".to_string());
}

pub struct BuildCtx<'a> {
  pub(crate) tree: Pin<&'a widget_tree::WidgetTree>,
  wid: WidgetId,
}

impl<'a> BuildCtx<'a> {
  /// The data from the closest Theme instance that encloses this context.
  pub fn theme(&self) -> AttrRef<Theme> {
    let tree = &*self.tree;
    self
      .wid
      .ancestors(tree)
      .find_map(|id| {
        id.get(tree)
          .and_then(|w| (w as &dyn AttrsAccess).find_attr::<Theme>())
      })
      .unwrap_or(AttrRef::Ref(&DEFAULT_THEME))
  }

  #[inline]
  pub(crate) fn new(tree: Pin<&'a widget_tree::WidgetTree>, widget: WidgetId) -> Self {
    Self { tree, wid: widget }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::{cell::RefCell, rc::Rc};

  #[test]
  fn default_theme() {
    let win_size = Size::zero();
    let sized = widget::SizedBox::from_size(win_size);
    let mut wnd = window::Window::without_render(sized.box_it(), win_size);
    wnd.render_ready();
    let tree = wnd.widget_tree();

    let ctx = BuildCtx::new(tree.as_ref(), tree.root().unwrap());
    ctx.theme();
  }

  #[derive(Debug)]
  struct ThemeTrack {
    themes: Rc<RefCell<Vec<Theme>>>,
  }

  impl CombinationWidget for ThemeTrack {
    fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
      self.themes.borrow_mut().push(ctx.theme().clone());
      SizedBox::from_size(Size::zero()).box_it()
    }
  }

  #[test]
  fn nearest_theme() {
    let track_themes: Rc<RefCell<Vec<Theme>>> = <_>::default();
    let dark = material::dark("dark".to_string());
    let light = material::light("light".to_string());

    let theme_track = ThemeTrack { themes: track_themes.clone() };

    let light_theme = SizedBox::shrink()
      .with_theme(light.clone())
      .have(theme_track.box_it());
    let dark_light_theme = SizedBox::expanded()
      .with_theme(dark.clone())
      .have(light_theme.box_it());

    let mut wnd = window::Window::without_render(dark_light_theme.box_it(), Size::zero());
    wnd.render_ready();
    assert_eq!(track_themes.borrow().len(), 1);
    assert_eq!(
      track_themes.borrow()[0].brightness,
      widget::theme::Brightness::Light
    );

    let theme = ThemeTrack { themes: track_themes.clone() };
    let dark_theme = SizedBox::shrink().with_theme(dark).have(theme.box_it());
    let light_dark_theme = SizedBox::expanded()
      .with_theme(light)
      .have(dark_theme.box_it());

    let mut wnd = window::Window::without_render(light_dark_theme.box_it(), Size::zero());
    wnd.render_ready();
    assert_eq!(track_themes.borrow().len(), 2);
    assert_eq!(
      track_themes.borrow()[1].brightness,
      widget::theme::Brightness::Dark
    );
  }
}
