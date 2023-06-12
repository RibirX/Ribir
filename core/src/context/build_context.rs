use crate::{
  prelude::*,
  widget::{widget_id::new_node, WidgetTree},
};
use std::{ops::Deref, rc::Rc};

pub struct BuildCtx<'a> {
  pub(crate) themes: Option<Vec<Rc<Theme>>>,
  /// The widget which this `BuildCtx` is created from. It's not means this
  /// is the parent of the widget which is builded by this `BuildCtx`.
  ctx_from: Option<WidgetId>,
  tree: &'a mut WidgetTree,
}

impl<'a> BuildCtx<'a> {
  pub fn window(&self) -> Rc<Window> { self.tree.window() }

  /// Get the widget which this `BuildCtx` is created from.
  pub fn ctx_from(&self) -> WidgetId { self.ctx_from.unwrap_or_else(|| self.tree.root()) }

  pub fn reset_ctx_from(&mut self, reset: Option<WidgetId>) -> Option<WidgetId> {
    std::mem::replace(&mut self.ctx_from, reset)
  }

  #[inline]
  pub(crate) fn new(from: Option<WidgetId>, tree: &'a mut WidgetTree) -> Self {
    Self { themes: None, ctx_from: from, tree }
  }

  pub(crate) fn find_cfg<T>(&self, f: impl Fn(&Theme) -> Option<&T>) -> Option<&T> {
    for t in self.themes().iter().rev() {
      let v = f(t);
      if v.is_some() {
        return v;
      } else if matches!(t.deref(), Theme::Full(_)) {
        return None;
      }
    }
    f(AppCtx::app_theme())
  }

  /// Get the widget back of `id`, panic if not exist.
  pub(crate) fn assert_get(&self, id: WidgetId) -> &dyn Render { id.assert_get(&self.tree.arena) }

  pub(crate) fn assert_get_mut(&self, id: WidgetId) -> &mut Box<dyn Render> {
    id.assert_get_mut(&mut self.force_as_mut().tree.arena)
  }

  pub(crate) fn alloc_widget(&self, widget: Box<dyn Render>) -> WidgetId {
    let arena = &mut self.force_as_mut().tree.arena;
    new_node(arena, widget)
  }

  pub(crate) fn append_child(&self, parent: WidgetId, child: WidgetId) {
    let arena = &mut self.force_as_mut().tree.arena;
    parent.append(child, arena);
  }

  #[inline]
  pub(crate) fn push_theme(&self, theme: Rc<Theme>) { self.themes().push(theme); }

  #[inline]
  pub(crate) fn pop_theme(&self) { self.themes().pop(); }

  /// todo: tmp code
  /// because we use `BuildCtx` as reference now, but we need to use it as
  /// mutable reference. Do a unsafe cast here, and remove it when we use
  /// `BuildCtx` as mutable reference in `Widget`
  #[allow(clippy::mut_from_ref)]
  pub(crate) fn force_as_mut(&self) -> &mut Self {
    #[allow(clippy::cast_ref_to_mut)]
    unsafe {
      &mut *(self as *const Self as *mut Self)
    }
  }

  fn themes(&self) -> &mut Vec<Rc<Theme>> {
    let this = self.force_as_mut();
    this.themes.get_or_insert_with(|| {
      let mut themes = vec![];
      let Some(p) = self.ctx_from else { return themes };

      let arena = &mut this.tree.arena;
      p.ancestors(arena).any(|p| {
        p.assert_get(arena).query_all_type(
          |t: &Rc<Theme>| {
            themes.push(t.clone());
            matches!(t.deref(), Theme::Inherit(_))
          },
          QueryOrder::InnerFirst,
        );
        matches!(themes.last().map(Rc::deref), Some(Theme::Full(_)))
      });
      themes
    })
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
              FnWidget::new(move |ctx: &BuildCtx| {
                no_watch!(*themes) = ctx.themes().clone();
                Void
              })
            }
          }
        }
      }
    };

    let wnd = TestWindow::new(light_dark);
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
