//! An implementation of a frame cache, all items in the collection not hit by
//! last frame will drop.
//!
//! ** This implementation is base on [https://github.com/jeromefroe/lru-rs]!**, extend it from an LRU to a Frame Cache.
//!
//! ```

use std::{
  borrow::Borrow,
  collections::HashMap,
  fmt,
  hash::{Hash, Hasher},
  iter::FusedIterator,
  marker::PhantomData,
  mem::MaybeUninit,
  ptr::{self, NonNull},
};

// Struct used to hold a reference to a key
struct KeyRef<K> {
  k: *const K,
}

impl<K: Hash> Hash for KeyRef<K> {
  fn hash<H: Hasher>(&self, state: &mut H) { unsafe { (*self.k).hash(state) } }
}

impl<K: PartialEq> PartialEq for KeyRef<K> {
  fn eq(&self, other: &KeyRef<K>) -> bool { unsafe { (*self.k).eq(&*other.k) } }
}

impl<K: Eq> Eq for KeyRef<K> {}

// This type exists to allow a "blanket" Borrow impl for KeyRef without
// conflicting with the  stdlib blanket impl
#[repr(transparent)]
struct KeyWrapper<K: ?Sized>(K);

impl<K: ?Sized> KeyWrapper<K> {
  fn from_ref(key: &K) -> &Self {
    // safety: KeyWrapper is transparent, so casting the ref like this is allowable
    unsafe { &*(key as *const K as *const KeyWrapper<K>) }
  }
}

impl<K: ?Sized + Hash> Hash for KeyWrapper<K> {
  fn hash<H: Hasher>(&self, state: &mut H) { self.0.hash(state) }
}

impl<K: ?Sized + PartialEq> PartialEq for KeyWrapper<K> {
  fn eq(&self, other: &Self) -> bool { self.0.eq(&other.0) }
}

impl<K: ?Sized + Eq> Eq for KeyWrapper<K> {}

impl<K, Q> Borrow<KeyWrapper<Q>> for KeyRef<K>
where
  K: Borrow<Q>,
  Q: ?Sized,
{
  fn borrow(&self) -> &KeyWrapper<Q> {
    let key = unsafe { &*self.k }.borrow();
    KeyWrapper::from_ref(key)
  }
}

// Struct used to hold a key value pair. Also contains references to previous
// and next entries so we can maintain the entries in a linked list ordered by
// their use.
struct LruEntry<K, V> {
  key: MaybeUninit<K>,
  val: MaybeUninit<V>,
  prev: *mut LruEntry<K, V>,
  next: *mut LruEntry<K, V>,
}

impl<K, V> LruEntry<K, V> {
  fn new(key: K, val: V) -> Self {
    LruEntry {
      key: MaybeUninit::new(key),
      val: MaybeUninit::new(val),
      prev: ptr::null_mut(),
      next: ptr::null_mut(),
    }
  }

  fn new_sigil() -> Self {
    LruEntry {
      key: MaybeUninit::uninit(),
      val: MaybeUninit::uninit(),
      prev: ptr::null_mut(),
      next: ptr::null_mut(),
    }
  }
}

/// An LRU Cache
pub struct FrameCache<K, V> {
  map: HashMap<KeyRef<K>, NonNull<LruEntry<K, V>>, ahash::RandomState>,
  // head and tail are sigil nodes to facilitate inserting entries
  head: *mut LruEntry<K, V>,
  tail: *mut LruEntry<K, V>,
}

impl<K: Hash + Eq, V> FrameCache<K, V> {
  /// Creates a new Frame Cache
  pub fn new() -> FrameCache<K, V> {
    let cache = FrameCache {
      map: HashMap::default(),
      head: Box::into_raw(Box::new(LruEntry::new_sigil())),
      tail: Box::into_raw(Box::new(LruEntry::new_sigil())),
    };

    unsafe {
      (*cache.head).next = cache.tail;
      (*cache.tail).prev = cache.head;
    }

    cache
  }
}

impl<K: Hash + Eq, V> FrameCache<K, V> {
  /// Puts a key-value pair into cache. If the key already exists in the cache,
  /// then it updates the key's value and returns the old value. Otherwise,
  /// `None` is returned.
  ///
  /// # Example
  ///
  /// ```
  /// use ribir_algo::FrameCache;
  ///
  /// let mut cache = FrameCache::new();
  ///
  /// assert_eq!(None, cache.put(1, "a"));
  /// assert_eq!(None, cache.put(2, "b"));
  /// assert_eq!(Some("b"), cache.put(2, "beta"));
  ///
  /// assert_eq!(cache.get(&1), Some(&"a"));
  /// assert_eq!(cache.get(&2), Some(&"beta"));
  /// ```
  pub fn put(&mut self, k: K, v: V) -> Option<V> { self.push(k, v).map(|(_, v)| v) }

  /// Pushes a key-value pair into the cache. If an entry with key `k` already
  /// exists in the cache, then it returns the old entry's key-value pair.
  /// Otherwise, returns `None`.
  ///
  /// # Example
  ///
  /// ```
  /// use ribir_algo::FrameCache;
  ///
  /// let mut cache = FrameCache::new();
  ///
  /// assert_eq!(None, cache.push(1, "a"));
  ///
  /// cache.end_frame("");
  ///
  /// assert_eq!(None, cache.push(2, "b"));
  ///
  /// // This push call returns (2, "b") because that was previously 2's entry in the cache.
  /// assert_eq!(Some((2, "b")), cache.push(2, "beta"));
  /// assert_eq!(None, cache.push(3, "alpha"));
  /// cache.end_frame("");
  ///
  /// assert_eq!(cache.get(&1), None);
  /// assert_eq!(cache.get(&2), Some(&"beta"));
  /// assert_eq!(cache.get(&3), Some(&"alpha"));
  /// ```
  pub fn push(&mut self, k: K, mut v: V) -> Option<(K, V)> {
    let node_ref = self.map.get_mut(&KeyRef { k: &k });

    match node_ref {
      Some(node_ref) => {
        // if the key is already in the cache just update its value and move it to the
        // front of the list
        let node_ptr: *mut LruEntry<K, V> = node_ref.as_ptr();

        // gets a reference to the node to perform a swap and drops it right after
        let node_ref = unsafe { &mut (*(*node_ptr).val.as_mut_ptr()) };
        std::mem::swap(&mut v, node_ref);
        let _ = node_ref;

        self.detach(node_ptr);
        self.attach(node_ptr);
        Some((k, v))
      }
      None => {
        let node = self.create_node(k, v);
        let node_ptr: *mut LruEntry<K, V> = node.as_ptr();

        self.attach(node_ptr);

        let keyref = unsafe { (*node_ptr).key.as_ptr() };
        self.map.insert(KeyRef { k: keyref }, node);

        None
      }
    }
  }

