use crate::prelude::*;
use std::pin::Pin;

pub struct BuildCtx<'a> {
  pub(crate) tree: Pin<&'a mut widget_tree::WidgetTree>,
  #[allow(dead_code)]
  widget: WidgetId,
}

impl<'a> BuildCtx<'a> {
  #[inline]
  pub(crate) fn new(tree: Pin<&'a mut widget_tree::WidgetTree>, widget: WidgetId) -> Self {
    Self { tree, widget }
  }

  /// The data from the closest Theme instance that encloses this context.
  pub fn theme(&self) -> &ThemeData {
    let tree = &*self.tree;
    let theme = self
      .widget
      .ancestors(tree)
      .find_map(|id| {
        id.get(tree)
          .and_then(|w| Widget::dynamic_cast_ref::<Theme>(w))
      })
      .expect("At least， root theme should be found.");
    &theme.data
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
    let theme = tree
      .root()
      .and_then(|root| root.get(&*tree))
      .and_then(|root| Widget::dynamic_cast_ref::<Theme>(root));

    assert!(theme.is_some());
  }

  #[derive(Debug)]
  struct ThemeTrack {
    themes: Rc<RefCell<Vec<ThemeData>>>,
  }

  impl CombinationWidget for ThemeTrack {
    fn build(&self, ctx: &mut BuildCtx) -> BoxWidget {
      self.themes.borrow_mut().push(ctx.theme().clone());
      SizedBox::empty_box(Size::zero()).box_it()
    }
  }

  #[test]
  fn nearest_theme() {
    let track_themes: Rc<RefCell<Vec<ThemeData>>> = <_>::default();
    let dark = material::dark("dark".to_string());
    let light = material::light("light".to_string());

    let theme = ThemeTrack {
      themes: track_themes.clone(),
    };
    let light_theme = Theme {
      data: dark.clone(),
      widget: SizedBox::expanded(Theme {
        data: light.clone(),
        widget: SizedBox::shrink(theme).box_it(),
      })
      .box_it(),
    };
    let mut wnd = window::Window::without_render(light_theme, Size::zero());
    wnd.render_ready();
    assert_eq!(track_themes.borrow().len(), 1);
    assert_eq!(track_themes.borrow()[0], light);

    let theme = ThemeTrack {
      themes: track_themes.clone(),
    };
    let dark_theme = Theme {
      data: light.clone(),
      widget: SizedBox::expanded(Theme {
        data: dark.clone(),
        widget: SizedBox::shrink(theme).box_it(),
      })
      .box_it(),
    };
    let mut wnd = window::Window::without_render(dark_theme, Size::zero());
    wnd.render_ready();
    assert_eq!(track_themes.borrow().len(), 2);
    assert_eq!(track_themes.borrow()[1], dark);
  }
}
