use proc_macro2::Span;
use syn::Ident;

pub(crate) const AVOID_CONFLICT_SUFFIX: &str = "ಠ_ಠ";

pub fn ribir_suffix_variable(from: &Ident, suffix: &str) -> Ident {
  let name = format!("{}_{suffix}_{AVOID_CONFLICT_SUFFIX}", from);
  Ident::new(&name, from.span())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinMemberType {
  Field,
  Method,
}

pub struct BuiltinMember {
  pub host_ty: &'static str,
  pub mem_ty: BuiltinMemberType,
  pub var_name: &'static str,
  pub run_before_clone: Option<&'static str>,
}

impl BuiltinMember {
  pub fn get_builtin_widget_method(&self, span: Span) -> Ident {
    Ident::new(&format!("get_{}_widget", self.var_name), span)
  }

  pub fn run_before_clone_method(&self, span: Span) -> Option<Ident> {
    self
      .run_before_clone
      .as_ref()
      .map(|method| Ident::new(method, span))
  }
}

use phf::phf_map;

use self::BuiltinMemberType::*;

macro_rules! builtin_member {
  ($host_ty:literal, $mem_ty:ident, $var_name:literal, $run_before_clone:literal) => {
    BuiltinMember {
      host_ty: $host_ty,
      mem_ty: $mem_ty,
      var_name: $var_name,
      run_before_clone: Some($run_before_clone),
    }
  };
  ($host_ty:literal, $mem_ty:ident, $var_name:literal) => {
    BuiltinMember {
      host_ty: $host_ty,
      mem_ty: $mem_ty,
      var_name: $var_name,
      run_before_clone: None,
    }
  };
}
pub static BUILTIN_INFOS: phf::Map<&'static str, BuiltinMember> = phf_map! {
  // Class
  "class" => builtin_member!{"Class", Field, "class"},
  // MixFlags
  "has_focus" => builtin_member!{"MixFlags", Method, "mix_flags", "trace_focus" },
  "is_hover" => builtin_member!{"MixFlags", Method, "mix_flags", "trace_hover" },
  "is_pointer_pressed" => builtin_member!{"MixFlags", Method, "mix_flags", "trace_pointer_pressed" },
  "is_auto_focus" => builtin_member!{"MixFlags", Method, "mix_flags"},
  "set_auto_focus" => builtin_member!{"MixFlags", Method, "mix_flags"},
  "tab_index" => builtin_member!{"MixFlags", Method, "mix_flags"},
  "set_tab_index" => builtin_member!{"MixFlags", Method, "mix_flags"},
  // MixBuiltin
  "on_event" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "on_mounted" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "on_disposed" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "on_performed_layout" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "on_pointer_down" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "on_pointer_down_capture" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "on_pointer_up" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "on_pointer_up_capture" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "on_pointer_move" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "on_pointer_move_capture" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "on_pointer_cancel" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "on_pointer_enter" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "on_pointer_leave" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "on_tap" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "on_tap_capture" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "on_double_tap" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "on_double_tap_capture" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "on_triple_tap" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "on_triple_tap_capture" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "on_x_times_tap" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "on_x_times_tap_capture" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "on_ime_pre_edit" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "on_ime_pre_edit_capture" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "on_wheel" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "on_wheel_capture" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "on_chars" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "on_chars_capture" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "on_key_down" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "on_key_down_capture" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "on_key_up" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "on_key_up_capture" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "on_focus" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "on_blur" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "on_focus_in" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "on_focus_in_capture" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "on_focus_out" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "on_focus_out_capture" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "on_custom_concrete_event" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "on_custom_event" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},
  "events_stream" => builtin_member!{"MixBuiltin", Method, "mix_builtin"},

  // RequestFocus
  "request_focus" => builtin_member!{"RequestFocus", Method, "request_focus"},
  "unfocus" => builtin_member!{"RequestFocus", Method, "request_focus"},
  // FittedBox
  "box_fit" => builtin_member!{"FittedBox", Field, "fitted_box"},
  // Background
  "background" => builtin_member!{"Background", Field, "background"},
  // BorderWidget
  "border" => builtin_member!{"BorderWidget", Field, "border"},
  // RadiusWidget
  "radius" => builtin_member!{"RadiusWidget", Field, "radius"},
  // Foreground
  "foreground" => builtin_member! { "Foreground", Field, "foreground"},
  // PaintingStyleWidget
  "painting_style" => builtin_member! { "PaintingStyleWidget", Field, "painting_style" },
  // TextStyle
  "text_style" => builtin_member! { "TextStyleWidget", Field, "text_style" },
  "font_size" => builtin_member! { "TextStyleWidget", Method, "text_style" },
  "font_face" => builtin_member! { "TextStyleWidget", Method, "text_style" },
  "letter_space" => builtin_member! { "TextStyleWidget", Method, "text_style" },
  "text_line_height" => builtin_member! { "TextStyleWidget", Method, "text_style" },
  "text_overflow" => builtin_member! { "TextStyleWidget", Method, "text_style" },
  // Padding
  "padding" => builtin_member!{"Padding", Field, "padding"},
  // LayoutBox
  "layout_rect" => builtin_member!{"LayoutBox", Method, "layout_box"},
  "layout_pos" => builtin_member!{"LayoutBox", Method, "layout_box"},
  "layout_size" => builtin_member!{"LayoutBox", Method, "layout_box"},
  "layout_left" => builtin_member!{"LayoutBox", Method, "layout_box"},
  "layout_top" => builtin_member!{"LayoutBox", Method, "layout_box"},
  "layout_width" => builtin_member!{"LayoutBox", Method, "layout_box"},
  "layout_height" => builtin_member!{"LayoutBox", Method, "layout_box"},
  // GlobalAnchor
  "global_anchor_x" => builtin_member!{"GlobalAnchor", Field, "global_anchor_x"},
  "global_anchor_y" => builtin_member!{"GlobalAnchor", Field, "global_anchor_y"},
  // Cursor
  "cursor" => builtin_member!{"Cursor", Field, "cursor"},
  // Margin
  "margin" => builtin_member!{"Margin", Field, "margin"},
  // ScrollableWidget
  "scrollable" => builtin_member!{"ScrollableWidget", Field, "scrollable"},
  "get_scroll_pos" => builtin_member!{"ScrollableWidget", Method, "scrollable"},
  "scroll_view_size" => builtin_member!{"ScrollableWidget", Method, "scrollable"},
  "scroll_content_size" => builtin_member!{"ScrollableWidget", Method, "scrollable"},
  "jump_to" => builtin_member!{"ScrollableWidget", Method, "scrollable"},
  // ConstrainedBox
  "clamp" => builtin_member!{"ConstrainedBox", Field, "constrained_box"},
  // TransformWidget
  "transform" => builtin_member!{"TransformWidget", Field, "transform"},
  // HAlignWidget
  "h_align" => builtin_member!{"HAlignWidget", Field, "h_align"},
  // VAlignWidget
  "v_align" => builtin_member!{"VAlignWidget", Field, "v_align"},
  // RelativeAnchor
  "anchor" => builtin_member!{"RelativeAnchor", Field, "relative_anchor"},
  // Visibility
  "visible" => builtin_member!{"Visibility", Field, "visibility"},
  // Opacity
  "opacity" => builtin_member!{"Opacity", Field, "opacity"},
  // KeepAlive
  "keep_alive" => builtin_member!{"KeepAlive", Field, "keep_alive"},
  // Tooltips
  "tooltips" => builtin_member!{"Tooltips", Field, "tooltips"},
  // TrackWidgetId
  "track_id" => builtin_member!{"TrackWidgetId", Method, "track_id"},
  // ClipBoundary
  "clip_boundary" => builtin_member!{"ClipBoundary", Field, "clip_boundary"},
  // Providers
  "providers" => builtin_member!{"Providers", Field, "providers"},
  // Disabled
  "disabled" => builtin_member!{"Disabled", Field, "disabled"},
};
