use crate::render_object::RenderCtx;
use crate::widget::*;
use blake3;
use std::{any::Any, fmt::Debug};

pub trait Key: Debug {
  fn as_any(&self) -> &dyn Any;
  fn eq(&self, other: &dyn Key) -> bool;
}

impl PartialEq for Box<dyn Key> {
  fn eq(&self, other: &Box<dyn Key>) -> bool { Key::eq(&**self, &**other) }
}

pub struct KeyDetect {
  key: Box<dyn Key>,
  child: Widget,
}
#[derive(Debug)]
pub struct KeyRender;

impl KeyDetect {
  pub fn new<K>(key: K, child: Widget) -> Self
  where
    K: Into<Box<dyn Key>>,
  {
    KeyDetect {
      key: key.into(),
      child,
    }
  }

  #[inline(always)]
  pub fn key(&self) -> &Box<dyn Key> { &self.key }
}

impl<'a> SingleChildWidget<'a> for KeyDetect {
  fn split(self: Box<Self>) -> (Box<dyn for<'r> RenderWidget<'r>>, Widget) {
    (Box::new(self.key), self.child)
  }
}

impl<'a> RenderWidget<'a> for Box<dyn Key> {
  fn create_render_object(&self) -> Box<dyn RenderObject> {
    Box::new(KeyRender)
  }
}

impl RenderObject for KeyRender {
  fn paint(&self) {
    unimplemented!();
  }
  fn perform_layout(&mut self, _ctx: RenderCtx) {
    unimplemented!();
  }
}

macro from_key_impl($($ty: ty)*) {
  $(
    impl Key for $ty {
      #[inline(always)]
      fn as_any(&self) -> &dyn Any {
        &*self
      }
      fn eq(&self, other: &dyn Key) -> bool{
        other
          .as_any()
          .downcast_ref::<Self>()
          .map_or(false, |other| other == self)
      }
    }
  )*
}

from_key_impl!(
  ()
  usize u8 u16 u32 u64 u128
  isize i8 i16 i32 i64 i128
  f32 f64
  bool char
  StringKey
  [u8;32]
);

impl<T: Key + 'static> From<T> for Box<dyn Key> {
  #[inline(always)]
  fn from(v: T) -> Self { Box::new(v) }
}

#[derive(Clone, PartialEq, Debug)]
pub enum StringKey {
  Str(String),
  HashStr([u8; blake3::OUT_LEN]),
}

const MAX_KEY_STR: usize = 64;

impl From<String> for Box<dyn Key> {
  fn from(s: String) -> Self {
    let k = if s.len() > MAX_KEY_STR {
      StringKey::Str(s)
    } else {
      StringKey::HashStr(blake3::hash(s.as_bytes()).into())
    };
    Box::new(k)
  }
}

impl From<&str> for Box<dyn Key> {
  fn from(s: &str) -> Self {
    let k = if s.len() > MAX_KEY_STR {
      StringKey::Str(s.to_owned())
    } else {
      StringKey::HashStr(blake3::hash(s.as_bytes()).into())
    };
    Box::new(k)
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
  assert!(&k1.key != &k2.key);
  assert!(&k2.key == &k3.key);
  assert!(&k3.key != &k1.key);
  assert!(ck1.key == ck2.key);
}
