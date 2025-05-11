use crate::{pipe::InnerPipe, prelude::*, render_helper::PureRender};

/// Reciprocal conversion trait with explicit kind marker
///
/// This is the counterpart to [`RFrom`], representing the inverse conversion
/// relationship. Implementations should generally be done through [`RFrom`],
/// which will automatically provide implementations for this trait through a
/// blanket implementation.
///
/// The `Kind` type parameter acts as a "conversion context" that:
/// - Allows disambiguating between multiple possible conversions
/// - Carries type-level metadata about the conversion
/// - Enables specialization through different kind markers
pub trait RInto<Target, Kind: ?Sized> {
  fn r_into(self) -> Target;
}

/// Context-aware type conversion trait with kind-based specialization
///
/// This extends [`std::convert::From`] with additional type safety through the
/// `Kind` parameter, which serves three primary purposes:
/// 1. **Disambiguation**: Enables multiple conversions between the same types
///    through different kind markers
/// 2. **Type Safety**: Ensures conversions are only valid in specific contexts
///
/// # When to Use
/// - When you need multiple distinct conversions between the same types
/// - When conversions depend on specific context or implementation details
/// - When working with trait objects that require explicit conversion paths
///
/// # Automatic Implementation
/// The standard library's [`From`] implementations automatically provide:
/// ```rust,ignore
/// impl<T, U> RFrom<T, IntoKind> for U where U: From<T>
/// ```
/// allowing seamless interoperability with existing [`From`] implementations.
pub trait RFrom<Source, Kind: ?Sized> {
  fn r_from(from: Source) -> Self;
}

// ------------- Core Conversion Implementations  -------------

/// Standard conversion bridge between [`Into`] and [`RFrom`]
pub struct IntoKind;
impl<C, T> RFrom<C, IntoKind> for T
where
  C: Into<T>,
{
  #[inline]
  fn r_from(from: C) -> Self { from.into() }
}

// ------------- Template Building Conversions ---------------

/// Template composition implementation for builder patterns
impl<Builder, C, K: ?Sized> RFrom<C, TmlKind<K>> for Builder
where
  Builder: TemplateBuilder + ComposeWithChild<C, K, Target = Builder>,
{
  fn r_from(from: C) -> Self { Builder::default().with_child(from) }
}

// --------------- Pair Structure Conversions ----------------

/// Pair conversion handling for widget hierarchies
impl<'c, P, C, K> RFrom<Pair<P, C>, K> for Pair<P, Widget<'c>>
where
  C: IntoWidget<'c, K>,
  K: NotWidgetSelf,
{
  fn r_from(from: Pair<P, C>) -> Self {
    let (parent, child) = from.unzip();
    Pair::new(parent, child.into_widget())
  }
}

// Stateful pair conversion with composition tracking
impl<'c, W, C, K: ?Sized> RFrom<Pair<W, C>, K> for PairOf<'c, W>
where
  W: ComposeChild<'c, Child: RFrom<C, K>> + 'static,
{
  fn r_from(from: Pair<W, C>) -> Self {
    let (parent, child) = from.unzip();
    Self(FatObj::new(Pair::new(State::value(parent), child.r_into())))
  }
}

impl<'c, W, C, K: ?Sized> RFrom<Pair<State<W>, C>, K> for PairOf<'c, W>
where
  W: ComposeChild<'c, Child: RFrom<C, K>> + 'static,
{
  fn r_from(from: Pair<State<W>, C>) -> Self {
    let (parent, child) = from.unzip();
    Self(FatObj::new(Pair::new(parent, child.r_into())))
  }
}

impl<'c, W, C, K: ?Sized> RFrom<FatObj<Pair<State<W>, C>>, K> for PairOf<'c, W>
where
  W: ComposeChild<'c, Child: RFrom<C, K>> + 'static,
{
  fn r_from(from: FatObj<Pair<State<W>, C>>) -> Self {
    let pair = from.map(|p| {
      let (parent, child) = p.unzip();
      Pair::new(parent, child.r_into())
    });
    Self(pair)
  }
}

// -------------- Widget Conversion Markers ------------------
// widget conversion has three kinds markers
// - `IntoKind` only for the `Widget` self, that based on the standard `From`
//   implementation
// - `PipeOptionWidget` for pipe optional widget.
// - `OtherWidget` for any other widget type

/// Resolution marker for pipe optional widget ambiguity
///
/// Handles dual behavior cases for `MultiChild` with:
/// 1. **Direct Widget**: Treat pipe as single optional widget
/// 2. **Iterated Widgets**: Process pipe values as successive widgets
pub struct PipeOptionWidget<K: ?Sized>(PhantomData<fn() -> K>);

/// Default marker for non-specialized widget conversions
pub struct OtherWidget<K: ?Sized>(PhantomData<fn() -> K>);
pub(crate) trait NotWidgetSelf {}
impl<K: ?Sized> NotWidgetSelf for OtherWidget<K> {}
impl<K: ?Sized> NotWidgetSelf for PipeOptionWidget<K> {}

// --------------- Composition Implementations ---------------

// Base composition conversion for static components
impl<C: Compose + 'static> RFrom<C, OtherWidget<dyn Compose>> for Widget<'static> {
  fn r_from(widget: C) -> Self { Compose::compose(State::value(widget)) }
}

