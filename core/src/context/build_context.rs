use crate::prelude::*;
use ::text::FontFamily;
use std::{marker::PhantomData, rc::Rc};

thread_local!(static DEFAULT_THEME: Rc<Theme> =
  Rc::new(  widget::material::light(Box::new([FontFamily::Name(std::borrow::Cow::Borrowed("Roboto"))])))
);

pub struct BuildCtx<'a, W> {
  widget: &'a dyn CombinationNode,
  parent: Option<WidgetId>,
  ctx: &'a Context,
  default_theme: Option<Rc<Theme>>,
  _mark: PhantomData<W>,
}

impl<'a, W> BuildCtx<'a, W> {
  /// The data from the closest Theme instance that encloses this context.
  pub fn theme(&mut self) -> &Theme {
    let tree = &self.ctx.widget_tree;
    self
      .widget
      .as_attrs()
      .and_then(Attributes::find)
      .or_else(|| {
        self.parent.and_then(|p| {
          p.ancestors(tree)
            .find_map(|id| id.assert_get(tree).get_theme())
        })
      })
      .unwrap_or_else(|| {
        self
          .default_theme
          .get_or_insert_with(|| DEFAULT_THEME.with(|f| f.clone()))
      })
  }

  #[inline]
  pub fn state_ref(&self) -> StateRef<W> { todo!("") }

  #[inline]
  pub(crate) fn new(
    ctx: &'a Context,
    parent: Option<WidgetId>,
    widget: &'a dyn CombinationNode,
  ) -> Self {
    Self {
      ctx,
      parent,
      default_theme: None,
      widget,
      _mark: PhantomData,
    }
  }

  /// Caller promise `X` And `W` are same widget.
  pub(crate) unsafe fn cast_type<X>(&mut self) -> &mut BuildCtx<'a, X> { std::mem::transmute(self) }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::{cell::RefCell, rc::Rc};

  #[test]
  #[should_panic(expected = "Get a default theme from context")]
  fn always_have_default_theme() {
    struct T;
    impl CombinationWidget for T {
      fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
        let _ = ctx.theme();
        panic!("Get a default theme from context");
      }
    }
    // should panic when construct the context
    Context::new(T.box_it(), 1.);
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
    let family = Box::new([FontFamily::Name(std::borrow::Cow::Borrowed("serif"))]);
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

    let mut wnd = Window::without_render(dark_light_theme.box_it(), Size::zero());
    wnd.render_ready();
    assert_eq!(track_themes.borrow().len(), 1);
    assert_eq!(
      track_themes.borrow()[0].brightness,
      widget::Brightness::Light
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

    let mut wnd = Window::without_render(light_dark_theme.box_it(), Size::zero());
    wnd.render_ready();
    assert_eq!(track_themes.borrow().len(), 2);
    assert_eq!(
      track_themes.borrow()[1].brightness,
      widget::Brightness::Dark
    );
  }
}