  /// Returns a reference to the value of the key in the cache or `None` if it
  /// is not present in the cache. Moves the key to the head of the LRU list
  /// if it exists.
  ///
  /// # Example
  ///
  /// ```
  /// use ribir_algo::FrameCache;
  /// let mut cache = FrameCache::new();
  ///
  /// cache.put(1, "a");
  /// cache.end_frame("");
  ///
  /// cache.put(2, "b");
  /// cache.put(2, "c");
  /// cache.put(3, "d");
  /// cache.end_frame("");
  ///
  /// assert_eq!(cache.get(&1), None);
  /// assert_eq!(cache.get(&2), Some(&"c"));
  /// assert_eq!(cache.get(&3), Some(&"d"));
  /// ```
  pub fn get<'a, Q>(&'a mut self, k: &Q) -> Option<&'a V>
  where
    K: Borrow<Q>,
    Q: Hash + Eq + ?Sized,
  {
    if let Some(node) = self.map.get_mut(KeyWrapper::from_ref(k)) {
      let node_ptr: *mut LruEntry<K, V> = node.as_ptr();

      self.detach(node_ptr);
      self.attach(node_ptr);

      Some(unsafe { &*(*node_ptr).val.as_ptr() })
    } else {
      None
    }
  }

  /// Returns a mutable reference to the value of the key in the cache or `None`
  /// if it is not present in the cache. Moves the key to the head of the LRU
  /// list if it exists.
  ///
  /// # Example
  ///
  /// ```
  /// use ribir_algo::FrameCache;
  /// let mut cache = FrameCache::new();
  ///
  /// cache.put("apple", 8);
  /// cache.end_frame("");
  ///
  /// cache.put("banana", 4);
  /// cache.put("banana", 6);
  /// cache.put("pear", 2);
  /// cache.end_frame("");
  ///
  /// assert_eq!(cache.get_mut(&"apple"), None);
  /// assert_eq!(cache.get_mut(&"banana"), Some(&mut 6));
  /// assert_eq!(cache.get_mut(&"pear"), Some(&mut 2));
  /// ```
  pub fn get_mut<'a, Q>(&'a mut self, k: &Q) -> Option<&'a mut V>
  where
    K: Borrow<Q>,
    Q: Hash + Eq + ?Sized,
  {
    if let Some(node) = self.map.get_mut(KeyWrapper::from_ref(k)) {
      let node_ptr: *mut LruEntry<K, V> = node.as_ptr();

      self.detach(node_ptr);
      self.attach(node_ptr);

      Some(unsafe { &mut *(*node_ptr).val.as_mut_ptr() })
    } else {
      None
    }
  }

  /// Returns a reference to the value of the key in the cache if it is
  /// present in the cache and moves the key to the head of the LRU list.
  /// If the key does not exist the provided `FnOnce` is used to populate
  /// the list and a reference is returned.
  ///
  /// # Example
  ///
  /// ```
  /// use ribir_algo::FrameCache;
  /// let mut cache = FrameCache::new();
  ///
  /// cache.put(1, "a");
  /// cache.put(2, "b");
  /// cache.put(2, "c");
  /// cache.put(3, "d");
  ///
  /// assert_eq!(cache.get_or_insert(2, || "a"), &"c");
  /// assert_eq!(cache.get_or_insert(3, || "a"), &"d");
  /// assert_eq!(cache.get_or_insert(1, || "a"), &"a");
  /// assert_eq!(cache.get_or_insert(1, || "b"), &"a");
  /// ```
  pub fn get_or_insert<F>(&mut self, k: K, f: F) -> &V
  where
    F: FnOnce() -> V,
  {
    if let Some(node) = self.map.get_mut(&KeyRef { k: &k }) {
      let node_ptr: *mut LruEntry<K, V> = node.as_ptr();

      self.detach(node_ptr);
      self.attach(node_ptr);

      unsafe { &*(*node_ptr).val.as_ptr() }
    } else {
      let v = f();
      let node = self.create_node(k, v);
      let node_ptr: *mut LruEntry<K, V> = node.as_ptr();

      self.attach(node_ptr);

      let keyref = unsafe { (*node_ptr).key.as_ptr() };
      self.map.insert(KeyRef { k: keyref }, node);
      unsafe { &*(*node_ptr).val.as_ptr() }
    }
  }

  /// Returns a mutable reference to the value of the key in the cache if it is
  /// present in the cache and moves the key to the head of the LRU list.
  /// If the key does not exist the provided `FnOnce` is used to populate
  /// the list and a mutable reference is returned.
  ///
  /// # Example
  ///
  /// ```
  /// use ribir_algo::FrameCache;
  /// let mut cache = FrameCache::new();
  ///
  /// cache.put(1, "a");
  /// cache.put(2, "b");
  ///
  /// let v = cache.get_or_insert_mut(2, || "c");
  /// assert_eq!(v, &"b");
  /// *v = "d";
  /// assert_eq!(cache.get_or_insert_mut(2, || "e"), &mut "d");
  /// assert_eq!(cache.get_or_insert_mut(3, || "f"), &mut "f");
  /// assert_eq!(cache.get_or_insert_mut(3, || "e"), &mut "f");
  /// ```
  pub fn get_or_insert_mut<F>(&mut self, k: K, f: F) -> &mut V
  where
    F: FnOnce() -> V,
  {
    if let Some(node) = self.map.get_mut(&KeyRef { k: &k }) {
      let node_ptr: *mut LruEntry<K, V> = node.as_ptr();

      self.detach(node_ptr);
      self.attach(node_ptr);

      unsafe { &mut *(*node_ptr).val.as_mut_ptr() }
    } else {
      let v = f();
      let node = self.create_node(k, v);
      let node_ptr: *mut LruEntry<K, V> = node.as_ptr();

      self.attach(node_ptr);

      let keyref = unsafe { (*node_ptr).key.as_ptr() };
      self.map.insert(KeyRef { k: keyref }, node);
      unsafe { &mut *(*node_ptr).val.as_mut_ptr() }
    }
  }

  /// Returns a reference to the value corresponding to the key in the cache or
  /// `None` if it is not present in the cache. Unlike `get`, `peek` does not
  /// update the LRU list so the key's position will be unchanged.
  ///
  /// # Example
  ///
  /// ```
  /// use ribir_algo::FrameCache;
  /// let mut cache = FrameCache::new();
  ///
  /// cache.put(1, "a");
  /// cache.put(2, "b");
  ///
  /// assert_eq!(cache.peek(&1), Some(&"a"));
  /// assert_eq!(cache.peek(&2), Some(&"b"));
  /// ```
  pub fn peek<'a, Q>(&'a self, k: &Q) -> Option<&'a V>
  where
    K: Borrow<Q>,
    Q: Hash + Eq + ?Sized,
  {
    self
      .map
      .get(KeyWrapper::from_ref(k))
      .map(|node| unsafe { &*node.as_ref().val.as_ptr() })
  }

  /// Returns a mutable reference to the value corresponding to the key in the
  /// cache or `None` if it is not present in the cache. Unlike `get_mut`,
  /// `peek_mut` does not update the LRU list so the key's position will be
  /// unchanged.
  ///
  /// # Example
  ///
  /// ```
  /// use ribir_algo::FrameCache;
  /// let mut cache = FrameCache::new();
  ///
  /// cache.put(1, "a");
  /// cache.put(2, "b");
  ///
  /// assert_eq!(cache.peek_mut(&1), Some(&mut "a"));
  /// assert_eq!(cache.peek_mut(&2), Some(&mut "b"));
  /// ```
  pub fn peek_mut<'a, Q>(&'a mut self, k: &Q) -> Option<&'a mut V>
  where
    K: Borrow<Q>,
    Q: Hash + Eq + ?Sized,
  {
    match self.map.get_mut(KeyWrapper::from_ref(k)) {
      None => None,
      Some(node) => Some(unsafe { &mut *(*node.as_ptr()).val.as_mut_ptr() }),
    }
  }

  /// Returns the value corresponding to the least recently used item or `None`
  /// if the cache is empty. Like `peek`, `peek_lru` does not update the LRU
  /// list so the item's position will be unchanged.
  ///
  /// # Example
  ///
  /// ```
  /// use ribir_algo::FrameCache;
  /// let mut cache = FrameCache::new();
  ///
  /// cache.put(1, "a");
  /// cache.put(2, "b");
  ///
  /// assert_eq!(cache.peek_lru(), Some((&1, &"a")));
  /// ```
  pub fn peek_lru(&self) -> Option<(&K, &V)> {
    if self.is_empty() {
      return None;
    }

    let (key, val);
    unsafe {
      let node = (*self.tail).prev;
      key = &(*(*node).key.as_ptr()) as &K;
      val = &(*(*node).val.as_ptr()) as &V;
    }

    Some((key, val))
  }

  /// Returns a bool indicating whether the given key is in the cache. Does not
  /// update the LRU list.
  ///
  /// # Example
  ///
  /// ```
  /// use ribir_algo::FrameCache;
  /// let mut cache = FrameCache::new();
  ///
  /// cache.put(1, "a");
  /// cache.put(2, "b");
  /// cache.put(3, "c");
  ///
  /// assert!(cache.contains(&1));
  /// assert!(cache.contains(&2));
  /// assert!(cache.contains(&3));
  /// ```
  pub fn contains<Q>(&self, k: &Q) -> bool
  where
    K: Borrow<Q>,
    Q: Hash + Eq + ?Sized,
  {
    self.map.contains_key(KeyWrapper::from_ref(k))
  }

  /// Removes and returns the value corresponding to the key from the cache or
  /// `None` if it does not exist.
  ///
  /// # Example
  ///
  /// ```
  /// use ribir_algo::FrameCache;
  /// let mut cache = FrameCache::new();
  ///
  /// cache.put(2, "a");
  ///
  /// assert_eq!(cache.pop(&1), None);
  /// assert_eq!(cache.pop(&2), Some("a"));
  /// assert_eq!(cache.pop(&2), None);
  /// assert_eq!(cache.len(), 0);
  /// ```
  pub fn pop<Q>(&mut self, k: &Q) -> Option<V>
  where
    K: Borrow<Q>,
    Q: Hash + Eq + ?Sized,
  {
    match self.map.remove(KeyWrapper::from_ref(k)) {
      None => None,
      Some(old_node) => {
        let mut old_node = unsafe {
          let mut old_node = *Box::from_raw(old_node.as_ptr());
          ptr::drop_in_place(old_node.key.as_mut_ptr());

          old_node
        };

        self.detach(&mut old_node);

        let LruEntry { key: _, val, .. } = old_node;
        unsafe { Some(val.assume_init()) }
      }
    }
  }

  /// Removes and returns the key and the value corresponding to the key from
  /// the cache or `None` if it does not exist.
  ///
  /// # Example
  ///
  /// ```
  /// use ribir_algo::FrameCache;
  /// let mut cache = FrameCache::new();
  ///
  /// cache.put(1, "a");
  /// cache.put(2, "a");
  ///
  /// assert_eq!(cache.pop(&1), Some("a"));
  /// assert_eq!(cache.pop_entry(&2), Some((2, "a")));
  /// assert_eq!(cache.pop(&1), None);
  /// assert_eq!(cache.pop_entry(&2), None);
  /// assert_eq!(cache.len(), 0);
  /// ```
  pub fn pop_entry<Q>(&mut self, k: &Q) -> Option<(K, V)>
  where
    K: Borrow<Q>,
    Q: Hash + Eq + ?Sized,
  {
    match self.map.remove(KeyWrapper::from_ref(k)) {
      None => None,
      Some(old_node) => {
        let mut old_node = unsafe { *Box::from_raw(old_node.as_ptr()) };

        self.detach(&mut old_node);

        let LruEntry { key, val, .. } = old_node;
        unsafe { Some((key.assume_init(), val.assume_init())) }
      }
    }
  }

  /// Removes and returns the key and value corresponding to the least recently
  /// used item or `None` if the cache is empty.
  ///
  /// # Example
  ///
  /// ```
  /// use ribir_algo::FrameCache;
  /// let mut cache = FrameCache::new();
  ///
  /// cache.put(2, "a");
  /// cache.put(3, "b");
  /// cache.put(4, "c");
  /// cache.get(&3);
  ///
  /// assert_eq!(cache.pop_lru(), Some((2, "a")));
  /// assert_eq!(cache.pop_lru(), Some((4, "c")));
  /// assert_eq!(cache.pop_lru(), Some((3, "b")));
  /// assert_eq!(cache.len(), 0);
  /// ```
  pub fn pop_lru(&mut self) -> Option<(K, V)> {
    let node = self.remove_last()?;
    // N.B.: Can't destructure directly because of https://github.com/rust-lang/rust/issues/28536
    let node = *node;
    let LruEntry { key, val, .. } = node;
    unsafe { Some((key.assume_init(), val.assume_init())) }
  }

  /// Marks the key as the most recently used one.
  ///
  /// # Example
  ///
  /// ```
  /// use ribir_algo::FrameCache;
  /// let mut cache = FrameCache::new();
  ///
  /// cache.put(1, "a");
  /// cache.put(2, "b");
  /// cache.put(3, "c");
  /// cache.get(&1);
  /// cache.get(&2);
  ///
  /// // If we do `pop_lru` now, we would pop 3.
  /// // assert_eq!(cache.pop_lru(), Some((3, "c")));
  ///
  /// // By promoting 3, we make sure it isn't popped.
  /// cache.promote(&3);
  /// assert_eq!(cache.pop_lru(), Some((1, "a")));
  /// ```
  pub fn promote<Q>(&mut self, k: &Q)
  where
    K: Borrow<Q>,
    Q: Hash + Eq + ?Sized,
  {
    if let Some(node) = self.map.get_mut(KeyWrapper::from_ref(k)) {
      let node_ptr: *mut LruEntry<K, V> = node.as_ptr();
      self.detach(node_ptr);
      self.attach(node_ptr);
    }
  }

  /// Marks the key as the least recently used one.
  ///
  /// # Example
  ///
  /// ```
  /// use ribir_algo::FrameCache;
  /// let mut cache = FrameCache::new();
  ///
  /// cache.put(1, "a");
  /// cache.put(2, "b");
  /// cache.put(3, "c");
  /// cache.get(&1);
  /// cache.get(&2);
  ///
  /// // If we do `pop_lru` now, we would pop 3.
  /// assert_eq!(cache.pop_lru(), Some((3, "c")));
  ///
  /// // By demoting 1 and 2, we make sure those are popped first.
  /// cache.demote(&2);
  /// cache.demote(&1);
  /// assert_eq!(cache.pop_lru(), Some((1, "a")));
  /// assert_eq!(cache.pop_lru(), Some((2, "b")));
  /// ```
  pub fn demote<Q>(&mut self, k: &Q)
  where
    K: Borrow<Q>,
    Q: Hash + Eq + ?Sized,
  {
    if let Some(node) = self.map.get_mut(KeyWrapper::from_ref(k)) {
      let node_ptr: *mut LruEntry<K, V> = node.as_ptr();
      self.detach(node_ptr);
      self.attach_last(node_ptr);
    }
  }

  /// Returns the number of key-value pairs that are currently in the the cache.
  ///
  /// # Example
  ///
  /// ```
  /// use ribir_algo::FrameCache;
  /// let mut cache = FrameCache::new();
  /// assert_eq!(cache.len(), 0);
  ///
  /// cache.put(1, "a");
  /// assert_eq!(cache.len(), 1);
  ///
  /// cache.put(2, "b");
  /// assert_eq!(cache.len(), 2);
  ///
  /// cache.put(3, "c");
  /// assert_eq!(cache.len(), 3);
  /// ```
  pub fn len(&self) -> usize { self.map.len() }

  /// Returns a bool indicating whether the cache is empty or not.
  ///
  /// # Example
  ///
  /// ```
  /// use ribir_algo::FrameCache;
  /// let mut cache = FrameCache::new();
  /// assert!(cache.is_empty());
  ///
  /// cache.put(1, "a");
  /// assert!(!cache.is_empty());
  /// ```
  pub fn is_empty(&self) -> bool { self.map.len() == 0 }

  /// Clears the contents of the cache.
  ///
  /// # Example
  ///
  /// ```
  /// use ribir_algo::FrameCache;
  /// let mut cache: FrameCache<isize, &str> = FrameCache::new();
  /// assert_eq!(cache.len(), 0);
  ///
  /// cache.put(1, "a");
  /// assert_eq!(cache.len(), 1);
  ///
  /// cache.put(2, "b");
  /// assert_eq!(cache.len(), 2);
  ///
  /// cache.clear();
  /// assert_eq!(cache.len(), 0);
  /// ```
  pub fn clear(&mut self) { while self.pop_lru().is_some() {} }

  /// An iterator visiting all entries in most-recently used order. The iterator
  /// element type is `(&K, &V)`.
  ///
  /// # Examples
  ///
  /// ```
  /// use ribir_algo::FrameCache;
  ///
  /// let mut cache = FrameCache::new();
  /// cache.put("a", 1);
  /// cache.put("b", 2);
  /// cache.put("c", 3);
  ///
  /// for (key, val) in cache.iter() {
  ///   println!("key: {} val: {}", key, val);
  /// }
  /// ```
  pub fn iter(&self) -> Iter<'_, K, V> {
    Iter {
      len: self.len(),
      ptr: unsafe { (*self.head).next },
      end: unsafe { (*self.tail).prev },
      phantom: PhantomData,
    }
  }

  /// An iterator visiting all entries in most-recently-used order, giving a
  /// mutable reference on V.  The iterator element type is `(&K, &mut V)`.
  ///
  /// # Examples
  ///
  /// ```
  /// use ribir_algo::FrameCache;
  ///
  /// struct HddBlock {
  ///   dirty: bool,
  ///   data: [u8; 512],
  /// }
  ///
  /// let mut cache = FrameCache::new();
  /// cache.put(0, HddBlock { dirty: false, data: [0x00; 512] });
  /// cache.put(1, HddBlock { dirty: true, data: [0x55; 512] });
  /// cache.put(2, HddBlock { dirty: true, data: [0x77; 512] });
  ///
  /// // write dirty blocks to disk.
  /// for (block_id, block) in cache.iter_mut() {
  ///   if block.dirty {
  ///     // write block to disk
  ///     block.dirty = false
  ///   }
  /// }
  /// ```
  pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
    IterMut {
      len: self.len(),
      ptr: unsafe { (*self.head).next },
      end: unsafe { (*self.tail).prev },
      phantom: PhantomData,
    }
  }

  // Used internally to swap out a node if the cache is full or to create a new
  // node if space is available. Shared between `put`, `push`, `get_or_insert`,
  // and `get_or_insert_mut`.
  #[allow(clippy::type_complexity)]
  fn create_node(&mut self, k: K, v: V) -> NonNull<LruEntry<K, V>> {
    // if the cache is not full allocate a new LruEntry
    // Safety: We allocate, turn into raw, and get NonNull all in one step.
    unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(LruEntry::new(k, v)))) }
  }

  fn remove_last(&mut self) -> Option<Box<LruEntry<K, V>>> {
    let prev;
    unsafe { prev = (*self.tail).prev }
    if prev != self.head {
      let old_key = KeyRef { k: unsafe { &(*(*(*self.tail).prev).key.as_ptr()) } };
      let old_node = self.map.remove(&old_key).unwrap();
      let node_ptr: *mut LruEntry<K, V> = old_node.as_ptr();
      self.detach(node_ptr);
      unsafe { Some(Box::from_raw(node_ptr)) }
    } else {
      None
    }
  }

  fn detach(&mut self, node: *mut LruEntry<K, V>) {
    unsafe {
      (*(*node).prev).next = (*node).next;
      // node has at least one predecessor (head), but not necessarily a successor
      if !(*node).next.is_null() {
        (*(*node).next).prev = (*node).prev;
      }
    }
  }

  fn break_at(&mut self, node: *mut LruEntry<K, V>) {
    unsafe {
      (*(*node).prev).next = ptr::null_mut();
      (*node).prev = ptr::null_mut();
    }
  }

  // Attaches `node` after the sigil `self.head` node.
  fn attach(&mut self, node: *mut LruEntry<K, V>) {
    unsafe {
      (*node).next = (*self.head).next;
      (*node).prev = self.head;
      (*self.head).next = node;
      // node has at least one predecessor (head), but not necessarily a successor
      if !(*node).next.is_null() {
        (*(*node).next).prev = node;
      }
    }
  }

  // Attaches `node` before the sigil `self.tail` node.
  fn attach_last(&mut self, node: *mut LruEntry<K, V>) {
    unsafe {
      (*node).next = self.tail;
      (*node).prev = (*self.tail).prev;
      (*self.tail).prev = node;
      (*(*node).prev).next = node;
    }
  }
}

