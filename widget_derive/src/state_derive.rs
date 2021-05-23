use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Ident, Index};

use crate::widget_derive::ProxyDeriveInfo;

const STATE_ATTR_NAME: &'static str = "state";

pub(crate) fn stateful_derive(input: &syn::DeriveInput) -> TokenStream2 {
  let info = ProxyDeriveInfo::new(input, "Stateful", STATE_ATTR_NAME)
    .and_then(|stt| stt.none_proxy_specified_error());

  match info {
    Ok(info) => {
      let name = &input.ident;
      let state_name: Ident = syn::parse_str(&format!("{}State", input.ident)).unwrap();

      let state_generis = info.attr_fields.attr_fields_generics();
      let (_, ty_generics, where_clause) = state_generis.split_for_impl();
      let (w_impl_generics, w_ty_generics, w_where_clause) = input.generics.split_for_impl();
      let state_fields = info.attr_fields.attr_fields().into_iter().map(|(f, _)| f);

      let vis = input.vis.clone();

      let state_field_names = state_fields.clone().enumerate().map(|(idx, f)| {
         f
          .ident
          .as_ref()
          .map_or_else(||{
            let index = Index::from(idx);
            quote!{#index}
          }, |ident|quote!{#ident})
      });

      let state_fn_names = state_field_names
        .clone()
        .map(|name| prefix_ident("state_", &name));

      let state_ty = state_fields.clone().map(|f| &f.ty);

      let stateful_name = prefix_ident("Stateful", &quote!{#name} );

      let state_def = if info.attr_fields.is_tuple {
        quote! {
          #[derive(StatePartialEq)]
          #vis struct #state_name #ty_generics #where_clause (
            #(#state_fields ,)*
          );
        }
      } else {
        quote! {
          #[derive(StatePartialEq)]
          #vis struct #state_name #ty_generics #where_clause {
            #(#state_fields, )*
          }
        }
      };

      quote! {
        // Define custom state.
        #state_def

        // A stateful version widget
        #[derive(Widget)]
        #vis struct #stateful_name #w_ty_generics(StatefulImpl<#name #w_ty_generics>) #w_where_clause;

        // Every state have a state observable. 
        impl #w_impl_generics #stateful_name #w_ty_generics #w_where_clause {
          #(
            pub fn #state_fn_names(&mut self)
              -> impl LocalObservable<'static, Item = StateChange<#state_ty>, Err = ()> {
              self.0.state_change(|w| w.#state_field_names.clone())
            }
          )*
        }

        impl #w_impl_generics Stateful for #stateful_name #w_ty_generics #w_where_clause {
          type RawWidget = #name #w_ty_generics;
          fn ref_cell(&self) -> StateRefCell<Self::RawWidget> {
            self.0.ref_cell()
          }
        }

        impl #w_impl_generics CloneStates for #name #w_ty_generics #w_where_clause {
          type States =  #state_name #ty_generics;
          fn clone_states(&self) -> Self::States {
            unimplemented!()
          }
        }

        impl #w_impl_generics IntoStateful for #name #w_ty_generics #w_where_clause {
          type S = #stateful_name #w_ty_generics;

          #[inline]
          fn into_stateful(self) -> Self::S {
            #stateful_name(StatefulImpl::new(self))
          }
        }

        impl #w_impl_generics std::ops::Deref for #stateful_name #w_ty_generics #w_where_clause {
          type Target = StatefulImpl<#name #w_ty_generics>;
          #[inline]
          fn deref(&self) -> &Self::Target { &self.0}
        }

        impl #w_impl_generics std::ops::DerefMut for #stateful_name #w_ty_generics #w_where_clause {
          #[inline]
          fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
        }
      }
    }
    Err(err) => err,
  }
}

fn prefix_ident(prefix: &str, ident: &TokenStream2) -> Ident {
  syn::parse_str::<Ident>(&format!("{}{}", prefix, ident)).unwrap()
}
