use super::desugar::NamedObjMap;
use crate::{error::CircleUsedPath, widget_macro::desugar::NamedObj};
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use std::collections::HashMap;
use syn::Ident;

#[derive(Clone, Debug)]
pub struct NameUsedInfo {
  pub used_type: UsedType,
  pub spans: Vec<Span>,
}

bitflags::bitflags! {
 pub struct UsedType: u16 {
    /// subscribe the named object modifies.
    const SUBSCRIBE = 0x0001;
    /// directly use it.
    const REF = 0x0010;
    /// named object be used and subscribe, the default behavior.
    const USED = Self::SUBSCRIBE.bits() | Self::REF.bits();
    /// named object used in inner scope with needn't provide `state_ref` by outside.
    const SCOPE_CAPTURE = 0x1000;
  }
}

#[derive(Clone, Debug)]
pub struct UsedPart<'a> {
  pub scope_label: Option<&'a Ident>,
  pub used_info: &'a HashMap<Ident, NameUsedInfo, ahash::RandomState>,
}

#[derive(Clone, Debug)]
pub struct ObjectUsed<'a>(pub Vec<UsedPart<'a>>);

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
  pub scope_label: Option<&'a Ident>,
  pub used_obj: &'a Ident,
  pub used_info: &'a NameUsedInfo,
}

impl<'a> ObjectUsed<'a> {
  // return the iterator of tuple, the tuple compose by a field and a widget name,
  // the widget name is what the field follow on
  pub fn used_full_path_iter<'r>(
    &'r self,
    self_name: &'r Ident,
  ) -> impl Iterator<Item = ObjectUsedPath<'r>> + 'r {
    self.iter().flat_map(move |p| {
      let &UsedPart { scope_label, used_info } = p;
      used_info
        .iter()
        .map(move |(used_obj, used_info)| ObjectUsedPath {
          obj: self_name,
          scope_label,
          used_obj,
          used_info,
        })
    })
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

  pub fn upstream_modifies_tokens(&self, raw: bool) -> Option<TokenStream> {
    self.subscribe_widget().map(|directly_used| {
      let modifies = if raw {
        quote! { raw_modifies }
      } else {
        quote! {modifies}
      };
      let upstream = directly_used.clone().map(|w| {
        quote_spanned! { w.span() => #w.#modifies() }
      });
      if directly_used.count() > 1 {
        quote! {  observable::from_iter([#(#upstream),*]).merge_all(usize::MAX) }
      } else {
        quote! { #(#upstream)* }
      }
    })
  }

  pub fn subscribe_widget(&self) -> Option<impl Iterator<Item = &Ident> + Clone + '_> {
    self.filter_widget(|info| info.used_type.contains(UsedType::SUBSCRIBE))
  }

  pub fn ref_widgets(&self) -> Option<impl Iterator<Item = &Ident> + Clone + '_> {
    self.filter_widget(|info| info.used_type.contains(UsedType::REF))
  }

  pub fn prepend_bundle_refs<'a>(&self, tokens: &mut TokenStream) {
    if let Some(names) = self.ref_widgets() {
      let c_names = names.clone();
      if names.clone().nth(1).is_some() {
        quote! {let _guard = (#(#names.modify_guard(),)*);}.to_tokens(tokens);
      }
      c_names.clone().for_each(|n| {
        quote_spanned! {n.span() => let mut #n = #n.state_ref();}.to_tokens(tokens);
      });
    }
  }

  pub fn iter_mut(&mut self) -> impl Iterator<Item = (&Ident, &mut NameUsedInfo)> {
    self.0.iter_mut().flat_map(|m| m.iter_mut())
  }

  pub fn all_used(&self) -> Option<impl Iterator<Item = &Ident> + Clone + '_> {
    let used = self.0.as_ref()?;
    (!used.is_empty()).then(|| used.keys())
  }

  pub fn used_part<'a>(&'a self, scope_label: Option<&'a Ident>) -> Option<UsedPart> {
    self
      .0
      .as_ref()
      .map(|used_info| UsedPart { scope_label, used_info })
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

  fn filter_widget(
    &self,
    filter: impl Fn(&NameUsedInfo) -> bool + Clone,
  ) -> Option<impl Iterator<Item = &Ident> + Clone> {
    self.filter_item(filter).map(|iter| iter.map(|(w, _)| w))
  }
}

impl<'a> ObjectUsedPath<'a> {
  pub fn to_used_path(&self, declare_objs: &NamedObjMap) -> CircleUsedPath {
    fn src_name<'a>(name: &Ident, declare_objs: &'a NamedObjMap) -> Option<&'a Ident> {
      declare_objs.get(name).map(|obj| match obj {
        NamedObj::Host(obj) => &obj.name,
        NamedObj::Builtin { src_name, .. } => src_name,
      })
    }
    let obj = src_name(&self.obj, declare_objs)
      .unwrap_or_else(|| {
        if self.scope_label.is_none() {
          self.obj
        } else {
          // same id, but use the one which at the define place to provide more friendly
          // compile error.
          declare_objs
            .get_name_obj(self.obj)
            .expect("Some named object not collect.")
            .0
        }
      })
      .clone();

    let used_obj = src_name(self.used_obj, declare_objs).map_or_else(
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