pub struct FrameDrain<'a, K: Hash + Eq, V> {
  size: usize,
  label: &'a str,
  cursor: *mut LruEntry<K, V>,
  cache: &'a mut FrameCache<K, V>,
}

impl<'a, K: Hash + Eq, V> FrameDrain<'a, K, V> {
  pub fn new(cache: &'a mut FrameCache<K, V>, label: &'a str) -> Self {
    let tail = cache.tail;

    unsafe {
      cache.break_at(tail);
      let cursor = (*tail).next;
      if !cursor.is_null() {
        cache.break_at(cursor);
      }
      cache.attach(tail);
      let size = cache.len();
      FrameDrain { size, cache, label, cursor }
    }
  }
}

impl<'a, K: Hash + Eq, V> Iterator for FrameDrain<'a, K, V> {
  type Item = V;

  fn next(&mut self) -> Option<Self::Item> {
    if !self.cursor.is_null() {
      unsafe {
        let old_key = KeyRef { k: &(*(*self.cursor).key.as_ptr()) };

        self.cache.map.remove(&old_key).unwrap();
        let v = *Box::from_raw(self.cursor);

        let LruEntry { mut key, val, next, .. } = v;
        ptr::drop_in_place(key.as_mut_ptr());

        self.cursor = next;
        Some(val.assume_init())
      }
    } else {
      None
    }
  }
}

