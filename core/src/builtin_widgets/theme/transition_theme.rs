use std::{collections::HashMap, time::Duration};

use super::Theme;
use crate::{
  animation::RocBoxClone,
  context::BuildCtx,
  fill_transition,
  prelude::{easing, EasingTransition, Transition},
};

pub struct TransitionTheme {
  pub transitions: HashMap<TransitionIdent, Box<dyn RocBoxClone>, ahash::RandomState>,
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

impl TransitionIdent {
  pub const fn new(idx: usize) -> Self { Self(idx) }

  /// get transition of the transition identify from the context if it have or
  /// return linear transition.
  pub fn of(self, ctx: &BuildCtx) -> Box<dyn Transition> {
    ctx
      .find_cfg(|t| match t {
        Theme::Full(t) => {
          let transitions = &t.transitions_theme.transitions;
          transitions.get(&self).or_else(|| {
            log::info!("Transition({:?}) not init in theme.", self);
            transitions.get(&transitions::LINEAR)
          })
        }
        Theme::Inherit(i) => i
          .transitions_theme
          .as_ref()
          .and_then(|t| t.transitions.get(&self)),
      })
      .unwrap()
      .box_clone()
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
