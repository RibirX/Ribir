use std::{borrow::Borrow, collections::HashMap, hash::Hash};

use ahash::RandomState;

/// A hashmap frame cache, which only keep the hit cached in the last frame.
/// Call [`frame_end`] when a frame finish.
pub struct FrameCache<K, V> {
  cache: HashMap<K, CacheValue<V>, RandomState>,
}

impl<K, V> FrameCache<K, V>
where
  K: Eq + Hash,
{
  #[inline]
  pub fn contains_key<Q>(&self, key: &Q) -> bool
  where
    K: Borrow<Q>,
    Q: Hash + Eq,
  {
    self.cache.contains_key(key)
  }

  #[inline]
  pub fn no_hit_get<Q>(&self, key: &Q) -> Option<&V>
  where
    K: Borrow<Q>,
    Q: Hash + Eq,
  {
    self.cache.get(key).map(|v| &v.value)
  }

  pub fn get<Q>(&mut self, key: &Q) -> Option<&V>
  where
    K: Borrow<Q>,
    Q: Hash + Eq,
  {
    self.cache.get_mut(key).map(|v| {
      v.last_frame_used = true;
      &v.value
    })
  }

  pub fn get_or_insert_with_key<F>(&mut self, key: K, default: F) -> &mut V
  where
    F: FnOnce(&K) -> V,
  {
    let v = self.cache.entry(key).or_insert_with_key(|k| {
      let value = default(k);
      CacheValue { value, last_frame_used: true }
    });
    v.last_frame_used = true;
    &mut v.value
  }

  pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
  where
    K: Borrow<Q>,
    Q: Hash + Eq,
  {
    self.cache.get_mut(key).map(|v| {
      v.last_frame_used = true;
      &mut v.value
    })
  }

  pub fn insert(&mut self, key: K, value: V) -> Option<V> {
    self
      .cache
      .insert(key, CacheValue { last_frame_used: true, value })
      .map(|c| c.value)
  }

  #[inline]
  pub fn len(&self) -> usize { self.cache.len() }

  pub fn frame_end(&mut self, label: &str) { self.frame_end_with::<fn(bool, &mut V)>(label, None); }

  /// A frame end, and missed cache will be removed, a callback for every cache
  /// with two arguments, the first is whether the cache will be retain, the
  /// second is the cached value.
  pub fn frame_end_with<F>(&mut self, label: &str, mut f: Option<F>)
  where
    F: FnMut(bool, &mut V),
  {
    let mut hit = 0;
    let count = self.cache.len();
    self.cache.retain(|_, v| {
      let last_used = v.last_frame_used;
      v.last_frame_used = false;
      if last_used {
        hit += 1
      };
      if let Some(f) = &mut f {
        f(last_used, &mut v.value);
      }

      last_used
    });

    log::info!(
      "Frame[{}]:  cache hit percent is {:.1}%",
      label,
      hit as f32 / count as f32
    );
  }

  /// Iterator access not as cache hit.
  pub fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
    self.cache.values_mut().map(|v| &mut v.value)
  }

  pub fn clear(&mut self) { self.cache.clear(); }
}

struct CacheValue<V> {
  last_frame_used: bool,
  value: V,
}

impl<K, V> Default for FrameCache<K, V> {
  #[inline]
  fn default() -> Self { Self { cache: Default::default() } }
}
