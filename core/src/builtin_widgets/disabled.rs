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
/// # Example
///
/// ``` no_run
/// use ribir::prelude::*;
///
/// let w = button! {
///     on_tap: move |_| panic!("you can't trigger me"),
///     disabled: true,
///     @ { "disabled" }
/// };
/// App::run(w);
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
      let child = FatObj::new(child);
      @FocusScope {
        skip_descendants: pipe!($this.disabled()),
        skip_host: pipe!($this.disabled()),
        @ IgnorePointer {
          ignore: pipe!($this.disabled()).map(
            |v| if v { IgnoreScope::Subtree } else { IgnoreScope::None }
          ),
          @ $child{ class: pipe!($this.disabled()).map(|v| v.then_some(DISABLED)) }
        }
      }
    }
    .into_widget()
  }
}

impl Disabled {
  fn disabled(&self) -> bool { self.disabled }
}
