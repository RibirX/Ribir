use crate::prelude::*;
use std::pin::Pin;

static DEFAULT_THEME: ThemeData = material::light("Roboto".to_string());
pub struct BuildCtx<'a> {
  pub(crate) tree: Pin<&'a mut widget_tree::WidgetTree>,
  wid: WidgetId,
}

impl<'a> BuildCtx<'a> {
  /// The data from the closest Theme instance that encloses this context.
  pub fn theme(&self) -> &ThemeData {
    let tree = &*self.tree;
    self
      .wid
      .ancestors(tree)
      .find_map(|id| id.get(tree).and_then(|w| w.find_attr::<ThemeData>()))
      .unwrap_or(&DEFAULT_THEME)
  }

  #[inline]
  pub(crate) fn new(tree: Pin<&'a mut widget_tree::WidgetTree>, widget: WidgetId) -> Self {
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
    let sized = widget::SizedBox::empty_box(win_size);
    let mut wnd = window::Window::without_render(sized, win_size);
    wnd.render_ready();
    let tree = wnd.widget_tree();
    let has_them = tree
      .root()
      .and_then(|root| root.get(&*tree))
      .map_or(false, |w| w.find_attr::<ThemeData>().is_some());
    assert!(has_them);
  }

  #[derive(Debug, Widget)]
  struct ThemeTrack {
    themes: Rc<RefCell<Vec<ThemeData>>>,
  }

  impl CombinationWidget for ThemeTrack {
    fn build(&self, ctx: &mut BuildCtx) -> Box<dyn Widget> {
      self.themes.borrow_mut().push(ctx.theme().clone());
      SizedBox::empty_box(Size::zero()).box_it()
    }
  }

  #[test]
  fn nearest_theme() {
    let track_themes: Rc<RefCell<Vec<ThemeData>>> = <_>::default();
    let dark = material::dark("dark".to_string());
    let light = material::light("light".to_string());

    let theme_track = ThemeTrack { themes: track_themes.clone() };

    let light_theme = SizedBox::shrink(theme_track).with_theme(light.clone());
    let dark_light_theme = SizedBox::expanded(light_theme).with_theme(dark.clone());

    let mut wnd = window::Window::without_render(dark_light_theme, Size::zero());
    wnd.render_ready();
    assert_eq!(track_themes.borrow().len(), 1);
    assert_eq!(
      track_themes.borrow()[0].brightness,
      theme_data::Brightness::Light
    );

    let theme = ThemeTrack { themes: track_themes.clone() };
    let dark_theme = SizedBox::shrink(theme).with_theme(dark);
    let light_dark_theme = SizedBox::expanded(dark_theme).with_theme(light);

    let mut wnd = window::Window::without_render(light_dark_theme, Size::zero());
    wnd.render_ready();
    assert_eq!(track_themes.borrow().len(), 2);
    assert_eq!(
      track_themes.borrow()[1].brightness,
      theme_data::Brightness::Dark
    );
  }
}
