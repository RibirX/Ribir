use super::*;

/// A map can store any type of config, useful for widget which the common
/// information of theme mod not enough and need have itself theme.
#[derive(Default)]
pub struct CustomStyles {
  themes: HashMap<TypeId, Box<dyn Any + Send>, ahash::RandomState>,
}

/// macro use to batch set custom theme.
#[macro_export]
macro_rules! fill_custom_style {
  ($theme: ident, $($v: expr),+) => {
    $($theme.custom_styles.set_custom_style($($v)+);)+
  };
}

pub trait CustomStyle: Sized + Clone + 'static {
  fn default_style(ctx: &impl AsRef<ProviderCtx>) -> Self;

  #[inline]
  fn of(ctx: &impl AsRef<ProviderCtx>) -> Self {
    let tid = TypeId::of::<Self>();
    Provider::of::<CustomStyles>(ctx)
      .and_then(|t| {
        t.themes
          .get(&tid)
          .and_then(|c| c.downcast_ref::<Self>())
          .cloned()
      })
      .unwrap_or_else(|| Self::default_style(ctx))
  }
}

// impl CustomStyles {
//   #[inline]
//   pub fn set_custom_style<T: CustomStyle + 'static>(&mut self, v: T) {
//     self.themes.insert(v.type_id(), Box::new(v));
//   }
// }
