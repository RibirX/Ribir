use std::collections::{HashMap, HashSet};

use super::{
  animations::{Animate, State, Transition},
  dataflows::Dataflow,
  DeclareField,
};
use proc_macro2::Span;
use syn::Ident;

#[derive(Clone, Debug)]
pub struct NameUsedSpans {
  pub widget: Ident,
  pub spans: Box<[Span]>,
}

#[derive(Clone, Debug)]
pub struct UsedPart<'a> {
  pub origin: UsedScope<'a>,
  pub place_info: &'a [NameUsedSpans],
}
#[derive(Clone, Debug)]
pub struct NameUsed<'a>(Box<[UsedPart<'a>]>);

#[derive(Clone, Copy, Debug)]
pub enum UsedScope<'a> {
  Field(&'a DeclareField),
  DataFlow(&'a Dataflow),
  Animate(&'a Animate),
  State(&'a State),
  Transition(&'a Transition),
}

impl<'a, IntoIter> From<IntoIter> for NameUsed<'a>
where
  IntoIter: IntoIterator<Item = UsedPart<'a>>,
{
  #[inline]
  fn from(iter: IntoIter) -> Self { Self(iter.into_iter().collect()) }
}

impl<'a> NameUsed<'a> {
  #[inline]
  pub fn from_single_part(part: UsedPart<'a>) -> Self { Self(Box::new([part])) }

  // return the iterator of tuple, the tuple compose by a field and a widget name,
  // the widget name is what the field follow on
  pub fn follow_iter(&self) -> impl Iterator<Item = (UsedScope, &NameUsedSpans)> {
    self
      .iter()
      .flat_map(|p| p.place_info.iter().map(|f| (p.origin, f)))
  }
}

impl<'a> std::ops::Deref for NameUsed<'a> {
  type Target = [UsedPart<'a>];

  fn deref(&self) -> &Self::Target { &*self.0 }
}

impl<'a> std::ops::DerefMut for NameUsed<'a> {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut *self.0 }
}

impl<'a> FromIterator<UsedPart<'a>> for NameUsed<'a> {
  #[inline]
  fn from_iter<T: IntoIterator<Item = UsedPart<'a>>>(iter: T) -> Self {
    Self(iter.into_iter().collect())
  }
}

pub trait MergeDepends<'a> {
  fn merge_depends(self) -> Option<Vec<NameUsedSpans>>;
  fn unique_widget(self) -> Option<HashSet<&'a Ident>>;
}

impl<'a, T> MergeDepends<'a> for T
where
  T: Iterator<Item = &'a Vec<NameUsedSpans>>,
{
  fn merge_depends(self) -> Option<Vec<NameUsedSpans>> {
    let mut iter = self.into_iter();
    let first = iter.next()?;
    if let Some(second) = iter.next() {
      let mut map: HashMap<Ident, Vec<Span>, ahash::RandomState> = <_>::default();
      std::iter::once(second)
        .chain(iter)
        .flat_map(|elem| elem.into_iter())
        .for_each(|info| {
          map
            .entry(info.widget.clone())
            .or_default()
            .extend(info.spans.into_iter());
        });
      let info = map
        .into_iter()
        .map(|(widget, spans)| NameUsedSpans {
          widget,
          spans: spans.into_boxed_slice(),
        })
        .collect::<Vec<_>>();
      Some(info)
    } else {
      Some(first.clone())
    }
  }

  fn unique_widget(self) -> Option<HashSet<&'a Ident>> {
    let set = self
      .flat_map(|elem| elem.iter())
      .map(|p| &p.widget)
      .collect();
    Some(set)
  }
}
