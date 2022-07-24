pub struct IdMap<T> {
  array: Vec<Node<T>>,
  recovery: Vec<Id>,
}

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub struct Id {
  idx: usize,
  stamp: usize,
}

struct Node<T> {
  data: Option<T>,
  stamp: usize,
}

impl<T> IdMap<T> {
  pub fn insert(&mut self, value: T) -> Id {
    if let Some(mut id) = self.recovery.pop() {
      id.stamp += 1;
      let node = &mut self.array[id.idx];
      assert!(node.data.is_none());
      node.data = Some(value);
      node.stamp = id.stamp;
      id
    } else {
      let id = Id { idx: self.array.len(), stamp: 0 };
      self.array.push(Node { data: Some(value), stamp: id.stamp });
      id
    }
  }

  pub fn get(&mut self, id: Id) -> Option<&T> {
    self.array.get(id.idx).and_then(|node| {
      id.stamp_panic(node.stamp);
      node.data.as_ref()
    })
  }

  pub fn get_mut(&mut self, id: Id) -> Option<&mut T> {
    self.array.get_mut(id.idx).and_then(|node| {
      id.stamp_panic(node.stamp);
      node.data.as_mut()
    })
  }

  pub fn remove(&mut self, id: Id) -> Option<T> {
    self.array.get_mut(id.idx).and_then(|node| {
      id.stamp_panic(node.stamp);
      if id.stamp < usize::MAX {
        self.recovery.push(id)
      }
      node.data.take()
    })
  }
}

impl<T> std::ops::Index<Id> for IdMap<T> {
  type Output = T;

  #[inline]
  fn index(&self, id: Id) -> &Self::Output {
    let node = &self.array[id.idx];
    id.stamp_panic(node.stamp);
    node.data.as_ref().unwrap()
  }
}

impl<T> std::ops::IndexMut<Id> for IdMap<T> {
  #[inline]
  fn index_mut(&mut self, id: Id) -> &mut Self::Output {
    let node = &mut self.array[id.idx];
    id.stamp_panic(node.stamp);
    node.data.as_mut().unwrap()
  }
}

impl Id {
  fn stamp_panic(&self, expected_stamp: usize) {
    assert_eq!(self.stamp, expected_stamp, "Use an invalid identify.");
  }
}

impl<T> Default for IdMap<T> {
  #[inline]
  fn default() -> Self {
    Self {
      array: Default::default(),
      recovery: Default::default(),
    }
  }
}

#[cfg(test)]
mod tests {

  use super::*;

  #[test]
  fn insert_and_access() {
    let mut map = IdMap::default();
    let id = map.insert(0);
    assert_eq!(map[id], 0);

    assert_eq!(map.get(id), Some(&0));
  }

  #[test]
  fn node_reuse() {
    let mut map = IdMap::default();
    let old_id = map.insert(0);
    assert_eq!(map.remove(old_id), Some(0));
    assert!(!map.recovery.is_empty());

    let new_id = map.insert(1);
    assert_eq!(new_id.idx, old_id.idx);
    assert_ne!(new_id.stamp, old_id.stamp);
  }
}
