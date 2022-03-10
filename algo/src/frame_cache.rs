use std::{
  borrow::Borrow,
  collections::HashMap,
  hash::Hash,
  sync::atomic::{AtomicBool, Ordering},
};

use ahash::RandomState;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

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
    Q: Hash + Eq + ?Sized,
  {
    self.cache.contains_key(key)
  }

  #[inline]
  pub fn no_hit_get<Q>(&self, key: &Q) -> Option<&V>
  where
    K: Borrow<Q>,
    Q: Hash + Eq + ?Sized,
  {
    self.cache.get(key).map(|v| &v.value)
  }

  pub fn get<Q>(&self, key: &Q) -> Option<&V>
  where
    K: Borrow<Q>,
    Q: Hash + Eq + ?Sized,
  {
    self.cache.get(key).map(|v| {
      v.last_frame_used.store(true, Ordering::Relaxed);
      &v.value
    })
  }

  pub fn get_or_insert_with<Q, F>(&mut self, key: &Q, default: F) -> &mut V
  where
    K: Borrow<Q>,
    Q: Hash + ToOwned + Eq + ?Sized,
    Q::Owned: Into<K>,
    F: FnOnce() -> V,
  {
    if !self.contains_key(key) {
      let value = default();
      let key = key.to_owned().into();
      self.cache.insert(
        key,
        CacheValue {
          value,
          last_frame_used: AtomicBool::new(true),
        },
      );
    }
    self.get_mut(key).unwrap()
  }

  pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
  where
    K: Borrow<Q>,
    Q: Hash + Eq + ?Sized,
  {
    self.cache.get_mut(key).map(|v| {
      v.last_frame_used.store(true, Ordering::Relaxed);
      &mut v.value
    })
  }

  pub fn remove(&mut self, key: K) -> Option<V> { self.cache.remove(&key).map(|c| c.value) }

  pub fn insert(&mut self, key: K, value: V) -> Option<V> {
    self
      .cache
      .insert(
        key,
        CacheValue {
          last_frame_used: AtomicBool::new(true),
          value,
        },
      )
      .map(|c| c.value)
  }

  #[inline]
  pub fn as_uninit_map<A>(&mut self) -> UninitMap<K, V, A>
  where
    V: HeapPtr,
  {
    UninitMap { cache: self, uninit: vec![] }
  }

  #[inline]
  pub fn is_empty(&self) -> bool { self.cache.is_empty() }

  #[inline]
  pub fn len(&self) -> usize { self.cache.len() }

  pub fn end_frame(&mut self, label: &str) { self.frame_end_with::<fn(bool, &mut V)>(label, None); }

  /// A frame end, and missed cache will be removed, a callback for every cache
  /// with two arguments, the first is whether the cache will be retain, the
  /// second is the cached value.
  ///
  /// return the removed count
  pub fn frame_end_with<F>(&mut self, label: &str, mut f: Option<F>) -> usize
  where
    F: FnMut(bool, &mut V),
  {
    let mut hit = 0;
    let count = self.cache.len();
    self.cache.retain(|_, v| {
      let last_used = v.last_frame_used.load(Ordering::Relaxed);
      v.last_frame_used.store(false, Ordering::Relaxed);
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

    count - hit
  }

  /// Iterator access not as cache hit.
  pub fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
    self.cache.values_mut().map(|v| &mut v.value)
  }

  pub fn clear(&mut self) { self.cache.clear(); }
}

pub trait HeapPtr {
  type Target;
  fn heap_ptr(&self) -> *const Self::Target;

  fn heap_mut_ptr(&mut self) -> *mut Self::Target;
}

impl<T> HeapPtr for Box<T> {
  type Target = T;
  fn heap_ptr(&self) -> *const T { self.as_ref() as *const T }

  fn heap_mut_ptr(&mut self) -> *mut T { self.as_mut() as *mut T }
}

struct SendMutPtr<T>(*mut T);
// we know the pointer can pass across thread because is a unique pointer point
// to a heap and promise not access it before the task finish.
unsafe impl<T> Send for SendMutPtr<T> {}
impl<T> Clone for SendMutPtr<T> {
  fn clone(&self) -> Self { Self(self.0) }
}
impl<T> Copy for SendMutPtr<T> {}
impl<T> SendMutPtr<T> {
  unsafe fn as_mut(&mut self) -> &mut T { &mut *self.0 }
}

struct UninitRecord<V, A> {
  uninit_ptr: SendMutPtr<V>,
  key: A,
}
pub struct UninitMap<'a, K, V: HeapPtr, A> {
  cache: &'a mut FrameCache<K, V>,
  uninit: Vec<UninitRecord<V::Target, A>>,
}

impl<'a, K, V, A> UninitMap<'a, K, V, A>
where
  V: HeapPtr,
{
  /// Get the value pointer, notice the pointer may not init if it's not exist
  /// in map. `init_arg` use as argument to init the pointer when
  /// `par_init_with` call.

  /// # Safety
  /// The return pointer only valid access after `UninitMap::par_init` call and
  /// before another mutable borrow on its host `FrameCache`
  pub fn get_or_delay_init<Q>(&mut self, key: A) -> *mut V::Target
  where
    A: Borrow<Q>,
    K: Borrow<Q> + Hash + Eq,
    Q: Hash + Eq + ToOwned<Owned = K> + ?Sized,
    V: Default,
  {
    if let Some(v) = self.cache.get_mut(key.borrow()) {
      v.heap_mut_ptr()
    } else {
      let v = self.cache.get_or_insert_with(key.borrow(), V::default);

      let v_ptr = v.heap_mut_ptr();
      let v_mut_ptr = SendMutPtr(v_ptr);
      self
        .uninit
        .push(UninitRecord { uninit_ptr: v_mut_ptr, key });
      v_ptr
    }
  }

  /// Parallel init the uninit values, after this method called the pointer
  /// return by `get_or_delay_init` valid to use.
  pub fn par_init_with<F>(mut self, default: F)
  where
    F: Fn(A) -> V::Target + Send + Sync,
    A: Send,
  {
    let to_init = std::mem::take(&mut self.uninit);

    to_init
      .into_par_iter()
      .for_each(|UninitRecord { mut uninit_ptr, key: init_arg }| unsafe {
        *uninit_ptr.as_mut() = default(init_arg);
      });
  }
}

impl<'a, K, V: HeapPtr, A> Drop for UninitMap<'a, K, V, A> {
  fn drop(&mut self) {
    assert!(
      self.uninit.is_empty(),
      "Not call `par_init`, there are some value not init ."
    )
  }
}
struct CacheValue<V> {
  last_frame_used: AtomicBool,
  value: V,
}

impl<K, V> Default for FrameCache<K, V> {
  #[inline]
  fn default() -> Self { Self { cache: Default::default() } }
}