impl<'a, K: Hash + Eq, V> Drop for FrameDrain<'a, K, V> {
  fn drop(&mut self) {
    while self.next().is_some() {}

    if self.size > 0 {
      log::info!(
        "Frame[{}]: cache hit percent is {:.1}%",
        self.label,
        self.cache.len() as f32 / self.size as f32
      );
    }
  }
}

impl<K, V> Drop for FrameCache<K, V> {
  fn drop(&mut self) {
    self.map.drain().for_each(|(_, node)| unsafe {
      let mut node = *Box::from_raw(node.as_ptr());
      ptr::drop_in_place((node).key.as_mut_ptr());
      ptr::drop_in_place((node).val.as_mut_ptr());
    });
    // We rebox the head/tail, and because these are maybe-uninit
    // they do not have the absent k/v dropped.

    let _head = unsafe { *Box::from_raw(self.head) };
    let _tail = unsafe { *Box::from_raw(self.tail) };
  }
}

impl<'a, K: Hash + Eq, V> IntoIterator for &'a FrameCache<K, V> {
  type Item = (&'a K, &'a V);
  type IntoIter = Iter<'a, K, V>;

  fn into_iter(self) -> Iter<'a, K, V> { self.iter() }
}

impl<'a, K: Hash + Eq, V> IntoIterator for &'a mut FrameCache<K, V> {
  type Item = (&'a K, &'a mut V);
  type IntoIter = IterMut<'a, K, V>;

