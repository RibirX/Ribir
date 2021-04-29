use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
  parse_quote, punctuated::Punctuated, token::Comma, Data, DataStruct, Field, Fields, GenericParam,
  Generics, Ident, Type,
};

const STATE_ATTR_NAME: &'static str = "state";

pub(crate) fn state_gen(input: &syn::DeriveInput) -> Option<TokenStream2> {
  match &input.data {
    Data::Struct(s) => {
      let s = StateGen::new(s, &input.generics);
      let name = &input.ident;
      if s.state_fields.len() > 0 {
        let state_name: Ident = syn::parse_str(&format!("{}State", input.ident)).unwrap();
        let state_generis = s.state_generics();
        let (impl_generics, ty_generics, where_clause) = state_generis.split_for_impl();
        let (w_impl_generics, w_ty_generics, w_where_clause) = input.generics.split_for_impl();
        let state_fields = &s.state_fields;

        let vis = input.vis.clone();

        let state_def = if s.is_tuple {
          quote! {

            #[derive(PartialEq)]
            #vis struct #state_name #ty_generics #where_clause (
              #(#state_fields)*
            )
          }
        } else {
          quote! {
            #[derive(Clone, PartialEq)]
            #vis struct #state_name #ty_generics #where_clause {
              #(#state_fields)*
            }
          }
        };

        let state_field_names = state_fields.iter().enumerate().map(|(idx, f)| {
          let name = f
            .ident
            .as_ref()
            .map_or(format!("{}", idx), |ident| format!("{}", ident));
          syn::parse_str::<Ident>(&name).unwrap()
        });

        let state_fn_names = state_field_names
          .clone()
          .map(|name| prefix_ident("state_", &name));

        let state_fy = state_fields.iter().map(|f| &f.ty);

        let stateful_name = prefix_ident("Stateful", &name);
        let expanded = quote! {
          #state_def

          #vis struct #stateful_name #w_ty_generics(Stateful<#name #w_ty_generics>) #w_where_clause;

          // proxy_impl_as_trait!(Stateful<#name #w_ty_generics>, 0);


          // impl #w_impl_generics #stateful_name #w_ty_generics #w_where_clause{
          //   #(
          //     pub fn #state_fn_names(&mut self)
          //       -> impl LocalObservable<'static, Item = StateChange<#state_fy>, Err = ()> {
          //       self.0.state_change(|w| w.#state_field_names.clone())
          //     }
          //   )*
          // }
        };

        println!("{}", expanded);

        Some(expanded)
      } else {
        None
      }
    }
    _ => None,
  }
}

fn prefix_ident(prefix: &str, ident: &Ident) -> Ident {
  syn::parse_str::<Ident>(&format!("{}{}", prefix, ident)).unwrap()
}

struct StateGen<'a> {
  generics: &'a Generics,
  state_fields: Vec<Field>,
  is_tuple: bool,
}

impl<'a> StateGen<'a> {
  fn new(from: &'a DataStruct, generics: &'a Generics) -> Self {
    Self {
      state_fields: Self::state_fields(from),
      generics,
      is_tuple: matches!(from.fields, Fields::Unnamed(_)),
    }
  }

  fn state_fields(stt: &DataStruct) -> Vec<Field> {
    fn pick_state_fields(fds: &Punctuated<Field, Comma>) -> Vec<Field> {
      fds
        .iter()
        .cloned()
        .filter_map(|mut f| {
          let len = f.attrs.len();
          f.attrs.retain(|attr| {
            attr.path.segments.len() == 1 && attr.path.segments[0].ident != STATE_ATTR_NAME
          });
          if f.attrs.len() != len { Some(f) } else { None }
        })
        .collect()
    }

    match stt.fields {
      Fields::Unit => vec![],
      Fields::Unnamed(ref fds) => pick_state_fields(&fds.unnamed),
      Fields::Named(ref fds) => pick_state_fields(&fds.named),
    }
  }

  fn state_generics(&self) -> Generics {
    use syn::WherePredicate;
    let Generics {
      gt_token,
      mut params,
      lt_token,
      mut where_clause,
    } = self.generics.clone();
    params = params
      .iter()
      .map(|p| p.clone())
      .filter(|p| {
        let ident = match p {
          GenericParam::Type(t) => &t.ident,
          GenericParam::Lifetime(l) => &l.lifetime.ident,
          GenericParam::Const(c) => &c.ident,
        };
        self.state_fields.iter().any(|f| type_contain(&f.ty, ident))
      })
      .collect::<Punctuated<_, syn::token::Comma>>();

    where_clause = where_clause.and_then(|mut clause| {
      clause.predicates = clause
        .predicates
        .iter()
        .filter(|p| {
          self.state_fields.iter().any(|f| match p {
            WherePredicate::Lifetime(lf) => type_contain(&f.ty, &lf.lifetime.ident),
            WherePredicate::Type(ty) => f.ty == ty.bounded_ty,
            WherePredicate::Eq(eq) => f.ty == eq.lhs_ty,
          })
        })
        .cloned()
        .collect::<Punctuated<_, syn::token::Comma>>();
      Some(clause)
    });

    let generics = Generics {
      params,
      gt_token,
      lt_token,
      where_clause,
    };

    add_trait_bounds(generics)
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
// Add a bound `T: HeapSize` to every type parameter T.
fn add_trait_bounds(mut generics: Generics) -> Generics {
  for param in &mut generics.params {
    if let GenericParam::Type(ref mut type_param) = *param {
      type_param.bounds.push(parse_quote!(std::clone::Clone));
    }
  }
  generics
}
