use std::{collections::HashMap, rc::Rc, time::Duration};

use crate::{
  fill_transition,
  prelude::{easing, BuildCtx, Roc, Transition},
};

#[derive(Clone)]
pub struct TransitionTheme {
  pub default: Rc<Box<dyn Roc>>,
  pub transitions: HashMap<TransitionIdent, Rc<Box<dyn Roc>>, ahash::RandomState>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct TransitionIdent(usize);

pub mod transitions {
  use super::*;

  /// macro use to define a dozen of [`TransitionIdent`]! of icons.
  #[macro_export]
  macro_rules! define_transition_ident {
    ($from: ident, $define: ident, $($ident: ident),+) => {
      define_transition_ident!($from, $define);
      define_transition_ident!($define, $($ident), +);
    };
    ($value: ident, $define: ident) => {
      pub const $define: TransitionIdent = $value;
    }
  }

  /// macro use to specify transition for [`TransitionIdent`]!/
  #[macro_export]
  macro_rules! fill_transition {
      ($transitions: ident, $($name: path: $expr: expr),+) => {
        $($transitions.set_transition($name,  Rc::new(Box::new($expr)));)+
      };
    }

  pub const BEGIN: TransitionIdent = TransitionIdent::new(0);
  define_transition_ident!(
    BEGIN,
    EASE,
    LINEAR,
    EASE_IN,
    EASE_OUT,
    EASE_IN_OUT,
    SMOOTH_SCROLL,
    THEME_EXTEND
  );

  /// The user custom icon identify define start from.
  pub const CUSTOM_START: TransitionIdent = TransitionIdent::new(65536);
}

impl TransitionTheme {
  #[inline]
  pub fn set_transition(
    &mut self,
    ident: TransitionIdent,
    transition: Rc<Box<dyn Roc>>,
  ) -> Option<Rc<Box<dyn Roc>>> {
    self.transitions.insert(ident, transition)
  }
}

impl TransitionIdent {
  pub const fn new(idx: usize) -> Self { Self(idx) }

  /// get the svg icon of the ident from the context if it have otherwise return
  /// a default icon.
  pub fn get_from_or_default(self, ctx: &mut BuildCtx) -> Rc<Box<dyn Roc>> {
    self.get_from(ctx).unwrap_or_else(|| todo!())
  }

  /// get the svg icon of the ident from the context if it have.
  pub fn get_from(self, ctx: &mut BuildCtx) -> Option<Rc<Box<dyn Roc>>> {
    ctx
      .theme()
      .transitions_theme
      .transitions
      .get(&self)
      .cloned()
  }
}

impl Default for TransitionTheme {
  fn default() -> Self {
    let mut theme = Self {
      default: Rc::new(Box::new(Transition {
        delay: None,
        duration: Duration::from_millis(200),
        easing: easing::EASE,
        repeat: None,
      })),
      transitions: Default::default(),
    };
    fill_transition! { theme,
      transitions::EASE: Transition {
        delay: None,
        duration: Duration::from_millis(250),
        easing: easing::EASE,
        repeat: None,
      },
      transitions::LINEAR: Transition {
        delay: None,
        duration: Duration::from_millis(200),
        easing: easing::LINEAR,
        repeat: None,
      },
      transitions::EASE_IN: Transition {
        delay: None,
        duration: Duration::from_millis(250),
        easing: easing::EASE_IN,
        repeat: None,
      },
      transitions::EASE_OUT: Transition {
        delay: None,
        duration: Duration::from_millis(200),
        easing: easing::EASE_OUT,
        repeat: None,
      },
      transitions::EASE_IN_OUT: Transition {
        delay: None,
        duration: Duration::from_millis(250),
        easing: easing::EASE_IN_OUT,
        repeat: None,
      },
      transitions::SMOOTH_SCROLL:Transition {
        delay: None,
        duration: Duration::from_millis(200),
        easing: easing::EASE_IN,
        repeat: None,
      }
    }

    theme
  }
}