// State-aware composition conversion
impl<W: StateWriter<Value: Compose + Sized>>
  RFrom<W, OtherWidget<dyn StateWriter<Value = &dyn Compose>>> for Widget<'static>
{
  fn r_from(widget: W) -> Self { Compose::compose(widget) }
}

// Base render conversion
impl<R: Render + 'static> RFrom<R, OtherWidget<dyn Render>> for Widget<'static> {
  fn r_from(widget: R) -> Self { Widget::from_render(Box::new(PureRender(widget))) }
}

// State reader render proxy implementation
struct ReaderRender<T>(T);
impl<R: StateReader<Value: Render>> crate::render_helper::RenderProxy for ReaderRender<R> {
  #[inline(always)]
  fn proxy(&self) -> impl Deref<Target = impl Render + ?Sized> { self.0.read() }
}

macro_rules! impl_into_x_widget_for_state_reader {
  (<$($generics:ident $(: $bounds:ident)?),* > $ty:ty $(where $($t: tt)*)?) => {
    impl<$($generics $(:$bounds)?,)*> RFrom<$ty, OtherWidget<dyn Render>> for Widget<'static>
    $(where $($t)*)?
    {
      fn r_from(widget: $ty) -> Self {
        match widget.try_into_value() {
          Ok(value) => value.into_widget(),
          Err(s) => {
            ReaderRender(s).into_widget()
          },
        }
      }
    }
  };
}

macro_rules! impl_into_x_widget_for_state_watcher {
  (<$($generics:ident $(: $bounds:ident)?),* > $ty:ty $(where $($t: tt)*)?) => {
    impl<$($generics $(:$bounds)?,)*> RFrom<$ty, OtherWidget<dyn Render>> for Widget<'static>
    $(where $($t)*)?
    {
      fn r_from(widget: $ty) -> Self {
        match widget.try_into_value() {
          Ok(value) => value.into_widget(),
          Err(s) => {
            let modifies = s.raw_modifies();
            ReaderRender(s.clone_reader())
            .into_widget()
            .dirty_on(modifies, s.read().dirty_phase())
          },
        }
      }
    }
  };
}
impl_into_x_widget_for_state_reader!(<R: Render> Box<dyn StateReader<Value = R>>);
impl_into_x_widget_for_state_reader!(
  <O, M> MapReader<O, M>
  where MapReader<O, M>: StateReader<Value: Render + Sized>
);
impl_into_x_widget_for_state_watcher!(<R: Render> Stateful<R>);
impl_into_x_widget_for_state_watcher!(<R: Render> State<R>);
impl_into_x_widget_for_state_watcher!(
  <W, WM> MapWriter<W, WM>
  where MapWriter<W, WM>: StateWatcher<Value: Render + Sized>
);
impl_into_x_widget_for_state_watcher!(
  <O, M> SplittedWriter<O, M>
  where SplittedWriter<O, M>: StateWatcher<Value: Render + Sized>
);

// --- Function Kind ---
impl<'w, F, W, K> RFrom<F, OtherWidget<dyn FnOnce() -> K>> for Widget<'w>
where
  F: FnOnce() -> W + 'w,
  W: IntoWidget<'w, K> + 'w,
{
  #[inline]
  fn r_from(value: F) -> Self { Widget::from_fn(move |ctx| value().into_widget().call(ctx)) }
}

impl<'w, F, W, K> RFrom<FnWidget<W, F>, OtherWidget<dyn FnOnce() -> K>> for Widget<'w>
where
  F: FnOnce() -> W + 'w,
  W: IntoWidget<'w, K> + 'w,
{
  #[inline]
  fn r_from(value: FnWidget<W, F>) -> Self { value.0.into_widget() }
}

impl<F, W, K> RFrom<FnWidget<W, F>, dyn FnOnce() -> K> for GenWidget
where
  F: FnMut() -> W + 'static,
  W: IntoWidget<'static, K>,
{
  #[inline]
  fn r_from(value: FnWidget<W, F>) -> Self { GenWidget::from_fn_widget(value) }
}

impl<F, W, K> RFrom<F, dyn FnOnce() -> K> for GenWidget
where
  F: FnMut() -> W + 'static,
  W: IntoWidget<'static, K>,
{
  #[inline]
  fn r_from(value: F) -> Self { GenWidget::new(value) }
}

// -------------- Advanced Type Conversions ------------------

// Fat object conversion with nested widget transformation
impl<'w, T, K> RFrom<FatObj<T>, OtherWidget<FatObj<K>>> for Widget<'w>
where
  T: IntoWidget<'w, K>,
{
  fn r_from(value: FatObj<T>) -> Self { value.map(|w| w.into_widget()).compose() }
}

// Pipe-to-widget conversion for reactive streams
impl<P, K> RFrom<P, OtherWidget<dyn Pipe<Value = K>>> for Widget<'static>
where
  P: Pipe<Value: RInto<Widget<'static>, K>>,
{
  fn r_from(pipe: P) -> Self { pipe.build_single() }
}

impl<P, K, V> RFrom<P, PipeOptionWidget<K>> for Widget<'static>
where
  P: Pipe<Value = Option<V>>,
  V: RInto<Widget<'static>, K>,
{
  fn r_from(pipe: P) -> Self { pipe.build_single() }
}

// --------------- Blanket Implementation --------------------
impl<C, T, K: ?Sized> RInto<C, K> for T
where
  C: RFrom<T, K>,
{
  fn r_into(self) -> C { C::r_from(self) }
}
