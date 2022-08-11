use crate::error::CircleUsedPath;

use super::{widget_state_ref, DeclareCtx};
use proc_macro2::{Span, TokenStream};
use std::collections::HashMap;
use syn::Ident;

#[derive(Clone, Debug)]
pub struct NameUsedInfo {
  pub used_type: UsedType,
  pub spans: Vec<Span>,
}

bitflags::bitflags! {
 pub struct UsedType: u16 {
    /// named object used but not in closure
    const USED = 0x0001;
    /// named object used in ordinary closure.
    const CAPTURE = 0x0010;
    /// named object used in move closure.
    const MOVE_CAPTURE = 0x0100;
  }
}

#[derive(Clone, Debug)]
pub struct UsedPart<'a> {
  pub skip_nc_cfg: bool,
  pub scope_label: Option<&'a Ident>,
  pub used_info: &'a HashMap<Ident, NameUsedInfo, ahash::RandomState>,
}

#[derive(Clone, Debug)]
pub struct ObjectUsed<'a>(pub Box<[UsedPart<'a>]>);

#[derive(Debug, Default, Clone)]
pub struct ScopeUsedInfo(Option<HashMap<Ident, NameUsedInfo, ahash::RandomState>>);

impl<'a, IntoIter> From<IntoIter> for ObjectUsed<'a>
where
  IntoIter: IntoIterator<Item = UsedPart<'a>>,
{
  #[inline]
  fn from(iter: IntoIter) -> Self { Self(iter.into_iter().collect()) }
}

#[derive(Clone, Debug)]
pub struct ObjectUsedPath<'a> {
  pub obj: &'a Ident,
  pub skip_nc_cfg: bool,
  pub scope_label: Option<&'a Ident>,
  pub used_obj: &'a Ident,
  pub used_info: &'a NameUsedInfo,
}

impl<'a> ObjectUsed<'a> {
  #[inline]
  pub fn from_single_part(part: UsedPart<'a>) -> Self { Self(Box::new([part])) }

  // return the iterator of tuple, the tuple compose by a field and a widget name,
  // the widget name is what the field follow on
  pub fn used_full_path_iter<'r>(
    &'r self,
    self_name: &'r Ident,
  ) -> impl Iterator<Item = ObjectUsedPath<'r>> + 'r {
    self.iter().flat_map(move |p| {
      let &UsedPart { skip_nc_cfg, scope_label, used_info } = p;
      used_info
        .iter()
        .map(move |(used_obj, used_info)| ObjectUsedPath {
          obj: self_name,
          skip_nc_cfg,
          scope_label,
          used_obj,
          used_info,
        })
    })
  }

  pub fn used_obj_iter(&self) -> impl Iterator<Item = &Ident> + '_ {
    self.iter().flat_map(move |p| p.used_info.keys())
  }
}

impl<'a> std::ops::Deref for ObjectUsed<'a> {
  type Target = [UsedPart<'a>];

  fn deref(&self) -> &Self::Target { &*self.0 }
}

impl<'a> std::ops::DerefMut for ObjectUsed<'a> {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut *self.0 }
}

impl<'a> FromIterator<UsedPart<'a>> for ObjectUsed<'a> {
  #[inline]
  fn from_iter<T: IntoIterator<Item = UsedPart<'a>>>(iter: T) -> Self {
    Self(iter.into_iter().collect())
  }
}

impl NameUsedInfo {
  pub fn merge(&mut self, other: &NameUsedInfo) {
    self.used_type |= other.used_type;
    self.spans.extend(&other.spans)
  }
}

impl ScopeUsedInfo {
  #[inline]
  pub fn take(&mut self) -> Self { Self(self.0.take()) }

  pub fn add_used(&mut self, name: Ident, used_type: UsedType) {
    let span = name.span();
    self
      .0
      .get_or_insert_with(<_>::default)
      .entry(name)
      .and_modify(|info| {
        info.used_type |= used_type;
        info.spans.push(span)
      })
      .or_insert_with(|| NameUsedInfo { used_type, spans: vec![span] });
  }

  pub fn merge(&mut self, other: &Self) {
    match (self.0.as_mut(), other.0.as_ref()) {
      (Some(a), Some(b)) => b.iter().for_each(|(name, info)| {
        a.entry(name.clone())
          .and_modify(|i| i.merge(info))
          .or_insert_with(|| info.clone());
      }),
      (None, b @ Some(_)) => self.0 = b.cloned(),
      _ => {}
    }
  }

  pub fn directly_used_widgets(&self) -> Option<impl Iterator<Item = &Ident> + Clone + '_> {
    self.filter_widget(|info| info.used_type.contains(UsedType::USED))
  }

  pub fn refs_widgets(&self) -> Option<impl Iterator<Item = &Ident> + Clone + '_> {
    self.filter_widget(|info| info.used_type != UsedType::MOVE_CAPTURE)
  }

  pub fn refs_tokens(&self) -> Option<impl Iterator<Item = TokenStream> + '_> {
    self.refs_widgets().map(|iter| iter.map(widget_state_ref))
  }

  pub fn iter_mut(&mut self) -> impl Iterator<Item = (&Ident, &mut NameUsedInfo)> {
    self.0.iter_mut().flat_map(|m| m.iter_mut())
  }

  pub fn all_widgets(&self) -> Option<impl Iterator<Item = &Ident> + '_> {
    let used = self.0.as_ref()?;
    (!used.is_empty()).then(|| used.keys())
  }

  pub fn used_part<'a>(
    &'a self,
    scope_label: Option<&'a Ident>,
    skip_nc_cfg: bool,
  ) -> Option<UsedPart> {
    self
      .0
      .as_ref()
      .map(|used_info| UsedPart { scope_label, used_info, skip_nc_cfg })
  }

  pub fn filter_item(
    &self,
    filter: impl Fn(&NameUsedInfo) -> bool + Clone,
  ) -> Option<impl Iterator<Item = (&Ident, &NameUsedInfo)> + Clone> {
    let widgets = self
      .0
      .as_ref()?
      .iter()
      .filter(move |(_, info)| filter(info));

    widgets.clone().next().is_some().then(move || widgets)
  }

  pub fn len(&self) -> usize { self.0.as_ref().map_or(0, |map| map.len()) }

  pub fn get(&self, id: &Ident) -> Option<&NameUsedInfo> {
    self.0.as_ref().and_then(|map| map.get(id))
  }

  fn filter_widget(
    &self,
    filter: impl Fn(&NameUsedInfo) -> bool + Clone,
  ) -> Option<impl Iterator<Item = &Ident> + Clone> {
    self.filter_item(filter).map(|iter| iter.map(|(w, _)| w))
  }
}

impl<'a> ObjectUsedPath<'a> {
  pub fn to_used_path(&self, ctx: &DeclareCtx) -> CircleUsedPath {
    let obj = ctx
      .user_perspective_name(self.obj)
      .unwrap_or_else(|| {
        // same id, but use the one which at the define place to provide more friendly
        // compile error.
        ctx
          .named_objects
          .get_key_value(self.obj)
          .expect("Leak some named object not collect.")
          .0
      })
      .clone();

    let used_obj = ctx.user_perspective_name(self.used_obj).map_or_else(
      || self.used_obj.clone(),
      |user| Ident::new(&user.to_string(), self.used_obj.span()),
    );
    CircleUsedPath {
      obj,
      member: self.scope_label.cloned(),
      used_widget: used_obj,
      used_info: self.used_info.clone(),
    }
  }
}