  fn into_iter(self) -> IterMut<'a, K, V> { self.iter_mut() }
}

// The compiler does not automatically derive Send and Sync for FrameCache
// because it contains raw pointers. The raw pointers are safely encapsulated by
// FrameCache though so we can implement Send and Sync for it below.
unsafe impl<K: Send, V: Send> Send for FrameCache<K, V> {}
unsafe impl<K: Sync, V: Sync> Sync for FrameCache<K, V> {}

impl<K: Hash + Eq, V> fmt::Debug for FrameCache<K, V> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.debug_struct("FrameCache")
      .field("len", &self.len())
      .finish()
  }
}

/// An iterator over the entries of a `FrameCache`.
///
/// This `struct` is created by the [`iter`] method on
/// [`FrameCache`][`FrameCache`]. See its documentation for more.
///
/// [`iter`]: struct.FrameCache.html#method.iter
/// [`FrameCache`]: struct.FrameCache.html
pub struct Iter<'a, K: 'a, V: 'a> {
  len: usize,

  ptr: *const LruEntry<K, V>,
  end: *const LruEntry<K, V>,

  phantom: PhantomData<&'a K>,
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
  type Item = (&'a K, &'a V);

  fn next(&mut self) -> Option<(&'a K, &'a V)> {
    if self.len == 0 {
      return None;
    }

    let key = unsafe { &(*(*self.ptr).key.as_ptr()) as &K };
    let val = unsafe { &(*(*self.ptr).val.as_ptr()) as &V };

    self.len -= 1;
    self.ptr = unsafe { (*self.ptr).next };

    Some((key, val))
  }

  fn size_hint(&self) -> (usize, Option<usize>) { (self.len, Some(self.len)) }

  fn count(self) -> usize { self.len }
}

impl<'a, K, V> DoubleEndedIterator for Iter<'a, K, V> {
  fn next_back(&mut self) -> Option<(&'a K, &'a V)> {
    if self.len == 0 {
      return None;
    }

    let key = unsafe { &(*(*self.end).key.as_ptr()) as &K };
    let val = unsafe { &(*(*self.end).val.as_ptr()) as &V };

    self.len -= 1;
    self.end = unsafe { (*self.end).prev };

    Some((key, val))
  }
}

impl<'a, K, V> ExactSizeIterator for Iter<'a, K, V> {}
impl<'a, K, V> FusedIterator for Iter<'a, K, V> {}

impl<'a, K, V> Clone for Iter<'a, K, V> {
  fn clone(&self) -> Iter<'a, K, V> {
    Iter { len: self.len, ptr: self.ptr, end: self.end, phantom: PhantomData }
  }
}

// The compiler does not automatically derive Send and Sync for Iter because it
// contains raw pointers.
unsafe impl<'a, K: Send, V: Send> Send for Iter<'a, K, V> {}
unsafe impl<'a, K: Sync, V: Sync> Sync for Iter<'a, K, V> {}

/// An iterator over mutables entries of a `FrameCache`.
///
/// This `struct` is created by the [`iter_mut`] method on
/// [`FrameCache`][`FrameCache`]. See its documentation for more.
///
/// [`iter_mut`]: struct.FrameCache.html#method.iter_mut
/// [`FrameCache`]: struct.FrameCache.html
pub struct IterMut<'a, K: 'a, V: 'a> {
  len: usize,

  ptr: *mut LruEntry<K, V>,
  end: *mut LruEntry<K, V>,

  phantom: PhantomData<&'a K>,
}

impl<'a, K, V> Iterator for IterMut<'a, K, V> {
  type Item = (&'a K, &'a mut V);

  fn next(&mut self) -> Option<(&'a K, &'a mut V)> {
    if self.len == 0 {
      return None;
    }

    let key = unsafe { &mut (*(*self.ptr).key.as_mut_ptr()) as &mut K };
    let val = unsafe { &mut (*(*self.ptr).val.as_mut_ptr()) as &mut V };

    self.len -= 1;
    self.ptr = unsafe { (*self.ptr).next };

    Some((key, val))
  }

  fn size_hint(&self) -> (usize, Option<usize>) { (self.len, Some(self.len)) }

  fn count(self) -> usize { self.len }
}

impl<'a, K, V> DoubleEndedIterator for IterMut<'a, K, V> {
  fn next_back(&mut self) -> Option<(&'a K, &'a mut V)> {
    if self.len == 0 {
      return None;
    }

    let key = unsafe { &mut (*(*self.end).key.as_mut_ptr()) as &mut K };
    let val = unsafe { &mut (*(*self.end).val.as_mut_ptr()) as &mut V };

    self.len -= 1;
    self.end = unsafe { (*self.end).prev };

    Some((key, val))
  }
}

impl<'a, K, V> ExactSizeIterator for IterMut<'a, K, V> {}
impl<'a, K, V> FusedIterator for IterMut<'a, K, V> {}

// The compiler does not automatically derive Send and Sync for Iter because it
// contains raw pointers.
unsafe impl<'a, K: Send, V: Send> Send for IterMut<'a, K, V> {}
unsafe impl<'a, K: Sync, V: Sync> Sync for IterMut<'a, K, V> {}

/// An iterator that moves out of a `FrameCache`.
///
/// This `struct` is created by the [`into_iter`] method on
/// [`FrameCache`][`FrameCache`]. See its documentation for more.
///
/// [`into_iter`]: struct.FrameCache.html#method.into_iter
/// [`FrameCache`]: struct.FrameCache.html
pub struct IntoIter<K, V>
where
  K: Hash + Eq,
{
  cache: FrameCache<K, V>,
}

