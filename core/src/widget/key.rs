use crate::widget::*;
use blake3;

#[derive(Debug)]
pub struct Key<T>(T);

pub struct KeyDetect<T> {
  key: Key<T>,
  child: Widget,
}

impl<T> KeyDetect<T> {
  pub fn new<K>(key: K, child: Widget) -> Self
  where
    K: Into<Key<T>>,
  {
    KeyDetect {
      key: key.into(),
      child,
    }
  }
}

impl<T, Rhs> PartialEq<Key<Rhs>> for Key<T> {
  #[inline(always)]
  default fn eq(&self, _other: &Key<Rhs>) -> bool { false }
}

impl<T, Rhs> PartialEq<Key<Rhs>> for Key<T>
where
  T: PartialEq<Rhs>,
{
  #[inline(always)]
  fn eq(&self, other: &Key<Rhs>) -> bool { self.0 == other.0 }
}

macro from_key_impl($($ty: ty)*) {
  $(
    impl From<$ty> for Key<$ty> {
      #[inline]
      fn from(v: $ty) -> Self { Key(v) }
    }
  )*
}

from_key_impl!(
  ()
  usize u8 u16 u32 u64 u128
  isize i8 i16 i32 i64 i128
  f32 f64
  bool char
);

#[derive(Clone, PartialEq, Debug)]
pub enum StringKey {
  Str(String),
  HashStr([u8; blake3::OUT_LEN]),
}

const MAX_KEY_STR: usize = 64;

impl From<String> for Key<StringKey> {
  fn from(s: String) -> Self {
    let sk = if s.len() > MAX_KEY_STR {
      StringKey::Str(s)
    } else {
      StringKey::HashStr(blake3::hash(s.as_bytes()).into())
    };
    Key(sk)
  }
}

impl From<&str> for Key<StringKey> {
  fn from(s: &str) -> Self {
    let sk = if s.len() > MAX_KEY_STR {
      StringKey::Str(s.to_owned())
    } else {
      StringKey::HashStr(blake3::hash(s.as_bytes()).into())
    };
    Key(sk)
  }
}

pub macro complex_key($($k: expr),*) {
  {
    let mut hasher = blake3::Hasher::new();
    $(
      $k.consume(&mut hasher);
    )*
    let bytes: [u8;32] = hasher.finalize().into();
    Key(bytes)
  }
}

trait ConsumeByHasher {
  fn consume(self, hasher: &mut blake3::Hasher);
}

impl ConsumeByHasher for String {
  #[inline]
  fn consume(self, hasher: &mut blake3::Hasher) {
    hasher.update(self.as_bytes());
  }
}

impl<'a> ConsumeByHasher for &'a str {
  #[inline]
  fn consume(self, hasher: &mut blake3::Hasher) {
    hasher.update(self.as_bytes());
  }
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
  let k1 = KeyDetect::new(0, Text("").into());
  let k2 = KeyDetect::new(String::new(), Text("").into());
  let k3 = KeyDetect::new("", Text("").into());
  let ck1 = KeyDetect::new(complex_key!("asd", true, 1), Text("").into());
  let ck2 = KeyDetect::new(complex_key!("asd", true, 1), Text("").into());
  assert_ne!(k1.key, k2.key);
  assert_eq!(k2.key, k3.key);
  assert_ne!(k3.key, k1.key);
  assert_eq!(ck1.key, ck2.key);
}
