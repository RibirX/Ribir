use crate::{impl_query_self_only, prelude::*};
use std::{
  cell::Cell,
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

#[derive(Clone, Debug, PartialEq, Copy)]
pub struct KeyChange<V>(pub Option<V>, pub V);

impl<V: Default> Default for KeyChange<V> {
  fn default() -> Self { KeyChange(None, V::default()) }
}

/// A widget that can be used to track if the widget is the same widget in two
/// frames by its key. If two widget has same parent and key in two frames, the
/// new widget in the next frame will be treated as the same widget in the last
/// frame.
#[derive(Declare, Declare2)]
pub struct KeyWidget<V: Default + 'static = ()> {
  #[declare(convert=into)]
  pub key: Key,
  #[declare(default, strict)]
  pub value: V,
  #[declare(skip)]
  before_value: Option<V>,
  #[declare(skip)]
  has_successor: Cell<bool>,
}

/// A trait for `keyWidget` that use to record information of the previous and
/// next key widget.
pub(crate) trait AnyKey: Any {
  fn key(&self) -> Key;
  /// Record the previous KeyWidget associated with the same key.
  fn record_prev_key_widget(&self, key: &dyn AnyKey);
  /// Record the next KeyWidget associated with the same key.
  fn record_next_key_widget(&self, key: &dyn AnyKey);
  fn as_any(&self) -> &dyn Any;
}

impl<V> AnyKey for State<KeyWidget<V>>
where
  V: Default + Clone + PartialEq,
  Self: Any,
{
  fn key(&self) -> Key {
    match self {
      State::Stateless(this) => this.key.clone(),
      State::Stateful(this) => this.state_ref().key.clone(),
    }
  }

  fn record_prev_key_widget(&self, key: &dyn AnyKey) {
    assert_eq!(self.key(), key.key());
    let Some(key) = key.as_any().downcast_ref::<Self>() else {
      log::warn!("Different value type for same key.");
      return;
    };
    match self {
      // stateless key widget needn't record before value.
      State::Stateless(_) => {}
      State::Stateful(this) => match key {
        State::Stateless(key) => this.state_ref().record_before_value(key.value.clone()),
        State::Stateful(key) => this
          .state_ref()
          .record_before_value(key.state_ref().value.clone()),
      },
    }
  }

  fn record_next_key_widget(&self, _: &dyn AnyKey) { self.as_ref().has_successor.set(true); }

  fn as_any(&self) -> &dyn Any { self }
}

impl<V: 'static + Default + Clone + PartialEq> ComposeChild for KeyWidget<V> {
  type Child = Widget;
  #[inline]
  fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
    let data: Box<dyn AnyKey> = Box::new(this);
    DataWidget::attach(child, data)
  }
}

impl_query_self_only!(Box<dyn AnyKey>);

impl<V> KeyWidget<V>
where
  V: Default + Clone + PartialEq,
{
  /// Detect if the key widget is a new widget, there is not predecessor widget
  /// that has same key. Usually used in `on_mounted` callback.
  pub fn is_enter(&self) -> bool { self.before_value.is_none() }
  /// Detect if the key widget is really be disposed, there is not successor
  /// widget has same key. Usually used in `on_disposed` callback.
  pub fn is_leave(&self) -> bool { !self.has_successor.get() }

  /// Detect if the value of the key widget is changed
  pub fn is_changed(&self) -> bool {
    self.before_value.is_some() && self.before_value.as_ref() != Some(&self.value)
  }

  pub fn get_change(&self) -> KeyChange<V> {
    KeyChange(self.before_value.clone(), self.value.clone())
  }

  pub fn before_value(&self) -> Option<&V> { self.before_value.as_ref() }

  fn record_before_value(&mut self, value: V) { self.before_value = Some(value); }
}

impl_query_self_only!(Key);

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
