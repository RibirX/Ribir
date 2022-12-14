use crate::{data_widget::widget_attach_data, impl_query_self_only, prelude::*};
use std::{
  cmp::{Eq, Ord, PartialOrd},
  fmt::Debug,
};

/// `Key` help `Ribir` to track if two widget is a same widget in two frames.
/// Abstract all builtin key into a same type.
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum Key {
  Kusize(usize),
  Ku1(u8),
  Ku2(u16),
  Ku4(u32),
  Ku8(u64),
  Ku16(u128),

  Kisize(isize),
  Ki1(i8),
  Ki2(i16),
  Ki4(i32),
  Ki8(i64),
  Ki16(i128),

  Kbool(bool),
  Kchar(char),

  Kstring(String),
  K32([u8; 32]),
}

/// The KeyStatus is the status of the Key, not the status of the KeyWidget.
/// The change of Key status is determined by its associated KeyWidget state
/// and its own state.
///
/// The KeyStatus has four status:
/// The first status: The KeyWidget associated with the Key was constructed, the
/// Key didn't exist in DynWidget Key List, the key status is `KeyStatus::Init`.
///
/// The second status: The KeyWidget associated with the Key was mounted, and
/// now its status is `KeyStatus::Init`, the key status will be changed
/// `KeyStatus::Mounted`.
///
/// The third status: The KeyWidget associated with the Key was disposed, and
/// the same key has anther associated KeyWidget was mounted, the key status
/// will be changed `KeyStatus::Updated`.
///
/// The last status: The KeyWidget associated with the Key was disposed, the
/// same key don't has anther associated KeyWidget, the key status will be
/// changed `KeyStatus::Disposed`.
#[derive(PartialEq, Debug)]
pub enum KeyStatus {
  Init,
  Mounted,
  Updated,
  Disposed,
}

impl Default for KeyStatus {
  fn default() -> Self { Self::Init }
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub struct KeyChange<V>(pub Option<V>, pub Option<V>);

impl<V> Default for KeyChange<V> {
  fn default() -> Self { KeyChange(None, None) }
}
#[derive(Declare)]
pub struct KeyWidget<V = ()> {
  #[declare(convert=into)]
  pub key: Key,
  pub value: Option<V>,
  #[declare(default)]
  before_value: Option<V>,
  #[declare(default)]
  status: KeyStatus,
}

pub(crate) trait AnyKey: Any {
  fn key(&self) -> Key;
  fn record_before_value(&self, key: &dyn AnyKey);
  fn disposed(&self);
  fn mounted(&self);
  fn as_any(&self) -> &dyn Any;
}

impl<V> AnyKey for StateWidget<KeyWidget<V>>
where
  V: Clone + PartialEq,
  Self: Any,
{
  fn key(&self) -> Key {
    match self {
      StateWidget::Stateless(this) => this.key.clone(),
      StateWidget::Stateful(this) => this.state_ref().key.clone(),
    }
  }

  fn record_before_value(&self, key: &dyn AnyKey) {
    assert_eq!(self.key(), key.key());
    let Some(key) = key.as_any().downcast_ref::<Self>() else {
      log::warn!("Different value type for same key.");
      return;
    };
    match self {
      // stateless key widget needn't record before value.
      StateWidget::Stateless(_) => {}
      StateWidget::Stateful(this) => match key {
        StateWidget::Stateless(key) => this.state_ref().record_before_value(key.value.clone()),
        StateWidget::Stateful(key) => this
          .state_ref()
          .record_before_value(key.state_ref().value.clone()),
      },
    }
  }

  fn disposed(&self) {
    match self {
      StateWidget::Stateless(_) => {}
      StateWidget::Stateful(this) => {
        this.state_ref().status = KeyStatus::Disposed;
      }
    }
  }

  fn mounted(&self) {
    match self {
      StateWidget::Stateless(_) => {}
      StateWidget::Stateful(this) => {
        this.state_ref().status = KeyStatus::Mounted;
      }
    }
  }

  fn as_any(&self) -> &dyn Any { self }
}

impl<V: 'static + Clone + PartialEq> ComposeChild for KeyWidget<V> {
  type Child = Widget;
  #[inline]
  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget {
    let data: Box<dyn AnyKey> = Box::new(this);
    widget_attach_data(child, data)
  }
}

