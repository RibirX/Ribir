use std::cell::Cell;

use ribir_core::{impl_compose_child_for_wrap_render, prelude::*, wrap_render::WrapRender};

/// An widget represents an icon.
///
/// The icon size is determined by the text line height, so you can use
/// `text_line_height` to change the icon size. Use the text line height to
/// determine the icon size make it easier to match with the label beside it.
///
/// This widget accept either text or another widget as its child. If the child
/// is text, the widget uses the icon font ligature to display the text as an
/// icon. The icon font is specified in the `Theme` by `icon_font` property,
/// therefore if you want text as an icon, you need to set and load the icon
/// font before running the app.
///
///
/// # Example
///
/// ```
/// use ribir_core::prelude::*;
/// use ribir_widgets::prelude::*;
///
/// // To use an icon font, set the icon font before running the app.
/// let mut theme = AppCtx::app_theme().write();
/// theme
///   .font_files
///   .push("the font file path".to_string());
/// theme.icon_font = IconFont(FontFace {
///   families: Box::new([FontFamily::Name("Your icon font family name".into())]),
///   // The rest of the face configuration depends on your font file
///   ..<_>::default()
/// });
///
/// // Using a named SVG as an icon
/// let _icon = icon! { @ { svg_registry::get_or_default("delete") } };
/// // Using a font icon
/// let _icon = icon! { @ { "search" } };
/// // Using any widget you want
/// let _icon = icon! {
///   @Container {
///     size: Size::new(200., 200.),
///     background: Color::RED,
///   }
/// };
/// ```
///
/// To specify the icon size, you can use the `text_line_height` property.
///
/// ```
/// use ribir_core::prelude::*;
/// use ribir_widgets::prelude::*;
///
/// let _icon = icon! {
///   text_line_height: 64.,
///   @ { svg_registry::get_or_default("search") }
/// };
/// ```
#[derive(Declare, Default, Clone, Copy)]
pub struct Icon;

#[derive(Template)]
pub enum IconChild<'c> {
  /// The text to display as a icon.
  ///
  /// Use a `DeclareInit<CowArc<str>>` so that we can accept a pipe text.
  FontIcon(TextValue),
  Widget(Widget<'c>),
}

impl<'c> ComposeChild<'c> for Icon {
  type Child = IconChild<'c>;
  fn compose_child(_: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    child.into_icon_widget()
  }
}

struct IconText;
impl_compose_child_for_wrap_render!(IconText);

impl WrapRender for IconText {
  fn perform_layout(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx) -> Size {
    let font_face = Provider::of::<IconFont>(&ctx).unwrap().0.clone();
    let mut style = Provider::of::<TextStyle>(ctx).unwrap().clone();
    style.font_face = font_face;
    style.font_size = style.line_height;
    let mut style = Provider::new(style);
    style.setup(ctx.as_mut());
    let size = host.perform_layout(clamp, ctx);
    style.restore(ctx.as_mut());
    size
  }

  #[inline]
  fn wrapper_dirty_phase(&self) -> DirtyPhase { DirtyPhase::Layout }
}

#[derive(SingleChild)]
struct IconRender {
  scale: Cell<f32>,
}

impl Render for IconRender {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let icon_size = Provider::of::<TextStyle>(ctx)
      .unwrap()
      .line_height;
    let child_size = ctx
      .perform_single_child_layout(BoxClamp::default())
      .unwrap_or_default();
    let scale = icon_size / child_size.width.max(child_size.height);
    self.scale.set(scale);
    clamp.clamp(Size::splat(icon_size))
  }

  fn visual_box(&self, ctx: &mut VisualCtx) -> Option<Rect> {
    Some(Rect::from_size(ctx.box_size()?))
  }

  fn paint(&self, ctx: &mut PaintingCtx) {
    let child_size = ctx.single_child_box().unwrap().size;
    if !child_size.is_empty() {
      let size = ctx.box_size().unwrap();
      let painter = ctx.painter();
      let scale = self.scale.get();
      let real_size = child_size * scale;
      if real_size.greater_than(size).any() {
        painter.clip(Path::rect(&Rect::from_size(size)).into());
      }
      let offset = (size - real_size) / 2.0;
      painter
        .translate(offset.width, offset.height)
        .scale(scale, scale);
    }
  }

  fn get_transform(&self) -> Option<Transform> {
    let scale = self.scale.get();
    Some(Transform::scale(scale, scale))
  }

  fn size_affected_by_child(&self) -> bool { false }
}

impl<'c> IconChild<'c> {
  fn into_icon_widget(self) -> Widget<'c> {
    let child = match self {
      IconChild::FontIcon(text) => IconText.with_child(text! { text }).into_widget(),
      IconChild::Widget(child) => child,
    };

    IconRender { scale: Cell::new(0.) }
      .with_child(child)
      .into_widget()
  }
}

#[cfg(test)]
mod tests {
  use ribir_core::test_helper::*;
  use ribir_dev_helper::*;

  use super::*;
  use crate::prelude::*;

  widget_image_tests!(
    icons,
    WidgetTester::new(row! {
      text_line_height: 24.,
      @Icon {
        foreground: Color::BLUE,
        @ { svg_registry::get_or_default("delete") }
      }
      @Icon {
        foreground: Color::RED,
        @ { "search" }
      }
      @Icon { @SpinnerProgress { value: Some(0.8) }}
      @Icon {
        background: Color::RED,
        clamp: BoxClamp::fixed_size(Size::splat(48.)),
        @ { "search" }
      }
    })
    .with_wnd_size(Size::new(128., 64.))
    .with_env_init(|| {
      let mut theme = AppCtx::app_theme().write();
      // Specify the icon font.
      theme
        .font_bytes
        .push(include_bytes!("../../fonts/material-search.ttf").to_vec());
      theme.icon_font = IconFont(FontFace {
        families: Box::new([FontFamily::Name("Material Symbols Rounded 48pt".into())]),
        weight: FontWeight::NORMAL,
        ..<_>::default()
      });
    })
    .with_comparison(0.002)
  );

  widget_image_tests!(
    keep_icon_visual,
    WidgetTester::new(container! {
      size: Size::splat(24.),
      @Icon {
        foreground: Color::RED,
        text_line_height: 48.,
        @ { svg_registry::get_or_default("") }
      }
    })
    .with_wnd_size(Size::splat(64.))
    .with_comparison(0.0002)
  );
}
