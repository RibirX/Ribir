use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::{ToTokens, quote, quote_spanned};
use syn::{Fields, Ident, Visibility, spanned::Spanned};

use crate::{
  simple_declare_attr::*,
  util::data_struct_unwrap,
  variable_names::{BUILTIN_INFOS, BuiltinMemberType},
};

const DECLARE: &str = "Declare";

pub(crate) fn declare_derive(input: &mut syn::DeriveInput) -> syn::Result<TokenStream> {
  let syn::DeriveInput { vis, ident: host, generics, data, .. } = input;
  let stt = data_struct_unwrap(data, DECLARE)?;

  let mut tokens: TokenStream = if stt.fields.is_empty() {
    empty_impl(host, &stt.fields)
  } else {
    let extend_declare = Ident::new(&format!("{host}DeclareExtend"), host.span());
    let declarer = Declarer::new(host, &mut stt.fields)?;
    let Declarer { name, fields, .. } = &declarer;
    // reverse name check.
    fields
      .iter()
      .try_for_each(DeclareField::check_reserve)?;
    let set_methods = declarer_set_methods(fields);

    let field_names = declarer.fields.iter().map(DeclareField::member);
    let field_names2 = field_names.clone();

    let (builder_f_names, builder_f_tys) = declarer.declare_names_tys();
    let field_values = field_values(&declarer.fields, host);
    let (g_impl, g_ty, g_where) = generics.split_for_impl();
    quote! {
     #vis struct #name #generics #g_where {
       #(
         #[allow(clippy::type_complexity)]
         #builder_f_names : Option<DeclareInit<#builder_f_tys>>,
       )*
     }

     impl #g_impl Declare for #host #g_ty #g_where {
       type Builder = FatObj<#name #g_ty>;

       fn declarer() -> Self::Builder {
         FatObj::new(#name { #(#builder_f_names : None ,)* })
       }
     }

     impl #g_impl FatDeclarerExtend for #name #g_ty #g_where {
       type Target = State<#host #g_ty>;

       fn finish(mut fat_ಠ_ಠ: FatObj<Self>) -> FatObj<Self::Target> {
         #(#field_values)*
         let this_ಠ_ಠ = State::value(#host {
           #(#field_names : #field_names.0),*
         });
         #(
           if let Some(o) = #field_names2.1 {
             let this_ಠ_ಠ = this_ಠ_ಠ.clone_writer();
             let u = o.subscribe(move |(_, v)| this_ಠ_ಠ.write().#field_names2 = v);
             fat_ಠ_ಠ = fat_ಠ_ಠ.on_disposed(move |_| u.unsubscribe());
           }
         );*

         fat_ಠ_ಠ.map(move |_| this_ಠ_ಠ)
       }
     }

     #vis trait #extend_declare #g_ty: Sized #g_where {
      fn inner(&mut self) -> &mut #name #g_ty;

      #(#set_methods)*
     }

     impl #g_impl #extend_declare #g_ty for FatObj<#name #g_ty> #g_where {
        #[inline(always)]
       fn inner(&mut self) -> &mut #name #g_ty { &mut **self }
     }
    }
  };

  widget_macro_to_tokens(host, vis, &mut tokens);

  Ok(tokens)
}

fn widget_macro_to_tokens(name: &Ident, vis: &Visibility, tokens: &mut TokenStream) {
  let macro_name = name.to_string().to_snake_case();
  let doc =
    format!("Macro used to generate a function widget using `{}` as the root widget.", macro_name);
  let macro_name = Ident::new(&macro_name, name.span());
  let export_attr = if matches!(vis, Visibility::Public(_)) {
    quote! { #[macro_export] }
  } else {
    quote! { #[allow(unused_macros)] }
  };
  tokens.extend(quote! {
    #[allow(unused_macros)]
    #export_attr
    #[doc = #doc]
    macro_rules! #macro_name {
      ($($t: tt)*) => {
        fn_widget! { @ #name { $($t)* } }
      };
    }
    #[allow(unused_imports)]
    #vis use #macro_name;
  })
}

fn declarer_set_methods<'a>(fields: &'a [DeclareField]) -> impl Iterator<Item = TokenStream> + 'a {
  fields
    .iter()
    .filter(|f| f.need_set_method())
    .map(move |f| {
      let field_name = f.field.ident.as_ref().unwrap();
      let doc = f.doc_attr();
      let ty = &f.field.ty;
      let set_method = f.set_method_name();
      if f
        .attr
        .as_ref()
        .is_some_and(|attr| attr.strict.is_some())
      {
        quote! {
          #[inline]
          #doc
          fn #set_method(mut self, v: #ty) -> Self {
            self.inner().#field_name = Some(DeclareInit::Value(v));
            self
          }
        }
      } else {
        quote! {
          #[inline]
          #[allow(clippy::type_complexity)]
          #doc
          fn #set_method<const _M: usize>(mut self, v: impl DeclareInto<#ty, _M>) -> Self {
            self.inner().#field_name = Some(v.declare_into());
            self
          }
        }
      }
    })
}

fn field_values<'a>(
  fields: &'a [DeclareField], stt_name: &'a Ident,
) -> impl Iterator<Item = TokenStream> + 'a {
  fields.iter().map(move |f| {
    let f_name = f.member();
    let ty = &f.field.ty;

    let v = if f.is_not_skip() {
      if let Some(df) = f.default_value() {
        quote! {
          Option::take(&mut fat_ಠ_ಠ.#f_name).map_or_else(
            || (#df, None),
            |v| v.unzip()
          )
        }
      } else {
        let err = format!("Required field `{stt_name}::{f_name}` not set");
        quote! { Option::take(&mut fat_ಠ_ಠ.#f_name).expect(#err).unzip() }
      }
    } else {
      // skip field must have default value.
      let df = f.default_value().unwrap();
      quote! { (#df, None) }
    };
    quote_spanned! { f.field.span() =>
      #[allow(clippy::type_complexity)]
      let #f_name: (#ty, Option<ValueStream<#ty>>) = #v;
    }
  })
}

impl<'a> DeclareField<'a> {
  fn check_reserve(&self) -> syn::Result<()> {
    let member = self.member();
    if let Some(r) = BUILTIN_INFOS
      .get(member.to_string().as_str())
      .filter(|info| info.mem_ty == BuiltinMemberType::Field)
    {
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

fn empty_impl(name: &Ident, fields: &Fields) -> TokenStream {
  let construct = match fields {
    Fields::Named(_) => quote!(#name {}),
    Fields::Unnamed(_) => quote!(#name()),
    Fields::Unit => quote!(#name),
  };
  quote! {
    impl Declare for #name  {
      type Builder = FatObj<#name>;
      fn declarer() -> Self::Builder { FatObj::new(#construct) }
    }

    impl FatDeclarerExtend for #name {
      type Target = #name;
      fn finish(this: FatObj<#name>) -> FatObj<#name> { this }
    }
  }
}