impl Query for Box<dyn AnyKey> {
  impl_query_self_only!();
}

impl<V> KeyWidget<V>
where
  V: Clone + PartialEq,
{
  fn record_before_value(&mut self, value: Option<V>) {
    self.status = KeyStatus::Updated;
    self.before_value = value;
  }

  pub fn is_enter(&self) -> bool { self.status == KeyStatus::Mounted }

  pub fn is_modified(&self) -> bool { self.status == KeyStatus::Updated }

  pub fn is_changed(&self) -> bool { self.is_modified() && self.before_value != self.value }

  pub fn is_disposed(&self) -> bool { self.status == KeyStatus::Disposed }

  pub fn get_change(&self) -> KeyChange<V> {
    KeyChange(self.before_value.clone(), self.value.clone())
  }
}

impl Query for Key {
  impl_query_self_only!();
}

macro from_key_impl($($ty: ty : $name: ident)*) {
  $(
    impl From<$ty> for Key {
      fn from(s: $ty) -> Self {
        Key::$name(s)
      }
    }
  )*
}

from_key_impl!(
  usize:Kusize u8:Ku1 u16:Ku2 u32:Ku4 u64:Ku8 u128:Ku16
  isize:Kisize i8:Ki1 i16:Ki2 i32:Ki4 i64:Ki8 i128:Ki16
  bool:Kbool char:Kchar
  [u8;32]:K32
);

const MAX_KEY_STR: usize = 16;

impl From<String> for Key {
  fn from(s: String) -> Self {
    if s.len() < MAX_KEY_STR {
      Key::Kstring(s)
    } else {
      Key::K32(blake3::hash(s.as_bytes()).into())
    }
  }
}

impl From<&str> for Key {
  fn from(s: &str) -> Self {
    if s.len() < MAX_KEY_STR {
      Key::Kstring(s.to_owned())
    } else {
      Key::K32(blake3::hash(s.as_bytes()).into())
    }
  }
}

pub macro complex_key($($k: expr),*) {
  {
    let mut hasher = blake3::Hasher::new();
    $(
      $k.consume(&mut hasher);
    )*
    let bytes: [u8;32] = hasher.finalize().into();
    bytes
  }
}

trait ConsumeByHasher {
  fn consume(self, hasher: &mut blake3::Hasher);
}

impl ConsumeByHasher for String {
  #[inline]
  fn consume(self, hasher: &mut blake3::Hasher) { hasher.update(self.as_bytes()); }
}

impl<'a> ConsumeByHasher for &'a str {
  #[inline]
  fn consume(self, hasher: &mut blake3::Hasher) { hasher.update(self.as_bytes()); }
}

macro impl_as_u8_consume_by_hasher($($t: ty)*) {
  $(
    impl ConsumeByHasher for $t {
      #[inline]
      fn consume(self, hasher: &mut blake3::Hasher) {
        hasher.update(&[self as u8]);
      }
    }
  )*
}
impl_as_u8_consume_by_hasher!(bool char);

macro impl_bytes_consume_by_hasher($($ty: ty)*) {
  $(
    impl ConsumeByHasher for $ty {
      #[inline]
      fn consume(self, hasher: &mut blake3::Hasher) {
        hasher.update(&self.to_ne_bytes());
      }
    }
  )*
}

impl_bytes_consume_by_hasher!(
  usize u8 u16 u32 u64 u128
  isize i8 i16 i32 i64 i128
  f32 f64
);

#[test]
fn key_detect() {
  let k1: Key = 0i32.into();
  let k2: Key = String::new().into();
  let k3: Key = "".into();
  let ck1 = complex_key!("asd", true, 1);
  let ck2 = complex_key!("asd", true, 1);
  assert!(k1 != k2);
  assert!(k2 == k3);
  assert!(k3 != k1);
  assert!(ck1 == ck2);
}
