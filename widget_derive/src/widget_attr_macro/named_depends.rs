use std::collections::{HashMap, HashSet};

use super::{
  animations::{Animate, State, Transition},
  dataflows::Dataflow,
  DeclareField,
};
use proc_macro2::Span;
use syn::Ident;

#[derive(Clone, Debug)]
pub struct DependPlaceInfo {
  pub widget: Ident,
  pub spans: Box<[Span]>,
}

#[derive(Clone, Debug)]
pub struct DependPart<'a> {
  pub origin: DependIn<'a>,
  pub place_info: &'a [DependPlaceInfo],
}
#[derive(Clone, Debug)]
pub struct Depends<'a>(Box<[DependPart<'a>]>);

#[derive(Clone, Copy, Debug)]
pub enum DependIn<'a> {
  Field(&'a DeclareField),
  DataFlow(&'a Dataflow),
  Animate(&'a Animate),
  State(&'a State),
  Transition(&'a Transition),
}

impl<'a, IntoIter> From<IntoIter> for Depends<'a>
where
  IntoIter: IntoIterator<Item = DependPart<'a>>,
{
  #[inline]
  fn from(iter: IntoIter) -> Self { Self(iter.into_iter().collect()) }
}

impl<'a> Depends<'a> {
  #[inline]
  pub fn from_single_part(part: DependPart<'a>) -> Self { Self(Box::new([part])) }

  // return the iterator of tuple, the tuple compose by a field and a widget name,
  // the widget name is what the field follow on
  pub fn follow_iter(&self) -> impl Iterator<Item = (DependIn, &DependPlaceInfo)> {
    self
      .iter()
      .flat_map(|p| p.place_info.iter().map(|f| (p.origin, f)))
  }
}

impl<'a> std::ops::Deref for Depends<'a> {
  type Target = [DependPart<'a>];

  fn deref(&self) -> &Self::Target { &*self.0 }
}

impl<'a> std::ops::DerefMut for Depends<'a> {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut *self.0 }
}

impl<'a> FromIterator<DependPart<'a>> for Depends<'a> {
  #[inline]
  fn from_iter<T: IntoIterator<Item = DependPart<'a>>>(iter: T) -> Self {
    Self(iter.into_iter().collect())
  }
}

pub trait MergeDepends<'a> {
  fn merge_depends(self) -> Option<Vec<DependPlaceInfo>>;
  fn unique_widget(self) -> Option<HashSet<&'a Ident>>;
}

impl<'a, T> MergeDepends<'a> for T
where
  T: Iterator<Item = &'a Vec<DependPlaceInfo>>,
{
  fn merge_depends(self) -> Option<Vec<DependPlaceInfo>> {
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
        .map(|(widget, spans)| DependPlaceInfo {
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
