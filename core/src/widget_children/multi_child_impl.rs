use super::*;

/// Type-erased container for widgets that manage multiple children.
///
/// Acts as a bridge between compile-time safety and runtime flexibility by:
/// - Enforcing multi-child constraints at type system level
/// - Allowing dynamic dispatch through [`BoxedParent`]
///
/// # Type Safety
/// Automatically derived via [`From`] for any type implementing:
/// - [`MultiChild`] (composition constraint)
/// - [`Parent`] (hierarchy capability)
pub struct XMultiChild<'p>(pub(crate) Box<dyn BoxedParent + 'p>);

/// Intermediate builder for constructing parent-child widget relationships.
///
/// Combines a parent widget with its collected children during composition.
/// Enables fluent chaining of child additions while preserving parent type
/// information until final widget construction.
///
/// # Type Parameters
/// - `P`: Parent widget type implementing multi-child management capabilities
///
/// # Composition Flow
/// 1. Start with base parent widget
/// 2. Chain `.with_child()` calls to add children
/// 3. Finalize conversion to [`Widget`] via framework traits
pub struct MultiPair<'a, P> {
  pub(super) parent: P,
  pub(super) children: Vec<Widget<'a>>,
}

impl<'p, P> MultiPair<'p, P> {
  /// Adds one or more child widgets to the current parent-child pair.
  ///
  /// # Lifetime Handling
  /// - `'c`: Lifetime of child widget sources
  /// - `'w`: Output widget lifetime (must outlive `'p` and `'c`)
  /// - Maintains parent ownership while extending child collection
  ///
  /// # Usage
  /// Accepts any type implementing [`IntoWidgetIter`], including:
  /// - Single widgets
  /// - Iterators of widgets
  /// - Reactive pipes producing widget collections
  pub fn with_child<'c: 'w, 'w, K: ?Sized>(
    self, child: impl IntoWidgetIter<'c, K>,
  ) -> MultiPair<'w, P>
  where
    'p: 'w,
  {
    let MultiPair { parent, mut children } = self;
    for c in child.into_widget_iter() {
      children.push(c);
    }
    MultiPair { parent, children }
  }
}

// ------ Core Type Conversions ------

/// Automatic conversion from any valid multi-child parent to XMultiChild
///
/// Enables seamless integration of custom multi-child widgets into the
/// framework's container system through trait-based coercion.
impl<'p, P> From<P> for XMultiChild<'p>
where
  P: Parent + MultiChild + 'p,
{
  fn from(value: P) -> Self { XMultiChild(Box::new(value)) }
}

// ------ Widget Iterator Conversions ------
impl<'w, I, K> IntoWidgetIter<'w, dyn Iterator<Item = K>> for I
where
  I: IntoIterator<Item: IntoWidget<'w, K>>,
{
  fn into_widget_iter(self) -> impl Iterator<Item = Widget<'w>> {
    self.into_iter().map(IntoWidget::into_widget)
  }
}

impl<P, K> IntoWidgetIter<'static, dyn Pipe<Value = [K]>> for P
where
  P: Pipe<Value: IntoIterator<Item: IntoWidget<'static, K>>>,
{
  fn into_widget_iter(self) -> impl Iterator<Item = Widget<'static>> {
    self.build_multi().into_iter()
  }
}

impl<'w, W: IntoWidget<'w, IntoKind>> IntoWidgetIter<'w, IntoKind> for W {
  fn into_widget_iter(self) -> impl Iterator<Item = Widget<'w>> {
    std::iter::once(self.into_widget())
  }
}

impl<'w, W, K: ?Sized> IntoWidgetIter<'w, OtherWidget<K>> for W
where
  W: IntoWidget<'w, OtherWidget<K>>,
{
  fn into_widget_iter(self) -> impl Iterator<Item = Widget<'w>> {
    std::iter::once(self.into_widget())
  }
}

// ------ MultiChild Implementations ------

impl<'p> MultiChild for XMultiChild<'p> {}

impl<T> MultiChild for T where T: StateReader<Value: MultiChild> {}

impl<P: MultiChild> MultiChild for FatObj<P> {}

impl<P: MultiChild, F: FnOnce() -> P> MultiChild for FnWidget<P, F> {}

/// Macro-generated implementations for reactive pipe types
///
/// Applies MultiChild trait to all pipe variants that carry
/// container-compatible values
macro_rules! impl_multi_child_for_pipe {
  (<$($generics:ident),*> , $pipe:ty) => {
    impl<$($generics),*> MultiChild for $pipe
    where
      $pipe: Pipe<Value: Into<XMultiChild<'static>>>,
    {}
  };
}
crate::pipe::iter_all_pipe_type_to_impl!(impl_multi_child_for_pipe);

// ------ Final Composition Conversions ------

/// Finalizes widget hierarchy construction from a MultiPair
///
/// This conversion consumes the accumulated parent-children pair and
/// invokes the parent's layout implementation to create the final widget.
impl<'w, 'c: 'w, P> RFrom<MultiPair<'c, P>, OtherWidget<dyn Compose>> for Widget<'w>
where
  P: MultiChild + XParent + 'w,
{
  fn r_from(value: MultiPair<'c, P>) -> Self {
    let MultiPair { parent, children } = value;
    parent.x_with_children(children)
  }
}

impl<'p> RFrom<XMultiChild<'p>, OtherWidget<dyn Compose>> for Widget<'p> {
  #[inline]
  fn r_from(value: XMultiChild<'p>) -> Self { value.0.boxed_with_children(vec![]) }
}

impl<'p, P> std::ops::Deref for MultiPair<'p, P> {
  type Target = P;
  fn deref(&self) -> &Self::Target { &self.parent }
}

impl<'p, P> std::ops::DerefMut for MultiPair<'p, P> {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.parent }
}