impl<K, V> Iterator for IntoIter<K, V>
where
  K: Hash + Eq,
{
  type Item = (K, V);

  fn next(&mut self) -> Option<(K, V)> { self.cache.pop_lru() }

  fn size_hint(&self) -> (usize, Option<usize>) {
    let len = self.cache.len();
    (len, Some(len))
  }

  fn count(self) -> usize { self.cache.len() }
}

impl<K, V> ExactSizeIterator for IntoIter<K, V> where K: Hash + Eq {}
impl<K, V> FusedIterator for IntoIter<K, V> where K: Hash + Eq {}

impl<K: Hash + Eq, V> IntoIterator for FrameCache<K, V> {
  type Item = (K, V);
  type IntoIter = IntoIter<K, V>;

  fn into_iter(self) -> IntoIter<K, V> { IntoIter { cache: self } }
}

impl<K: Hash + Eq, V> FrameCache<K, V> {
  /// End the frame and return a iterator of the items will removed.
  pub fn end_frame<'a>(&'a mut self, label: &'a str) -> FrameDrain<'a, K, V> {
    FrameDrain::new(self, label)
  }
}

impl<K: Hash + Eq, V> Default for FrameCache<K, V> {
  #[inline]
  fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
  use core::fmt::Debug;
  use std::sync::atomic::{AtomicUsize, Ordering};

  use scoped_threadpool::Pool;

  use super::FrameCache;

  fn assert_opt_eq<V: PartialEq + Debug>(opt: Option<&V>, v: V) {
    assert!(opt.is_some());
    assert_eq!(opt.unwrap(), &v);
  }

  fn assert_opt_eq_mut<V: PartialEq + Debug>(opt: Option<&mut V>, v: V) {
    assert!(opt.is_some());
    assert_eq!(opt.unwrap(), &v);
  }

  fn assert_opt_eq_tuple<K: PartialEq + Debug, V: PartialEq + Debug>(
    opt: Option<(&K, &V)>, kv: (K, V),
  ) {
    assert!(opt.is_some());
    let res = opt.unwrap();
    assert_eq!(res.0, &kv.0);
    assert_eq!(res.1, &kv.1);
  }

  fn assert_opt_eq_mut_tuple<K: PartialEq + Debug, V: PartialEq + Debug>(
    opt: Option<(&K, &mut V)>, kv: (K, V),
  ) {
    assert!(opt.is_some());
    let res = opt.unwrap();
    assert_eq!(res.0, &kv.0);
    assert_eq!(res.1, &kv.1);
  }

  #[test]
  fn test_unbounded() {
    let mut cache = FrameCache::new();
    for i in 0..13370 {
      cache.put(i, ());
    }
    assert_eq!(cache.len(), 13370);
  }

  #[test]
  fn test_put_and_get() {
    let mut cache = FrameCache::new();
    assert!(cache.is_empty());

    assert_eq!(cache.put("apple", "red"), None);
    assert_eq!(cache.put("banana", "yellow"), None);

    assert_eq!(cache.len(), 2);
    assert!(!cache.is_empty());
    assert_opt_eq(cache.get(&"apple"), "red");
    assert_opt_eq(cache.get(&"banana"), "yellow");
  }

  #[test]
  fn test_put_and_get_or_insert() {
    let mut cache = FrameCache::new();
    assert!(cache.is_empty());

    assert_eq!(cache.put("apple", "red"), None);
    assert_eq!(cache.put("banana", "yellow"), None);

    assert_eq!(cache.len(), 2);
    assert!(!cache.is_empty());
    assert_eq!(cache.get_or_insert("apple", || "orange"), &"red");
    assert_eq!(cache.get_or_insert("banana", || "orange"), &"yellow");
    assert_eq!(cache.get_or_insert("lemon", || "orange"), &"orange");
    assert_eq!(cache.get_or_insert("lemon", || "red"), &"orange");
  }

  #[test]
  fn test_put_and_get_or_insert_mut() {
    let mut cache = FrameCache::new();
    assert!(cache.is_empty());

    assert_eq!(cache.put("apple", "red"), None);
    assert_eq!(cache.put("banana", "yellow"), None);

    assert_eq!(cache.len(), 2);

    let v = cache.get_or_insert_mut("apple", || "orange");
    assert_eq!(v, &"red");
    *v = "blue";

    assert_eq!(cache.get_or_insert_mut("apple", || "orange"), &"blue");
    assert_eq!(cache.get_or_insert_mut("banana", || "orange"), &"yellow");
    assert_eq!(cache.get_or_insert_mut("lemon", || "orange"), &"orange");
    assert_eq!(cache.get_or_insert_mut("lemon", || "red"), &"orange");
  }

  #[test]
  fn test_put_and_get_mut() {
    let mut cache = FrameCache::new();

    cache.put("apple", "red");
    cache.put("banana", "yellow");

    assert_eq!(cache.len(), 2);
    assert_opt_eq_mut(cache.get_mut(&"apple"), "red");
    assert_opt_eq_mut(cache.get_mut(&"banana"), "yellow");
  }

  #[test]
  fn test_get_mut_and_update() {
    let mut cache = FrameCache::new();

    cache.put("apple", 1);
    cache.put("banana", 3);

    {
      let v = cache.get_mut(&"apple").unwrap();
      *v = 4;
    }

    assert_eq!(cache.len(), 2);
    assert_opt_eq_mut(cache.get_mut(&"apple"), 4);
    assert_opt_eq_mut(cache.get_mut(&"banana"), 3);
  }

  #[test]
  fn test_put_update() {
    let mut cache = FrameCache::new();

    assert_eq!(cache.put("apple", "red"), None);
    assert_eq!(cache.put("apple", "green"), Some("red"));

    assert_eq!(cache.len(), 1);
    assert_opt_eq(cache.get(&"apple"), "green");
  }

  #[test]
  fn test_put_removes_oldest() {
    let mut cache = FrameCache::new();

    assert_eq!(cache.put("apple", "red"), None);
    assert_eq!(cache.put("banana", "yellow"), None);
    assert_eq!(cache.put("pear", "green"), None);

    cache.end_frame("");
    assert_opt_eq(cache.get(&"banana"), "yellow");
    assert_opt_eq(cache.get(&"pear"), "green");
    cache.end_frame("");
    // Even though we inserted "apple" into the cache earlier it has since been
    // removed from the cache so there is no current value for `put` to return.
    assert_eq!(cache.put("apple", "green"), None);
    assert_eq!(cache.put("tomato", "red"), None);
    cache.end_frame("");

    assert!(cache.get(&"pear").is_none());
    assert_opt_eq(cache.get(&"apple"), "green");
    assert_opt_eq(cache.get(&"tomato"), "red");
  }

  #[test]
  fn test_peek() {
    let mut cache = FrameCache::new();

    cache.put("apple", "red");
    cache.put("banana", "yellow");

    assert_opt_eq(cache.peek(&"banana"), "yellow");
    assert_opt_eq(cache.peek(&"apple"), "red");

    cache.put("pear", "green");

    assert_opt_eq(cache.peek(&"banana"), "yellow");
    assert_opt_eq(cache.peek(&"pear"), "green");
  }

