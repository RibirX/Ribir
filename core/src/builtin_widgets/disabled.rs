use crate::prelude::*;

class_names! {
  #[doc = "Class name for the Disabled"]
  DISABLED,
}

/// A widget wrapper that marks its subtree as disabled.
///
/// When disabled, a widget and its descendants do not receive keyboard or
/// pointer events. Toggle the built-in `disabled` field to apply this
/// behavior. To change the visual appearance, provide a custom `Disabled`
/// style class.
///
/// This is a built-in `FatObj` field. Setting `disabled` attaches a
/// `Disabled` wrapper to the host.
///
/// # Example
///
/// Disable a text widget so it cannot be clicked.
///
/// ```rust
/// use ribir::prelude::*;
///
/// text! {
///   text: "You can't click me",
///   disabled: true,
///   on_tap: |_: &mut PointerEvent| println!("Click!"),
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
