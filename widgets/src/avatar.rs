use ribir_core::prelude::*;

use crate::prelude::*;

/// Represents an avatar widget that can display either text content or a custom
/// child widget.
///
/// # Usage Example:
/// ```rust
/// use ribir::prelude::*;
///
/// // Display an avatar with text content
/// let text_avatar = avatar! { @{"A"} };
///
/// // Display an avatar with a custom widget (e.g., an image)
/// let widget_avatar = avatar! { @Container { size: Size::splat(100.), background: Color::RED } };
/// ```
#[derive(Declare, Clone)]
pub struct Avatar;

class_names! {
    /// Root container class for avatars displaying text content.
    AVATAR_LABEL_CONTAINER,
    /// Root container class for avatars displaying custom widgets.
    AVATAR_WIDGET_CONTAINER,
    /// Text content class within avatars displaying text.
    AVATAR_LABEL,
    /// Wrapper class for custom widget content within avatars.
    AVATAR_WIDGET,
}

/// Defines the possible content types for an `Avatar` widget.
#[derive(Template)]
pub enum AvatarChild<'c> {
  /// Text display variant (e.g. user initials)
  Label(TextValue),
  /// Custom widget variant (e.g. profile image)
  Widget(Widget<'c>),
}

impl<'c> ComposeChild<'c> for Avatar {
  type Child = AvatarChild<'c>;

  fn compose_child(_: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    container! {
      class: child.container_class(),
      clip_boundary: true,
      size: Size::splat(40.),
      @ { child.wrap_with_class() }
    }
    .into_widget()
  }
}

impl<'w> AvatarChild<'w> {
  fn wrap_with_class(self) -> Widget<'w> {
    match self {
      AvatarChild::Label(text) => text! { text, class: AVATAR_LABEL }.into_widget(),
      AvatarChild::Widget(w) => class! {
        class: Some(AVATAR_WIDGET),
        box_fit: BoxFit::Contain,
        @ { w }
      }
      .into_widget(),
    }
  }

  fn container_class(&self) -> ClassName {
    match self {
      AvatarChild::Label(_) => AVATAR_LABEL_CONTAINER,
      AvatarChild::Widget(_) => AVATAR_WIDGET_CONTAINER,
    }
  }
}

#[cfg(test)]
mod tests {
  use ribir_core::test_helper::*;
  use ribir_dev_helper::*;

  use super::*;

  widget_image_tests! {
    label_avatar,
    WidgetTester::new(avatar!{
      h_align: HAlign::Center,
      v_align: VAlign::Center,
      @{"A"}
    }).with_wnd_size(Size::splat(64.))
  }

  widget_image_tests! {
    widget_avatar,
    WidgetTester::new(avatar!{
      h_align: HAlign::Center,
      v_align: VAlign::Center,
      @MockBox { size: Size::splat(100.), background: Color::RED }
    }).with_wnd_size(Size::splat(64.))
  }
}
