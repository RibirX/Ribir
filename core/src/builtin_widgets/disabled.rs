use crate::prelude::*;

class_names! {
  #[doc = "Class name for the Disabled"]
  DISABLED,
}

/// Disabled Widget
///
/// When a widget is disabled, it will no longer receive keyboard or pointer
/// events. Disabling a widget can be easily achieved by setting the built-in
/// disabled property to true. To customize the disabled appearance, you can
/// implement a dedicated Disabled class to override the default styling.
///
/// This is a builtin field of FatObj. You can simply set the `disabled` field
/// to attach a Disabled widget to the host widget.
///
/// # Example
///
/// ```rust
/// use ribir::prelude::*;
///
/// fn_widget! {
///   @Text {
///     text: "You can't click me",
///     disabled: true,
///     on_tap: |_: &mut PointerEvent| println!("Click!"),
///   }
/// };
/// ```
#[derive(Clone, Default)]
pub struct Disabled {
  pub disabled: bool,
}

impl Declare for Disabled {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl<'c> ComposeChild<'c> for Disabled {
  type Child = Widget<'c>;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    fn_widget! {
      let mut child = FatObj::new(child);
      @FocusScope {
        skip_descendants: pipe!($read(this).disabled()),
        skip_host: pipe!($read(this).disabled()),
        @IgnorePointer {
          ignore: pipe! {
            if $read(this).disabled() { IgnoreScope::Subtree } else { IgnoreScope::None }
          },
          @(child) { class: pipe!($read(this).disabled().then_some(DISABLED)) }
        }
      }
    }
    .into_widget()
  }
}

impl Disabled {
  fn disabled(&self) -> bool { self.disabled }
}