  #[test]
  fn test_peek_mut() {
    let mut cache = FrameCache::new();

    cache.put("apple", "red");
    cache.put("banana", "yellow");

    assert_opt_eq_mut(cache.peek_mut(&"banana"), "yellow");
    assert_opt_eq_mut(cache.peek_mut(&"apple"), "red");
    assert!(cache.peek_mut(&"pear").is_none());

    cache.put("pear", "green");

    assert_opt_eq_mut(cache.peek_mut(&"banana"), "yellow");
    assert_opt_eq_mut(cache.peek_mut(&"pear"), "green");

    {
      let v = cache.peek_mut(&"banana").unwrap();
      *v = "green";
    }

    assert_opt_eq_mut(cache.peek_mut(&"banana"), "green");
  }

  #[test]
  fn test_peek_lru() {
    let mut cache = FrameCache::new();

    assert!(cache.peek_lru().is_none());

    cache.put("apple", "red");
    cache.put("banana", "yellow");
    assert_opt_eq_tuple(cache.peek_lru(), ("apple", "red"));

    cache.get(&"apple");
    assert_opt_eq_tuple(cache.peek_lru(), ("banana", "yellow"));

    cache.clear();
    assert!(cache.peek_lru().is_none());
  }

  #[test]
  fn test_contains() {
    let mut cache = FrameCache::new();

    cache.put("apple", "red");
    cache.put("banana", "yellow");
    cache.put("pear", "green");

    assert!(cache.contains(&"apple"));
    assert!(cache.contains(&"banana"));
    assert!(cache.contains(&"pear"));
  }

  #[test]
  fn test_pop() {
    let mut cache = FrameCache::new();

    cache.put("apple", "red");
    cache.put("banana", "yellow");

    assert_eq!(cache.len(), 2);
    assert_opt_eq(cache.get(&"apple"), "red");
    assert_opt_eq(cache.get(&"banana"), "yellow");

    let popped = cache.pop(&"apple");
    assert!(popped.is_some());
    assert_eq!(popped.unwrap(), "red");
    assert_eq!(cache.len(), 1);
    assert!(cache.get(&"apple").is_none());
    assert_opt_eq(cache.get(&"banana"), "yellow");
  }

  #[test]
  fn test_pop_entry() {
    let mut cache = FrameCache::new();
    cache.put("apple", "red");
    cache.put("banana", "yellow");

    assert_eq!(cache.len(), 2);
    assert_opt_eq(cache.get(&"apple"), "red");
    assert_opt_eq(cache.get(&"banana"), "yellow");

    let popped = cache.pop_entry(&"apple");
    assert!(popped.is_some());
    assert_eq!(popped.unwrap(), ("apple", "red"));
    assert_eq!(cache.len(), 1);
    assert!(cache.get(&"apple").is_none());
    assert_opt_eq(cache.get(&"banana"), "yellow");
  }

  #[test]
  fn test_clear() {
    let mut cache = FrameCache::new();

    cache.put("apple", "red");
    cache.put("banana", "yellow");

    assert_eq!(cache.len(), 2);
    assert_opt_eq(cache.get(&"apple"), "red");
    assert_opt_eq(cache.get(&"banana"), "yellow");

    cache.clear();
    assert_eq!(cache.len(), 0);
  }

  #[test]
  fn test_send() {
    use std::thread;

    let mut cache = FrameCache::new();
    cache.put(1, "a");

    let handle = thread::spawn(move || {
      assert_eq!(cache.get(&1), Some(&"a"));
    });

    assert!(handle.join().is_ok());
  }

  #[test]
  fn test_multiple_threads() {
    let mut pool = Pool::new(1);
    let mut cache = FrameCache::new();
    cache.put(1, "a");

    let cache_ref = &cache;
    pool.scoped(|scoped| {
      scoped.execute(move || {
        assert_eq!(cache_ref.peek(&1), Some(&"a"));
      });
    });

    assert_eq!((cache_ref).peek(&1), Some(&"a"));
  }

  #[test]
  fn test_iter_forwards() {
    let mut cache = FrameCache::new();
    cache.put("a", 1);
    cache.put("b", 2);
    cache.put("c", 3);

    {
      // iter const
      let mut iter = cache.iter();
      assert_eq!(iter.len(), 3);
      assert_opt_eq_tuple(iter.next(), ("c", 3));

      assert_eq!(iter.len(), 2);
      assert_opt_eq_tuple(iter.next(), ("b", 2));

      assert_eq!(iter.len(), 1);
      assert_opt_eq_tuple(iter.next(), ("a", 1));

      assert_eq!(iter.len(), 0);
      assert_eq!(iter.next(), None);
    }
    {
      // iter mut
      let mut iter = cache.iter_mut();
      assert_eq!(iter.len(), 3);
      assert_opt_eq_mut_tuple(iter.next(), ("c", 3));

      assert_eq!(iter.len(), 2);
      assert_opt_eq_mut_tuple(iter.next(), ("b", 2));

      assert_eq!(iter.len(), 1);
      assert_opt_eq_mut_tuple(iter.next(), ("a", 1));

      assert_eq!(iter.len(), 0);
      assert_eq!(iter.next(), None);
    }
  }

  #[test]
  fn test_iter_backwards() {
    let mut cache = FrameCache::new();
    cache.put("a", 1);
    cache.put("b", 2);
    cache.put("c", 3);

    {
      // iter const
      let mut iter = cache.iter();
      assert_eq!(iter.len(), 3);
      assert_opt_eq_tuple(iter.next_back(), ("a", 1));

      assert_eq!(iter.len(), 2);
      assert_opt_eq_tuple(iter.next_back(), ("b", 2));

      assert_eq!(iter.len(), 1);
      assert_opt_eq_tuple(iter.next_back(), ("c", 3));

      assert_eq!(iter.len(), 0);
      assert_eq!(iter.next_back(), None);
    }

    {
      // iter mut
      let mut iter = cache.iter_mut();
      assert_eq!(iter.len(), 3);
      assert_opt_eq_mut_tuple(iter.next_back(), ("a", 1));

      assert_eq!(iter.len(), 2);
      assert_opt_eq_mut_tuple(iter.next_back(), ("b", 2));

      assert_eq!(iter.len(), 1);
      assert_opt_eq_mut_tuple(iter.next_back(), ("c", 3));

      assert_eq!(iter.len(), 0);
      assert_eq!(iter.next_back(), None);
    }
  }

  #[test]
  fn test_iter_forwards_and_backwards() {
    let mut cache = FrameCache::new();
    cache.put("a", 1);
    cache.put("b", 2);
    cache.put("c", 3);

    {
      // iter const
      let mut iter = cache.iter();
      assert_eq!(iter.len(), 3);
      assert_opt_eq_tuple(iter.next(), ("c", 3));

      assert_eq!(iter.len(), 2);
      assert_opt_eq_tuple(iter.next_back(), ("a", 1));

      assert_eq!(iter.len(), 1);
      assert_opt_eq_tuple(iter.next(), ("b", 2));

      assert_eq!(iter.len(), 0);
      assert_eq!(iter.next_back(), None);
    }
    {
      // iter mut
      let mut iter = cache.iter_mut();
      assert_eq!(iter.len(), 3);
      assert_opt_eq_mut_tuple(iter.next(), ("c", 3));

      assert_eq!(iter.len(), 2);
      assert_opt_eq_mut_tuple(iter.next_back(), ("a", 1));

      assert_eq!(iter.len(), 1);
      assert_opt_eq_mut_tuple(iter.next(), ("b", 2));

      assert_eq!(iter.len(), 0);
      assert_eq!(iter.next_back(), None);
    }
  }

