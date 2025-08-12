use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::{ToTokens, quote, quote_spanned};
use syn::{Ident, Visibility, spanned::Spanned};

use crate::simple_declare_attr::*;

pub(crate) fn declare_derive(stt: &mut syn::ItemStruct) -> syn::Result<TokenStream> {
  let declarer = Declarer::new(stt)?;

  let Declarer { name, fields, original, .. } = &declarer;
  let syn::ItemStruct { vis, ident: host, generics, .. } = &original;

  let set_methods = declarer_set_methods(vis, fields);
  let (g_impl, g_ty, g_where) = generics.split_for_impl();
  let builder_members = declarer.builder_members();
  let builder_members_2 = declarer.builder_members();
  let builder_tys = declarer.builder_tys();

  let mut tokens = quote! {
   #vis struct #name #generics #g_where {
     fat_ಠ_ಠ: FatObj<()>,
     _marker: std::marker::PhantomData<#host #g_ty>,
     #(
       #[allow(clippy::type_complexity)]
       #builder_members : Option<PipeValue<#builder_tys>>,
     )*
   }

   impl #g_impl Declare for #host #g_ty #g_where {
     type Builder = #name #g_ty;

     fn declarer() -> Self::Builder {
      #name {
        fat_ಠ_ಠ: FatObj::new(()),
        _marker: std::marker::PhantomData,
        #(#builder_members_2: None ,)*
      }
     }
   }

   impl #g_impl #name #g_ty #g_where {
      #(#set_methods)*
   }
  };

  if fields.is_empty() {
    let finish_obj = declarer.finish_obj(std::iter::empty());
    tokens.extend(quote! {
      impl #g_impl ObjDeclarer for #name #g_ty #g_where {
        type Target = FatObj<#host #g_ty>;

        #[track_caller]
        fn finish(mut self) -> Self::Target {
          self.fat_ಠ_ಠ.map(|_| #finish_obj)
        }
      }
    });
  } else {
    let field_names = declarer.all_members();
    let field_values = field_values(&declarer);
    let finish_obj = declarer.finish_obj(declarer.all_members().map(|m| quote! {#m.0}));
    tokens.extend(quote! {
      impl #g_impl ObjDeclarer for #name #g_ty #g_where {
        type Target = FatObj<Stateful<#host #g_ty>>;

        #[track_caller]
        fn finish(mut self) -> Self::Target {
          #(#field_values)*
          let this_ಠ_ಠ = Stateful::new(#finish_obj);
          let mut fat_ಠ_ಠ = self.fat_ಠ_ಠ;
          #(
            if let Some(o) = #field_names.1 {
              let this_ಠ_ಠ = this_ಠ_ಠ.clone_writer();
              let u = o.subscribe(move |v| this_ಠ_ಠ.write().#field_names = v);
              fat_ಠ_ಠ.on_disposed(move |_| u.unsubscribe());
            }
          );*

          fat_ಠ_ಠ.map(move |_| this_ಠ_ಠ)
        }
      }
    })
  }

  deref_fat_obj(&declarer).to_tokens(&mut tokens);
  widget_macro_to_tokens(host, vis).to_tokens(&mut tokens);

  Ok(tokens)
}

fn widget_macro_to_tokens(name: &Ident, vis: &Visibility) -> TokenStream {
  let macro_name = name.to_string().to_snake_case();
  let doc =
    format!("Macro used to generate a function widget using `{}` as the root widget.", macro_name);
  let macro_name = Ident::new(&macro_name, name.span());
  let export_attr = if matches!(vis, Visibility::Public(_)) {
    quote! { #[macro_export] }
  } else {
    quote! { #[allow(unused_macros)] }
  };
  quote! {
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
  }
}

fn declarer_set_methods<'a>(
  vis: &'a Visibility, fields: &'a [DeclareField],
) -> impl Iterator<Item = TokenStream> + 'a {
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
          #vis fn #set_method(&mut self, v: #ty) -> &mut Self {
            self.#field_name = Some(PipeValue::Value(v));
            self
          }
        }
      } else {
        quote! {
          #[inline]
          #[allow(clippy::type_complexity)]
          #doc
          #vis fn #set_method<_K: ?Sized>(&mut self, v: impl RInto<PipeValue<#ty>, _K>)
            -> &mut Self
          {
            self.#field_name = Some(v.r_into());
            self
          }
        }
      }
    })
}

fn field_values<'a>(declarer: &'a Declarer) -> impl Iterator<Item = TokenStream> + 'a {
  let host = declarer.host();
  declarer.fields.iter().map(move |f| {
    let f_name = f.member();
    let ty = &f.field.ty;

    let v = if f.is_not_skip() {
      if let Some(df) = f.default_value() {
        quote! {
          Option::take(&mut self.#f_name).map_or_else(
            || (#df, None),
            |v| v.unzip()
          )
        }
      } else {
        let err = format!("Required field `{host}::{f_name}` not set");
        quote! { Option::take(&mut self.#f_name).expect(#err).unzip() }
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

fn deref_fat_obj(declarer: &Declarer) -> TokenStream {
  let (g_impl, g_ty, g_where) = declarer.original.generics.split_for_impl();
  let name = &declarer.name;

  quote! {
    impl #g_impl std::ops::Deref for #name #g_ty #g_where {
      type Target = FatObj<()>;
      #[inline]
      fn deref(&self) -> &Self::Target {
        &self.fat_ಠ_ಠ
      }
    }

    impl #g_impl std::ops::DerefMut for #name #g_ty #g_where {
      #[inline]
      fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.fat_ಠ_ಠ
      }
    }
  }
}
