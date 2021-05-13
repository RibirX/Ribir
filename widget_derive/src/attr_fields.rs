use std::usize;

use proc_macro2::Span;
use syn::{
  punctuated::Punctuated, token::Comma, DataStruct, Field, Fields, GenericParam, Generics, Ident,
  Type, TypeParamBound, WherePredicate,
};

fn prefix_ident(prefix: &str, ident: &Ident) -> Ident {
  Ident::new(&format!("{}{}", prefix, ident), Span::call_site())
}

/// Pick fields from struct by specify inner attr.
pub struct AttrFields<'a> {
  generics: &'a Generics,
  attr_fields: Vec<(Field, usize)>,
  is_tuple: bool,
}

impl<'a> AttrFields<'a> {
  pub fn new(from: &'a DataStruct, generics: &'a Generics, attr_name: &'static str) -> Self {
    Self {
      attr_fields: Self::pick_attr_fields(from, attr_name),
      generics,
      is_tuple: matches!(from.fields, Fields::Unnamed(_)),
    }
  }

  fn pick_attr_fields(stt: &DataStruct, attr_name: &'static str) -> Vec<(Field, usize)> {
    let pick_state_fields = |fds: &Punctuated<Field, Comma>| -> Vec<(Field, usize)> {
      fds
        .iter()
        .enumerate()
        .filter_map(|(idx, f)| {
          let mut f = f.clone();
          let len = f.attrs.len();
          f.attrs.retain(|attr| {
            attr.path.segments.len() == 1 && attr.path.segments[0].ident != attr_name
          });
          if f.attrs.len() != len {
            Some((f, idx))
          } else {
            None
          }
        })
        .collect()
    };

    match stt.fields {
      Fields::Unit => vec![],
      Fields::Unnamed(ref fds) => pick_state_fields(&fds.unnamed),
      Fields::Named(ref fds) => pick_state_fields(&fds.named),
    }
  }

  pub fn attr_fields_generics(&self) -> Generics {
    let Generics {
      gt_token,
      mut params,
      lt_token,
      mut where_clause,
    } = self.generics.clone();

    params = params
      .iter()
      .cloned()
      .filter(|p| self.is_attr_generic(p))
      .collect::<Punctuated<_, syn::token::Comma>>();

    where_clause = where_clause.map(|mut clause| {
      clause.predicates = clause
        .predicates
        .iter()
        .filter(|p| self.is_attr_clause(p))
        .cloned()
        .collect::<Punctuated<_, syn::token::Comma>>();
      clause
    });

    Generics {
      lt_token,
      params,
      gt_token,
      where_clause,
    }
  }

  pub fn attr_fields(&self) -> &[(Field, usize)] { &self.attr_fields }

  pub fn is_attr_generic(&self, param: &GenericParam) -> bool {
    let ident = match param {
      GenericParam::Type(t) => &t.ident,
      GenericParam::Lifetime(l) => &l.lifetime.ident,
      GenericParam::Const(c) => &c.ident,
    };
    self
      .attr_fields
      .iter()
      .any(|(f, _)| type_contain(&f.ty, ident))
  }

  fn is_attr_clause(&self, where_predicate: &WherePredicate) -> bool {
    self.attr_fields.iter().any(|(f, _)| match where_predicate {
      WherePredicate::Lifetime(lf) => type_contain(&f.ty, &lf.lifetime.ident),
      WherePredicate::Type(ty) => f.ty == ty.bounded_ty,
      WherePredicate::Eq(eq) => f.ty == eq.lhs_ty,
    })
  }
}

fn type_contain(ty: &Type, generic_ident: &Ident) -> bool {
  use syn::{GenericArgument, PathArguments, ReturnType};
  fn return_type_contain(ret: &ReturnType, ident: &Ident) -> bool {
    match ret {
      ReturnType::Default => false,
      ReturnType::Type(_, t) => type_contain(&t, &ident),
    }
  }

  fn any_contain(types: &Punctuated<Type, Comma>, ident: &Ident) -> bool {
    types.iter().any(|t| type_contain(&t, &ident))
  }

  let res = match ty {
    Type::Reference(ty_ref) => ty_ref
      .lifetime
      .as_ref()
      .map(|l| &l.ident == generic_ident)
      .unwrap_or(false),
    Type::Slice(slice) => type_contain(&slice.elem, generic_ident),
    Type::Array(arr) => type_contain(&arr.elem, generic_ident),
    Type::Ptr(ptr) => type_contain(&ptr.elem, generic_ident),
    Type::BareFn(bare_fn) => {
      return_type_contain(&bare_fn.output, generic_ident)
        || bare_fn
          .inputs
          .iter()
          .any(|arg| type_contain(&arg.ty, generic_ident))
    }
    Type::Never(_) => false,
    Type::Tuple(tuple) => any_contain(&tuple.elems, &generic_ident),
    Type::Path(path) => path.path.segments.iter().any(|seg| {
      &seg.ident == generic_ident
        || match seg.arguments {
          PathArguments::AngleBracketed(ref args) => args.args.iter().any(|arg| match arg {
            GenericArgument::Lifetime(ref lifetime) => &lifetime.ident == generic_ident,
            GenericArgument::Type(ref ty) => type_contain(ty, generic_ident),
            _ => false,
          }),
          PathArguments::Parenthesized(ref func) => {
            return_type_contain(&func.output, generic_ident)
              || any_contain(&func.inputs, generic_ident)
          }
          _ => false,
        }
    }),
    Type::ImplTrait(_) => false,
    Type::TraitObject(_trait_obj) => {
      unimplemented!("TraitObject type cannot derive as state yet")
    }
    Type::Paren(_paren) => {
      unimplemented!("Paren  type cannot derive as state yet")
    }
    Type::Group(_group) => {
      unimplemented!("Group type cannot derive as state yet")
    }
    Type::Macro(_macro) => {
      unimplemented!("Macro type cannot derive as state yet")
    }
    Type::Verbatim(_verbatim) => {
      unimplemented!("Verbatim type cannot derive as state yet")
    }
    Type::Infer(_) => unreachable!(),
    Type::__TestExhaustive(_) => unreachable!(),
  };
  res
}

pub fn add_trait_bounds_if(
  mut generics: Generics,
  bound: TypeParamBound,
  func: impl Fn(&GenericParam) -> bool,
) -> Generics {
  for param in &mut generics.params {
    if func(param) {
      if let GenericParam::Type(ref mut type_param) = *param {
        type_param.bounds.push(bound.clone());
      }
    }
  }
  generics
}
