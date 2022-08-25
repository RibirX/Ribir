use crate::{
  impl_proxy_query, impl_query_self_only,
  prelude::{data_widget::compose_child_as_data_widget, *},
};
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

#[derive(Declare)]
pub struct KeyWidget {
  #[declare(builtin, convert=into)]
  pub key: Key,
}

impl ComposeSingleChild for KeyWidget {
  #[inline]
  fn compose_single_child(this: StateWidget<Self>, child: Widget, _: &mut BuildCtx) -> Widget {
    compose_child_as_data_widget(child, this)
  }
}

impl Query for KeyWidget {
  impl_proxy_query!(key);
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
  use crate::widget::*;
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
