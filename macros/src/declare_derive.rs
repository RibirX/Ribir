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
    let declarer = Declarer::new(host, &mut stt.fields)?;
    let Declarer { name, fields, .. } = &declarer;
    // reverse name check.
    fields
      .iter()
      .try_for_each(DeclareField::check_reserve)?;
    let set_methods = declarer_set_methods(fields, vis);

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
        fat_obj: FatObj<()>,
      }

      impl #g_impl Declare for #host #g_ty #g_where {
        type Builder = #name #g_ty;

        fn declarer() -> Self::Builder {
          #name {
            #(#builder_f_names : None ,)*
            fat_obj: FatObj::new(()),
          }
        }
      }

      impl #g_impl ObjDeclarer for #name #g_ty #g_where {
        #[allow(clippy::type_complexity)]
        type Target = FatObj<State<#host #g_ty>>;

        #[inline]
        fn finish(mut self) -> Self::Target {
          #(#field_values)*
          let mut _this_ಠ_ಠ = State::value(#host {
            #(#field_names : #field_names.0),*
          });
          let mut _fat_ಠ_ಠ = self.fat_obj;
          #(
            if let Some(o) = #field_names2.1 {
              let mut _this_ಠ_ಠ = _this_ಠ_ಠ.clone_writer();
              let u = o.subscribe(move |(_, v)| _this_ಠ_ಠ.write().#field_names2 = v);
              _fat_ಠ_ಠ = _fat_ಠ_ಠ.on_disposed(move |_| u.unsubscribe());
            }
          );*

          _fat_ಠ_ಠ.map(move |_| _this_ಠ_ಠ)
        }
      }


      impl #g_impl #name #g_ty #g_where {
        #(#set_methods)*
      }

      impl #g_impl #name #g_ty #g_where {
        #[doc="Specify the `Class` that should be applied to the widget."]
        #vis fn class<const _M: u8>(mut self, v: impl DeclareInto<ClassName, _M>) -> Self {
          self.fat_obj = self.fat_obj.class(v);
          self
        }

        #[doc="Initializes the widget with a tab index. The tab index is used to \
          allow or prevent widgets from being sequentially focusable(usually with \
          the Tab key, hence the name) and determine their relative ordering for \
          sequential focus navigation"]
        #vis fn tab_index<const _M: u8>(mut self, v: impl DeclareInto<i16, _M>) -> Self {
          self.fat_obj = self.fat_obj.tab_index(v);
          self
        }

        #[doc="Initializes whether the `widget` should automatically get focus \
          when the window loads."]
        #vis fn auto_focus<const _M: u8>(mut self, v: impl DeclareInto<bool, _M>) -> Self {
          self.fat_obj = self.fat_obj.auto_focus(v);
          self
        }

        #[doc="Attaches an event handler to the widget. It's triggered when any \
          event or lifecycle change happens."]
        #vis fn on_event(mut self, f: impl FnMut(&mut Event) + 'static) -> Self {
          self.fat_obj = self.fat_obj.on_event(f);
          self
        }

        #[doc="Attaches an event handler that runs when the widget is first \
          mounted to the tree"]
        #vis fn on_mounted(mut self, f: impl FnOnce(&mut LifecycleEvent) + 'static) -> Self {
          self.fat_obj = self.fat_obj.on_mounted(f);
          self
        }

        #[doc="Attaches an event handler that runs after the widget is performed layout."]
        #vis fn on_performed_layout(
          mut self,
          f: impl FnMut(&mut LifecycleEvent) + 'static
        ) -> Self {
          self.fat_obj = self.fat_obj.on_performed_layout(f);
          self
        }

        #[doc="Attaches an event handler that runs when the widget is disposed."]
        #vis fn on_disposed(mut self, f: impl FnOnce(&mut LifecycleEvent) + 'static) -> Self {
          self.fat_obj = self.fat_obj.on_disposed(f);
          self
        }

        #[doc="Attaches a handler to the widget that is triggered when a pointer \
          down occurs."]
        #vis fn on_pointer_down(mut self, f: impl FnMut(&mut PointerEvent) + 'static) -> Self {
          self.fat_obj = self.fat_obj.on_pointer_down(f);
          self
        }

        #[doc="Attaches a handler to the widget that is triggered during the capture \
          phase of a pointer down event. This is similar to `on_pointer_down`, but \
          it's triggered earlier in the event flow."]
        #vis fn on_pointer_down_capture(
          mut self,
          f: impl FnMut(&mut PointerEvent) + 'static
        ) -> Self {
          self.fat_obj = self.fat_obj.on_pointer_down_capture(f);
          self
        }

        #[doc="Attaches a handler to the widget that is triggered when a pointer \
          up occurs."]
        #vis fn on_pointer_up(mut self, f: impl FnMut(&mut PointerEvent) + 'static) -> Self {
          self.fat_obj = self.fat_obj.on_pointer_up(f);
          self
        }

        #[doc="Attaches a handler to the widget that is triggered during the capture \
          phase of a pointer up event. This is similar to `on_pointer_up`, but it's \
          triggered earlier in the event flow."]
        #vis fn on_pointer_up_capture(
          mut self,
          f: impl FnMut(&mut PointerEvent) + 'static
        ) -> Self {
          self.fat_obj = self.fat_obj.on_pointer_up_capture(f);
          self
        }

        #[doc="Attaches a handler to the widget that is triggered when a pointer \
          move occurs."]
        #vis fn on_pointer_move(
          mut self,
          f: impl FnMut(&mut PointerEvent) + 'static
        ) -> Self {
          self.fat_obj = self.fat_obj.on_pointer_move(f);
          self
        }

        #[doc="Attaches a handler to the widget that is triggered during the capture \
          phase of a pointer move event. This is similar to `on_pointer_move`, but \
          it's triggered earlier in the event flow."]
        #vis fn on_pointer_move_capture(
          mut self,
          f: impl FnMut(&mut PointerEvent) + 'static
        ) -> Self {
          self.fat_obj = self.fat_obj.on_pointer_move_capture(f);
          self
        }

        #[doc="Attaches a handler to the widget that is triggered when a pointer \
          event cancels."]
        #vis fn on_pointer_cancel(mut self, f: impl FnMut(&mut PointerEvent) + 'static) -> Self {
          self.fat_obj = self.fat_obj.on_pointer_cancel(f);
          self
        }

        #[doc="Attaches a handler to the widget that is triggered when a pointer device \
          is moved into the hit test boundaries of an widget or one of its descendants"]
        #vis fn on_pointer_enter(mut self, f: impl FnMut(&mut PointerEvent) + 'static) -> Self {
          self.fat_obj = self.fat_obj.on_pointer_enter(f);
          self
        }

        #[doc="Attaches a handler to the widget that is triggered when a pointer device
          is moved out of the hit test boundaries of an widget or one of its descendants."]
        #vis fn on_pointer_leave(mut self, f: impl FnMut(&mut PointerEvent) + 'static) -> Self {
          self.fat_obj = self.fat_obj.on_pointer_leave(f);
          self
        }

        #[doc="Attaches a handler to the widget that is triggered when a tap(click) occurs."]
        #vis fn on_tap(mut self, f: impl FnMut(&mut PointerEvent) + 'static) -> Self {
          self.fat_obj = self.fat_obj.on_tap(f);
          self
        }

        #[doc="Attaches a handler to the widget that is triggered during the capture
          phase of a tap event. This is similar to `on_tap`, but it's triggered
          earlier in the event flow."]
        #vis fn on_tap_capture(mut self, f: impl FnMut(&mut PointerEvent) + 'static) -> Self {
          self.fat_obj = self.fat_obj.on_tap_capture(f);
          self
        }

        #[doc="Attaches a handler to the widget that is triggered when a double tap occurs."]
        #vis fn on_double_tap(
          mut self,
          f: impl FnMut(&mut PointerEvent) + 'static
        ) -> Self {
          self.fat_obj = self.fat_obj.on_double_tap(f);
          self
        }

        #[doc="Attaches a handler to the widget that is triggered during the capture \
          phase of a double tap event. This is similar to `on_double_tap`, but it's \
          triggered earlier in the event flow."]
        #vis fn on_double_tap_capture(
          mut self,
          f: impl FnMut(&mut PointerEvent) + 'static
        ) -> Self {
          self.fat_obj = self.fat_obj.on_double_tap_capture(f);
          self
        }

        #[doc="Attaches a handler to the widget that is triggered when a triple tap occurs."]
        #vis fn on_triple_tap(
          mut self,
          f: impl FnMut(&mut PointerEvent) + 'static
        ) -> Self {
          self.fat_obj = self.fat_obj.on_triple_tap(f);
          self
        }

        #[doc="Attaches a handler to the widget that is triggered during the capture \
          phase of a triple tap event. This is similar to `on_triple_tap`, but it's \
          triggered earlier in the event flow."]
        #vis fn on_triple_tap_capture(
          mut self,
          f: impl FnMut(&mut PointerEvent) + 'static
        ) -> Self {
          self.fat_obj = self.fat_obj.on_triple_tap_capture(f);
          self
        }

        #[doc="Attaches a handler to the widget that is triggered when a x-times tap
          occurs."]
        #vis fn on_x_times_tap(
          mut self,
          f: (usize, impl FnMut(&mut PointerEvent) + 'static)
        ) -> Self {
          self.fat_obj = self.fat_obj.on_x_times_tap(f);
          self
        }

        #[doc="Attaches a handler to the widget that is triggered during the capture \
          phase of a x-times tap event. This is similar to `on_x_times_tap`, but it's \
          triggered earlier in the event flow."]
        #vis fn on_x_times_tap_capture(
          mut self,
          f: (usize, impl FnMut(&mut PointerEvent) + 'static),
        ) -> Self {
          self.fat_obj = self.fat_obj.on_x_times_tap_capture(f);
          self
        }

        #[doc="Attaches a handler to the widget that is triggered when the user rotates a
          wheel button on a pointing device (typically a mouse)."]
        #vis fn on_wheel(mut self, f: impl FnMut(&mut WheelEvent) + 'static) -> Self {
          self.fat_obj = self.fat_obj.on_wheel(f);
          self
        }

        #[doc="Attaches a handler to the widget that is triggered during the capture \
          phase of a wheel event. This is similar to `on_wheel`, but it's triggered \
          earlier in the event flow."]
        #vis fn on_wheel_capture(mut self, f: impl FnMut(&mut WheelEvent) + 'static) -> Self {
          self.fat_obj = self.fat_obj.on_wheel_capture(f);
          self
        }

        #[doc="Attaches a handler to the widget that is triggered when the input method
          pre-edit area is changed."]
        #vis fn on_ime_pre_edit(mut self, f: impl FnMut(&mut ImePreEditEvent) + 'static) -> Self {
          self.fat_obj = self.fat_obj.on_ime_pre_edit(f);
          self
        }

        #[doc="Attaches a handler to the widget that is triggered during the capture \
          phase of a input method pre-edit event. This is similar to `on_ime_pre_edit`, \
          but it's triggered earlier in the event flow."]
        #vis fn on_ime_pre_edit_capture(
          mut self,
          f: impl FnMut(&mut ImePreEditEvent) + 'static
        ) -> Self {
          self.fat_obj = self.fat_obj.on_ime_pre_edit_capture(f);
          self
        }

        #[doc="Attaches a handler to the widget that is triggered when the input method
          commits text or keyboard pressed the text key"]
        #vis fn on_chars(mut self, f: impl FnMut(&mut CharsEvent) + 'static) -> Self {
          self.fat_obj = self.fat_obj.on_chars(f);
          self
        }

        #[doc="Attaches a handler to the widget that is triggered during the capture \
          phase of a input method commit event. This is similar to `on_chars`, but it's \
          triggered earlier in the event flow."]
        #vis fn on_chars_capture(mut self, f: impl FnMut(&mut CharsEvent) + 'static) -> Self {
          self.fat_obj = self.fat_obj.on_chars_capture(f);
          self
        }

        #[doc="Attaches a handler to the widget that is triggered when the keyboard key
          is pressed."]
        #vis fn on_key_down(mut self, f: impl FnMut(&mut KeyboardEvent) + 'static) -> Self {
          self.fat_obj = self.fat_obj.on_key_down(f);
          self
        }

        #[doc="Attaches a handler to the widget that is triggered during the capture \
          phase of a key down event. This is similar to `on_key_down`, but it's \
          triggered earlier in the event flow."]
        #vis fn on_key_down_capture(mut self, f: impl FnMut(&mut KeyboardEvent) + 'static) -> Self {
          self.fat_obj = self.fat_obj.on_key_down_capture(f);
          self
        }

        #[doc="Attaches a handler to the widget that is triggered when the keyboard key
          is released."]
        #vis fn on_key_up(mut self, f: impl FnMut(&mut KeyboardEvent) + 'static) -> Self {
          self.fat_obj = self.fat_obj.on_key_up(f);
          self
        }

        #[doc="Attaches a handler to the widget that is triggered during the capture \
          phase of a key up event. This is similar to `on_key_up`, but it's \
          triggered earlier in the event flow."]
        #vis fn on_key_up_capture(mut self, f: impl FnMut(&mut KeyboardEvent) + 'static) -> Self {
          self.fat_obj = self.fat_obj.on_key_up_capture(f);
          self
        }

        #[doc="Attaches a handler to the widget that is triggered when the widget is focused."]
        #vis fn on_focus(mut self, f: impl FnMut(&mut FocusEvent) + 'static) -> Self {
          self.fat_obj = self.fat_obj.on_focus(f);
          self
        }

        #[doc="Attaches a handler to the widget that is triggered when the widget \
          is lost focus."]
        #vis fn on_blur(mut self, f: impl FnMut(&mut FocusEvent) + 'static) -> Self {
          self.fat_obj = self.fat_obj.on_blur(f);
          self
        }

        #[doc="Attaches a handler to the widget that is triggered when the widget or its \
          descendants are focused. The main difference between this event and focus \
          is that focusin bubbles while focus does not."]
        #vis fn on_focus_in(mut self, f: impl FnMut(&mut FocusEvent) + 'static) -> Self {
          self.fat_obj = self.fat_obj.on_focus_in(f);
          self
        }

        #[doc="Attaches a handler to the widget that is triggered during the capture \
          phase of a focus in event. This is similar to `on_focus_in`, but it's \
          triggered earlier in the event flow."]
        #vis fn on_focus_in_capture(mut self, f: impl FnMut(&mut FocusEvent) + 'static) -> Self {
          self.fat_obj = self.fat_obj.on_focus_in_capture(f);
          self
        }

        #[doc="Attaches a handler to the widget that is triggered when the widget\
          or its descendants are lost focus. The main difference between this event \
          and focusout is that focusout bubbles while blur does not"]
        #vis fn on_focus_out(mut self, f: impl FnMut(&mut FocusEvent) + 'static) -> Self {
          self.fat_obj = self.fat_obj.on_focus_out(f);
          self
        }

        #[doc="Attaches a handler to the widget that is triggered during the capture \
          phase of a focus out event. This is similar to `on_focus_out`, but it's \
          triggered earlier in the event flow."]
        #vis fn on_focus_out_capture(mut self, f: impl FnMut(&mut FocusEvent) + 'static) -> Self {
          self.fat_obj = self.fat_obj.on_focus_out_capture(f);
          self
        }

        #[doc="Initializes how its child should be scale to fit its box."]
        #vis fn box_fit<const _M: u8>(mut self, v: impl DeclareInto<BoxFit, _M>) -> Self {
          self.fat_obj = self.fat_obj.box_fit(v);
          self
        }

        #[doc="Initializes the background of the widget."]
        #vis fn background<const _M: u8>(mut self, v: impl DeclareInto<Option<Brush>, _M>) -> Self {
          self.fat_obj = self.fat_obj.background(v);
          self
        }

        #[doc="Initializes the border of the widget."]
        #vis fn border<const _M: u8>(mut self, v: impl DeclareInto<Option<Border>, _M>) -> Self {
          self.fat_obj = self.fat_obj.border(v);
          self
        }

        #[doc="Initializes the border radius of the widget."]
        #vis fn border_radius<const _M: u8>(
          mut self, v: impl DeclareInto<Option<Radius>, _M>
        ) -> Self {
          self.fat_obj = self.fat_obj.border_radius(v);
          self
        }

        #[doc="Initializes the foreground for this widget."]
        #vis fn foreground<const _M: u8>(mut self, v: impl DeclareInto<Brush, _M>) -> Self {
          self.fat_obj = self.fat_obj.foreground(v);
          self
        }

        #[doc="Initializes the painting style for shapes and paths."]
        #vis fn painting_style<const _M: u8>(mut self, v: impl DeclareInto<PaintingStyle, _M>)
          -> Self
        {
          self.fat_obj = self.fat_obj.painting_style(v);
          self
        }


        #[doc="Initializes the text style for this widget."]
        #vis fn text_style<const _M: u8>(mut self, v: impl DeclareInto<TextStyle, _M>) -> Self {
          self.fat_obj = self.fat_obj.text_style(v);
          self
        }


        #[doc="Initializes the extra space within the widget."]
        #vis fn padding<const _M: u8>(mut self, v: impl DeclareInto<EdgeInsets, _M>) -> Self {
          self.fat_obj = self.fat_obj.padding(v);
          self
        }

        #[doc="Initializes the cursor of the widget."]
        #vis fn cursor<const _M: u8>(mut self, v: impl DeclareInto<CursorIcon, _M>) -> Self {
          self.fat_obj = self.fat_obj.cursor(v);
          self
        }

        #[doc="Initializes the space around the widget."]
        #vis fn margin<const _M: u8>(mut self, v: impl DeclareInto<EdgeInsets, _M>) -> Self {
          self.fat_obj = self.fat_obj.margin(v);
          self
        }

        #[doc="Initializes how user can scroll the widget."]
        #vis fn scrollable<const _M: u8>(mut self, v: impl DeclareInto<Scrollable, _M>) -> Self {
          self.fat_obj = self.fat_obj.scrollable(v);
          self
        }

        #[doc="Initializes the constraints clamp of the widget."]
        #vis fn clamp<const _M: u8>(mut self, v: impl DeclareInto<BoxClamp, _M>) -> Self {
          self.fat_obj = self.fat_obj.clamp(v);
          self
        }

        #[doc="Initializes the transformation of the widget."]
        #vis fn transform<const _M: u8>(mut self, v: impl DeclareInto<Transform, _M>) -> Self {
          self.fat_obj = self.fat_obj.transform(v);
          self
        }

        #[doc="Initializes how the widget should be aligned horizontally."]
        #vis fn h_align<const _M: u8>(mut self, v: impl DeclareInto<HAlign, _M>) -> Self {
          self.fat_obj = self.fat_obj.h_align(v);
          self
        }

        #[doc="Initializes how the widget should be aligned vertically."]
        #vis fn v_align<const _M: u8>(mut self, v: impl DeclareInto<VAlign, _M>) -> Self {
          self.fat_obj = self.fat_obj.v_align(v);
          self
        }

        #[doc="Anchor child constraints relative to the parent widget. It's \
        important to note that if you anchor the child widget outside of its \
        parent, it may become unable to click, so ensure there is ample space \
        within the parent."]
        #vis fn anchor<const _M: u8>(mut self, v: impl DeclareInto<Anchor, _M>) -> Self {
          self.fat_obj = self.fat_obj.anchor(v);
          self
        }

        #[doc="Initializes the global anchor of the widget."]
        #vis fn global_anchor<const _M: u8>(mut self, v: impl DeclareInto<Anchor, _M>) -> Self {
          self.fat_obj = self.fat_obj.global_anchor(v);
          self
        }

        #[doc="Initializes the visibility of the widget."]
        #vis fn visible<const _M: u8>(mut self, v: impl DeclareInto<bool, _M>) -> Self
        {
          self.fat_obj = self.fat_obj.visible(v);
          self
        }

        #[doc="Initializes the opacity of the widget."]
        #vis fn opacity<const _M: u8>(mut self, v: impl DeclareInto<f32, _M>) -> Self
        {
          self.fat_obj = self.fat_obj.opacity(v);
          self
        }

        #[doc="Initializes the `keep_alive` value of the `KeepAlive` widget."]
        #vis fn keep_alive<const _M: u8>(mut self, v: impl DeclareInto<bool, _M>) -> Self
        {
          self.fat_obj = self.fat_obj.keep_alive(v);
          self
        }

        #[doc="Initializes the `track_id` value of the `TrackWidgetId` widget."]
        #vis fn track_id<const _M: u8>(mut self) -> Self
        {
          self.fat_obj = self.fat_obj.track_id();
          self
        }
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

fn declarer_set_methods<'a>(
  fields: &'a [DeclareField], vis: &'a Visibility,
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
          #vis fn #set_method(mut self, v: #ty) -> Self {
            self.#field_name = Some(DeclareInit::Value(v));
            self
          }
        }
      } else {
        quote! {
          #[inline]
          #[allow(clippy::type_complexity)]
          #doc
          #vis fn #set_method<const _M: u8>(mut self, v: impl DeclareInto<#ty, _M>) -> Self {
            self.#field_name = Some(v.declare_into());
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
          self.#f_name.take().map_or_else(
            || (#df, None),
            |v| v.unzip()
          )
        }
      } else {
        let err = format!("Required field `{stt_name}::{f_name}` not set");
        quote! { self.#f_name.take().expect(#err).unzip() }
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
  }
}
