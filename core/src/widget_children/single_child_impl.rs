use super::*;

/// Intermediate composition state holding parent-child relationship
///
/// Used during widget tree construction to:
/// - Maintain temporary parent/child association
/// - Enable incremental composition
/// - Support optional child patterns
pub struct SinglePair<'c, P> {
  pub(super) parent: P,
  pub(super) child: Option<Widget<'c>>,
}

// ------------------ SingleChild Trait Implementations ------------------

/// Enables optional parent components in widget hierarchies
impl<P: SingleChild> SingleChild for Option<P> {}

impl<P: Parent> Parent for Option<P> {
  /// Finalizes widget tree construction with child validation
  ///
  /// # Behavior
  /// - If parent exists: delegates to parent's composition logic
  /// - If parent missing: requires exactly one child widget
  ///
  /// # Panics
  /// When parent is None and child count != 1
  fn with_children<'w>(self, mut children: Vec<Widget<'w>>) -> Widget<'w>
  where
    Self: 'w,
  {
    if let Some(p) = self {
      p.with_children(children)
    } else {
      assert_eq!(children.len(), 1, "Either the parent or the child must exist.");
      children.pop().unwrap()
    }
  }
}

/// Core single-child implementations
impl<'p> SingleChild for XSingleChild<'p> {}
impl<T> SingleChild for T where T: StateReader<Value: SingleChild> {}
impl<T: SingleChild> SingleChild for FatObj<T> {}
impl<F: FnOnce() -> W, W: SingleChild> SingleChild for FnWidget<W, F> {}
impl<P: IntoSingleChild<'static>> SingleChild for Pipe<P> {}

// ------------------ Composition Conversions ------------------

/// Framework integration point for single-child components.
///
/// Use [`IntoSingleChild`] to explicitly convert a parent into an
/// `XSingleChild` when needed.

/// Final composition step converting parent-child pair to concrete widge
impl<'s: 'w, 'w, P> RFrom<SinglePair<'s, P>, OtherWidget<dyn Compose>> for Widget<'w>
where
  P: SingleChild + XParent + 'w,
{
  fn r_from(value: SinglePair<'s, P>) -> Self {
    let SinglePair { parent, child } = value;
    let children = child.map_or_else(Vec::new, |child| vec![child]);
    parent.x_with_children(children)
  }
}

/// Transparent parent access for composition pair
///
/// Enables direct method access to parent component while maintaining
/// child relationship through dereference patterns
impl<'c, P> std::ops::Deref for SinglePair<'c, P> {
  type Target = P;
  fn deref(&self) -> &Self::Target { &self.parent }
}

impl<'c, P> std::ops::DerefMut for SinglePair<'c, P> {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.parent }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_helper::MockBox;

  /// Verifies nested single-child composition patterns
  #[test]
  fn pair_with_child() {
    let mock_box = MockBox { size: ZERO_SIZE };
    let _ = |_: &BuildCtx| -> Widget {
      mock_box
        .clone()
        .with_child(mock_box.clone().with_child(mock_box))
        .into_widget()
    };
  }

  /// Ensures pipe-based optional widgets maintain single-child invariants
  #[test]
  fn fix_mock_box_compose_pipe_option_widget() {
    fn _x(w: Pipe<Option<BoxFnWidget<'static>>>) { MockBox { size: ZERO_SIZE }.with_child(w); }
  }
}
