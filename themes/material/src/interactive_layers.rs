use ribir_core::prelude::*;

use crate::{focus_indicator::*, ripple::*, state_layer::*};

/// A widget that provides material design interactive visual layers for its
/// child
///
/// # Features
///
/// - **Hover Layer**: Visual feedback on mouse hover
/// - **Focus Indicator**: Ring or layer for keyboard focus
/// - **Ripple Effect**: Animated touch/click feedback
/// - **Visual Hierarchy**: Maintains proper layer ordering and offsets
pub struct InteractiveLayers {
  ripple: Stateful<Ripple>,
  ring_outer_offset: f32,
}

/// Creates a widget function using `InteractiveLayers` as the root component
///
/// # Example
///
/// ```ignore
/// let my_widget = interactive_layers! {
///     on_tap: handle_click
///     @ { your_widget }
/// };
/// ```
#[macro_export]
macro_rules! interactive_layers {
  ($($content: tt)*) => {
      fn_widget! { @InteractiveLayers { $($content)* } }
  };
}

pub struct InteractiveLayersDeclarer {
  ripple: RippleDeclarer,
  ring_outer_offset: f32,
}

impl InteractiveLayersDeclarer {
  pub fn with_ring_outer_offset(&mut self, offset: f32) -> &mut Self {
    self.ring_outer_offset = offset;
    self
  }
}

impl<'c> ComposeChild<'c> for InteractiveLayers {
  type Child = Widget<'c>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    let mut child = FatObj::new(child);
    let hover_layer = StateLayer::created_for(LayerArea::FullContent, &mut child);
    let Self { ripple, ring_outer_offset } = this
      .try_into_value()
      .unwrap_or_else(|_| panic!("InteractiveLayers shouldn't be a stateless widget"));

    rdl! {
      @(ripple) {
        @(hover_layer) {
          @FocusIndicator {
            ring_outer_offset: ring_outer_offset,
            @ { child }
          }
        }
      }
    }
    .into_widget()
  }
}

impl Declare for InteractiveLayers {
  type Builder = InteractiveLayersDeclarer;

  #[inline]
  fn declarer() -> Self::Builder {
    InteractiveLayersDeclarer { ripple: Ripple::declarer(), ring_outer_offset: 0.0 }
  }
}

impl ObjDeclarer for InteractiveLayersDeclarer {
  type Target = FatObj<InteractiveLayers>;

  #[inline]
  fn finish(self) -> Self::Target {
    let Self { ripple, ring_outer_offset } = self;
    ripple
      .finish()
      .map(|ripple| InteractiveLayers { ripple, ring_outer_offset })
  }
}

impl std::ops::Deref for InteractiveLayersDeclarer {
  type Target = RippleDeclarer;
  fn deref(&self) -> &Self::Target { &self.ripple }
}

impl std::ops::DerefMut for InteractiveLayersDeclarer {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.ripple }
}
