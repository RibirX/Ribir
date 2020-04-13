use indextree::*;
use std::fmt::{Debug, Formatter, Result};
pub struct TreeFormatter<'a, T> {
  arena: &'a Arena<T>,
  root: NodeId,
}

impl<'a, T> TreeFormatter<'a, T> {
  #[inline]
  pub fn new(arena: &'a Arena<T>, root: NodeId) -> Self { Self { arena, root } }
}

impl<'a, T: Debug> Debug for TreeFormatter<'a, T> {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    let node_id = self.root;
    let children = 0;
    let level = 0;
    let last = vec![];
    let mut stack = vec![(node_id, children, level, last)];
    while let Some((node_id, childn, level, last)) = stack.pop() {
      debug_assert_eq!(
        last.len(),
        level,
        "each previous level should indicate whether it has reached the last node"
      );
      let node = &self.arena[node_id];
      if childn == 0 {
        for i in 1..level {
          if last[i - 1] {
            write!(f, "    ")?;
          } else {
            write!(f, "│   ")?;
          }
        }
        if level > 0 {
          if last[level - 1] {
            write!(f, "└── ")?;
          } else {
            write!(f, "├── ")?;
          }
        }
        writeln!(f, "{:?}", node.get())?;
      }
      let mut children = node_id.children(&self.arena).skip(childn);
      if let Some(child) = children.next() {
        let mut next_last = last.clone();
        if children.next().is_some() {
          stack.push((node_id, childn + 1, level, last));
          next_last.push(false);
        } else {
          next_last.push(true);
        }
        stack.push((child, 0, level + 1, next_last));
      }
    }
    Ok(())
  }
}
