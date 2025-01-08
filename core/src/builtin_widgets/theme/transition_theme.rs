use super::*;
use crate::fill_transition;

pub struct TransitionTheme {
  pub transitions: ahash::HashMap<TransitionIdent, Box<dyn RocBoxClone>>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct TransitionIdent(usize);

pub mod transitions {
  use super::*;

  /// macro use to define a dozen of [`TransitionIdent`]! of icons.
  #[macro_export]
  macro_rules! define_transition_ident {
    ($from: expr, $define: ident, $($ident: ident),+) => {
      define_transition_ident!($from, $define);
      define_transition_ident!(TransitionIdent($define.0 + 1), $($ident), +);
    };
    ($value: expr, $define: ident) => {
      pub const $define: TransitionIdent = $value;
    }
  }

  /// macro use to specify transition for [`TransitionIdent`]!/
  #[macro_export]
  macro_rules! fill_transition {
      ($transitions: ident, $($name: path: $expr: expr),+) => {
        $($transitions.set_transition($name, (Box::new($expr)));)+
      };
    }

  pub const BEGIN: TransitionIdent = TransitionIdent::new(0);
  define_transition_ident!(BEGIN, EASE, LINEAR, EASE_IN, EASE_OUT, EASE_IN_OUT, THEME_EXTEND);

  /// The user custom icon identify define start from.
  pub const CUSTOM_START: TransitionIdent = TransitionIdent::new(65536);
}

impl TransitionTheme {
  #[inline]
  pub fn set_transition(
    &mut self, ident: TransitionIdent, transition: Box<dyn RocBoxClone>,
  ) -> Option<Box<dyn RocBoxClone>> {
    self.transitions.insert(ident, transition)
  }
}

impl TransitionTheme {
  /// Retrieve the nearest `TransitionTheme` from the context among its
  /// ancestors
  #[inline]
  pub fn of(ctx: &impl AsRef<ProviderCtx>) -> QueryRef<Self> {
    // At least one application theme exists
    Provider::of::<Self>(ctx).unwrap()
  }

  /// Retrieve the nearest `TransitionTheme` from the context among its
  /// ancestors and return a write reference to the theme.
  #[inline]
  pub fn write_of(ctx: &impl AsRef<ProviderCtx>) -> WriteRef<Self> {
    // At least one application theme exists
    Provider::write_of::<Self>(ctx).unwrap()
  }
}

impl TransitionIdent {
  pub const fn new(idx: usize) -> Self { Self(idx) }

  /// get transition of the transition identify from the context if it have or
  /// return linear transition.
  pub fn of(self, ctx: &impl AsRef<ProviderCtx>) -> Box<dyn Transition> {
    let panic_msg = format!(
      "Neither `Transition({:?})` nor `transitions::LINEAR` are initialized in all \
       `TransitionTheme` instances.",
      self
    );
    self
      .find(ctx)
      .or(transitions::LINEAR.find(ctx))
      .expect(&panic_msg)
  }

  pub fn find(self, ctx: &impl AsRef<ProviderCtx>) -> Option<Box<dyn Transition>> {
    Provider::of::<TransitionTheme>(ctx)
      .and_then(|t| t.transitions.get(&self).map(|t| t.box_clone()))
  }
}

impl Default for TransitionTheme {
  fn default() -> Self {
    let mut theme = Self { transitions: Default::default() };
    fill_transition! { theme,
      transitions::EASE: EasingTransition {
        duration: Duration::from_millis(250),
        easing: easing::EASE,
      },
      transitions::LINEAR: EasingTransition {
        duration: Duration::from_millis(200),
        easing: easing::LINEAR,
      },
      transitions::EASE_IN: EasingTransition {
        duration: Duration::from_millis(250),
        easing: easing::EASE_IN,
      },
      transitions::EASE_OUT: EasingTransition {
        duration: Duration::from_millis(200),
        easing: easing::EASE_OUT,
      },
      transitions::EASE_IN_OUT: EasingTransition {
        duration: Duration::from_millis(250),
        easing: easing::EASE_IN_OUT,
      }
    }

    theme
  }
}