  #[test]
  fn test_iter_multiple_threads() {
    let mut pool = Pool::new(1);
    let mut cache = FrameCache::new();
    cache.put("a", 1);
    cache.put("b", 2);
    cache.put("c", 3);

    let mut iter = cache.iter();
    assert_eq!(iter.len(), 3);
    assert_opt_eq_tuple(iter.next(), ("c", 3));

    {
      let iter_ref = &mut iter;
      pool.scoped(|scoped| {
        scoped.execute(move || {
          assert_eq!(iter_ref.len(), 2);
          assert_opt_eq_tuple(iter_ref.next(), ("b", 2));
        });
      });
    }

    assert_eq!(iter.len(), 1);
    assert_opt_eq_tuple(iter.next(), ("a", 1));

    assert_eq!(iter.len(), 0);
    assert_eq!(iter.next(), None);
  }

  #[test]
  fn test_iter_clone() {
    let mut cache = FrameCache::new();
    cache.put("a", 1);
    cache.put("b", 2);

    let mut iter = cache.iter();
    let mut iter_clone = iter.clone();

    assert_eq!(iter.len(), 2);
    assert_opt_eq_tuple(iter.next(), ("b", 2));
    assert_eq!(iter_clone.len(), 2);
    assert_opt_eq_tuple(iter_clone.next(), ("b", 2));

    assert_eq!(iter.len(), 1);
    assert_opt_eq_tuple(iter.next(), ("a", 1));
    assert_eq!(iter_clone.len(), 1);
    assert_opt_eq_tuple(iter_clone.next(), ("a", 1));

    assert_eq!(iter.len(), 0);
    assert_eq!(iter.next(), None);
    assert_eq!(iter_clone.len(), 0);
    assert_eq!(iter_clone.next(), None);
  }

  #[test]
  fn test_into_iter() {
    let mut cache = FrameCache::new();
    cache.put("a", 1);
    cache.put("b", 2);
    cache.put("c", 3);

    let mut iter = cache.into_iter();
    assert_eq!(iter.len(), 3);
    assert_eq!(iter.next(), Some(("a", 1)));

    assert_eq!(iter.len(), 2);
    assert_eq!(iter.next(), Some(("b", 2)));

    assert_eq!(iter.len(), 1);
    assert_eq!(iter.next(), Some(("c", 3)));

    assert_eq!(iter.len(), 0);
    assert_eq!(iter.next(), None);
  }

  #[test]
  fn test_that_pop_actually_detaches_node() {
    let mut cache = FrameCache::new();

    cache.put("a", 1);
    cache.put("b", 2);
    cache.put("c", 3);
    cache.put("d", 4);
    cache.put("e", 5);

    assert_eq!(cache.pop(&"c"), Some(3));

    cache.put("f", 6);

    let mut iter = cache.iter();
    assert_opt_eq_tuple(iter.next(), ("f", 6));
    assert_opt_eq_tuple(iter.next(), ("e", 5));
    assert_opt_eq_tuple(iter.next(), ("d", 4));
    assert_opt_eq_tuple(iter.next(), ("b", 2));
    assert_opt_eq_tuple(iter.next(), ("a", 1));
    assert!(iter.next().is_none());
  }

  #[test]
  fn test_get_with_borrow() {
    let mut cache = FrameCache::new();

    let key = String::from("apple");
    cache.put(key, "red");

    assert_opt_eq(cache.get("apple"), "red");
  }

  #[test]
  fn test_get_mut_with_borrow() {
    let mut cache = FrameCache::new();

    let key = String::from("apple");
    cache.put(key, "red");

    assert_opt_eq_mut(cache.get_mut("apple"), "red");
  }

  #[test]
  fn test_no_memory_leaks() {
    static DROP_COUNT: AtomicUsize = AtomicUsize::new(0);

    struct DropCounter;

    impl Drop for DropCounter {
      fn drop(&mut self) { DROP_COUNT.fetch_add(1, Ordering::SeqCst); }
    }

    let n = 100;
    for _ in 0..n {
      let mut cache = FrameCache::new();
      for i in 0..n {
        cache.put(i, DropCounter {});
      }
    }
    assert_eq!(DROP_COUNT.load(Ordering::SeqCst), n * n);
  }

  #[test]
  fn test_no_memory_leaks_with_clear() {
    static DROP_COUNT: AtomicUsize = AtomicUsize::new(0);

    struct DropCounter;

    impl Drop for DropCounter {
      fn drop(&mut self) { DROP_COUNT.fetch_add(1, Ordering::SeqCst); }
    }

    let n = 100;
    for _ in 0..n {
      let mut cache = FrameCache::new();
      for i in 0..n {
        cache.put(i, DropCounter {});
      }
      cache.clear();
    }
    assert_eq!(DROP_COUNT.load(Ordering::SeqCst), n * n);
  }

  #[test]
  fn test_no_memory_leaks_with_resize() {
    static DROP_COUNT: AtomicUsize = AtomicUsize::new(0);

    struct DropCounter;

    impl Drop for DropCounter {
      fn drop(&mut self) { DROP_COUNT.fetch_add(1, Ordering::SeqCst); }
    }

    let n = 100;
    for _ in 0..n {
      let mut cache = FrameCache::new();
      for i in 0..n {
        cache.put(i, DropCounter {});
      }
      cache.clear();
    }
    assert_eq!(DROP_COUNT.load(Ordering::SeqCst), n * n);
  }

  #[test]
  fn test_no_memory_leaks_with_pop() {
    static DROP_COUNT: AtomicUsize = AtomicUsize::new(0);

    #[derive(Hash, PartialEq, Eq)]
    struct KeyDropCounter(usize);

    impl Drop for KeyDropCounter {
      fn drop(&mut self) { DROP_COUNT.fetch_add(1, Ordering::SeqCst); }
    }

    let n = 100;
    for _ in 0..n {
      let mut cache = FrameCache::new();

      for i in 0..100 {
        cache.put(KeyDropCounter(i), i);
        cache.pop(&KeyDropCounter(i));
      }
    }

    assert_eq!(DROP_COUNT.load(Ordering::SeqCst), n * n * 2);
  }

  #[test]
  fn test_promote_and_demote() {
    let mut cache = FrameCache::new();
    for i in 0..5 {
      cache.push(i, i);
    }
    cache.promote(&1);
    cache.promote(&0);
    cache.demote(&3);
    cache.demote(&4);
    assert_eq!(cache.pop_lru(), Some((4, 4)));
    assert_eq!(cache.pop_lru(), Some((3, 3)));
    assert_eq!(cache.pop_lru(), Some((2, 2)));
    assert_eq!(cache.pop_lru(), Some((1, 1)));
    assert_eq!(cache.pop_lru(), Some((0, 0)));
    assert_eq!(cache.pop_lru(), None);
  }

  #[test]
  fn end_frame_remove_none() {
    let mut cache = FrameCache::new();
    cache.put(1, 1);
    cache.end_frame("");
    cache.end_frame("");
  }
}
