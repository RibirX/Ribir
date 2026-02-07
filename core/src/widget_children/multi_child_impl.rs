use super::*;

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

/// Use [`IntoMultiChild`] to explicitly convert a parent into an `XMultiChild`
/// when needed.
// ------ Widget Iterator Conversions ------
impl<'w, I, K> IntoWidgetIter<'w, dyn Iterator<Item = K>> for I
where
  I: IntoIterator<Item: IntoWidget<'w, K>>,
{
  fn into_widget_iter(self) -> impl Iterator<Item = Widget<'w>> {
    self.into_iter().map(IntoWidget::into_widget)
  }
}

impl<P, K> IntoWidgetIter<'static, Pipe<fn() -> [K]>> for Pipe<P>
where
  P: IntoIterator<Item: IntoWidget<'static, K>> + 'static,
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

impl<P: IntoMultiChild<'static>> MultiChild for Pipe<P> {}

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

impl<'p, P> std::ops::Deref for MultiPair<'p, P> {
  type Target = P;
  fn deref(&self) -> &Self::Target { &self.parent }
}

impl<'p, P> std::ops::DerefMut for MultiPair<'p, P> {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.parent }
}
