use crate::simple_declare_attr::*;
use crate::util::data_struct_unwrap;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::{spanned::Spanned, Ident, Visibility};

const DECLARE: &str = "Declare";

pub(crate) fn declare_derive(input: &mut syn::DeriveInput) -> syn::Result<TokenStream> {
  let syn::DeriveInput { vis, ident: host, generics, data, .. } = input;
  let stt = data_struct_unwrap(data, DECLARE)?;

  if stt.fields.is_empty() {
    return empty_impl(host, &stt.fields);
  }

  let declarer = Declarer::new(host, &mut stt.fields, vis)?;
  let Declarer { name, fields, .. } = &declarer;
  // reverse name check.
  fields.iter().try_for_each(DeclareField::check_reserve)?;
  let set_methods = declarer_set_methods(fields, vis);

  let field_names = declarer.fields.iter().map(DeclareField::member);
  let field_names2 = field_names.clone();

  let (builder_f_names, builder_f_tys) = declarer.declare_names_tys();
  let field_values = field_values(&declarer.fields, host);
  let (g_impl, g_ty, g_where) = generics.split_for_impl();
  let tokens = quote! {
      #vis struct #name #generics #g_where {
        #(#builder_f_names : Option<DeclareInit<#builder_f_tys>>,)*
      }

      impl #g_impl Declare for #host #g_ty #g_where {
        type Builder = #name #g_ty;

        fn declare_builder() -> Self::Builder {
          #name {
            #(#builder_f_names : None ,)*
          }
        }
      }

      impl #g_impl #name #g_ty #g_where {
        #(#set_methods)*
      }

      impl #g_impl DeclareBuilder for #name #g_ty #g_where {
        type Target = State<#host #g_ty>;

        #[inline]
        fn build_declare(mut self, ctx!(): &BuildCtx) -> Self::Target {
          #(#field_values)*
          let mut _ribir_ಠ_ಠ = State::value(#host {
            #(#field_names : #field_names.0),*
          });

          #(
            if let Some(o) = #field_names2.1 {
              let mut _ribir2 = _ribir_ಠ_ಠ.clone_writer();
              let u = o.subscribe(move |(_, v)| _ribir2.write().#field_names2 = v);
              _ribir_ಠ_ಠ.as_stateful().unsubscribe_on_drop(u);
            }
          );*

          _ribir_ಠ_ಠ
        }
      }
  };

  Ok(tokens)
}

fn declarer_set_methods<'a>(
  fields: &'a [DeclareField],
  vis: &'a Visibility,
) -> impl Iterator<Item = TokenStream> + 'a {
  fields.iter().filter(|f| f.need_set_method()).map(move |f| {
    let field_name = f.field.ident.as_ref().unwrap();
    let ty = &f.field.ty;
    let set_method = f.set_method_name();
    if f.attr.as_ref().map_or(false, |attr| attr.strict.is_some()) {
      quote! {
        #[inline]
        #vis fn #set_method(mut self, v: #ty) -> Self {
          self.#field_name = Some(DeclareInit::Value(v));
          self
        }
      }
    } else {
      quote! {
        #[inline]
        #vis fn #set_method<_M, _V>(mut self, v: _V) -> Self
          where DeclareInit<#ty>: DeclareFrom<_V, _M>
        {
          self.#field_name = Some(DeclareInit::declare_from(v));
          self
        }
      }
    }
  })
}

fn field_values<'a>(
  fields: &'a [DeclareField],
  stt_name: &'a Ident,
) -> impl Iterator<Item = TokenStream> + 'a {
  fields.iter().map(move |f| {
    let f_name = f.member();
    let ty = &f.field.ty;

    let v = if f.is_not_skip() {
      if let Some(df) = f.default_value() {
        quote! {
          self.#f_name.take().map_or_else(
            || (#df, None),
            |v| v.unzip()
          )
        }
      } else {
        let err = format!("Required field `{stt_name}::{f_name}` not set");
        quote! { self.#f_name.expect(#err).unzip() }
      }
    } else {
      // skip field must have default value.
      let df = f.default_value().unwrap();
      quote! { (#df, None) }
    };
    quote_spanned! { f.field.span() => let #f_name: (#ty, Option<ValueStream<#ty>>) = #v; }
  })
}

impl<'a> DeclareField<'a> {
  fn check_reserve(&self) -> syn::Result<()> {
    // reverse name check.
    let member = self.member();
    if self
      .attr
      .as_ref()
      .map_or(false, |attr| attr.builtin.is_some())
    {
      return Ok(());
    }

    if let Some(r) = crate::variable_names::BUILTIN_INFOS.get(member.to_string().as_str()) {
      let mut field = self.field.clone();
      // not display the attrs in the help code.
      field.attrs.clear();

      let msg = format!(
        "the identifier `{}` is reserved for `{}`
To avoid name conflicts during declaration, use the `rename` meta, like so:
``` 
#[declare(rename = new_name)],
{}
```
",
        member,
        &r.host_ty,
        field.to_token_stream()
      );
      Err(syn::Error::new_spanned(field, msg))
    } else {
      Ok(())
    }
  }
}
