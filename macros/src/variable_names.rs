use syn::Ident;

pub(crate) const AVOID_CONFLICT_SUFFIX: &str = "ಠ_ಠ";

pub fn ribir_suffix_variable(from: &Ident, suffix: &str) -> Ident {
  let name_str = from.to_string();
  let prefix_size = if name_str.ends_with(AVOID_CONFLICT_SUFFIX) {
    name_str.len() - AVOID_CONFLICT_SUFFIX.len() - 1
  } else {
    name_str.len()
  };
  let prefix = &name_str[..prefix_size];
  let name = format!("{prefix}_{suffix}_{AVOID_CONFLICT_SUFFIX}");
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
}

use phf::phf_map;

use self::BuiltinMemberType::*;

pub static BUILTIN_INFOS: phf::Map<&'static str, BuiltinMember> = phf_map! {
  // BuiltinObj
  "lazy_host_id" => BuiltinMember { host_ty: "BuiltinObj", mem_ty: Method, var_name: "lazy"},
  "lazy_id" => BuiltinMember { host_ty: "BuiltinObj", mem_ty: Method, var_name: "lazy"},
  // MixBuiltin
  "auto_focus" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Field, var_name: "mix_builtin" },
  "tab_index" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Field, var_name: "mix_builtin" },
  "on_event" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Method, var_name: "mix_builtin" },
  "on_mounted" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Method, var_name: "mix_builtin" },
  "on_disposed" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Method, var_name: "mix_builtin" },
  "on_performed_layout" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Method, var_name: "mix_builtin" },
  "on_pointer_down" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Method, var_name: "mix_builtin" },
  "on_pointer_down_capture" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Method, var_name: "mix_builtin" },
  "on_pointer_up" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Method, var_name: "mix_builtin" },
  "on_pointer_up_capture" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Method, var_name: "mix_builtin" },
  "on_pointer_move" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Method, var_name: "mix_builtin" },
  "on_pointer_move_capture" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Method, var_name: "mix_builtin" },
  "on_pointer_cancel" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Method, var_name: "mix_builtin" },
  "on_pointer_enter" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Method, var_name: "mix_builtin" },
  "on_pointer_leave" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Method, var_name: "mix_builtin" },
  "on_tap" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Method, var_name: "mix_builtin" },
  "on_tap_capture" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Method, var_name: "mix_builtin" },
  "on_double_tap" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Method, var_name: "mix_builtin" },
  "on_double_tap_capture" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Method, var_name: "mix_builtin" },
  "on_triple_tap" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Method, var_name: "mix_builtin" },
  "on_triple_tap_capture" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Method, var_name: "mix_builtin" },
  "on_x_times_tap" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Method, var_name: "mix_builtin" },
  "on_x_times_tap_capture" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Method, var_name: "mix_builtin" },
  "on_ime_pre_edit" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Method, var_name: "mix_builtin" },
  "on_ime_pre_edit_capture" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Method, var_name: "mix_builtin" },
  "on_wheel" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Method, var_name: "mix_builtin" },
  "on_wheel_capture" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Method, var_name: "mix_builtin" },
  "on_chars" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Method, var_name: "mix_builtin" },
  "on_chars_capture" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Method, var_name: "mix_builtin" },
  "on_key_down" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Method, var_name: "mix_builtin" },
  "on_key_down_capture" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Method, var_name: "mix_builtin" },
  "on_key_up" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Method, var_name: "mix_builtin" },
  "on_key_up_capture" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Method, var_name: "mix_builtin" },
  "on_focus" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Method, var_name: "mix_builtin" },
  "on_blur" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Method, var_name: "mix_builtin" },
  "on_focus_in" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Method, var_name: "mix_builtin" },
  "on_focus_in_capture" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Method, var_name: "mix_builtin" },
  "on_focus_out" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Method, var_name: "mix_builtin" },
  "on_focus_out_capture" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Method, var_name: "mix_builtin" },
  "events_stream" => BuiltinMember { host_ty: "MixBuiltin", mem_ty: Method, var_name: "request_focus" },
  // RequestFocus
  "request_focus" => BuiltinMember { host_ty: "RequestFocus", mem_ty: Method, var_name: "request_focus" },
  "unfocus" => BuiltinMember { host_ty: "RequestFocus", mem_ty: Method, var_name: "request_focus" },
  // HasFocus
  "has_focus" => BuiltinMember { host_ty: "HasFocus", mem_ty: Method, var_name: "has_focus" },
  // MouseHover
  "mouse_hover" => BuiltinMember { host_ty: "MouseHover", mem_ty: Method, var_name: "mouse_hover" },
  // PointerPressed
  "pointer_pressed" => BuiltinMember { host_ty: "PointerPressed", mem_ty: Method, var_name: "pointer_pressed" },
  // FittedBox
  "box_fit" => BuiltinMember { host_ty: "FittedBox", mem_ty: Field, var_name: "fitted_box" },
  // BoxDecoration
  "background" => BuiltinMember { host_ty: "BoxDecoration", mem_ty: Field, var_name: "box_decoration" },
  "border" => BuiltinMember { host_ty: "BoxDecoration", mem_ty: Field, var_name: "box_decoration" },
  "border_radius" => BuiltinMember { host_ty: "BoxDecoration", mem_ty: Field, var_name: "box_decoration" },
  // Padding
  "padding" => BuiltinMember { host_ty: "Padding", mem_ty: Field, var_name: "padding" },
  // LayoutBox
  "layout_rect" => BuiltinMember { host_ty: "LayoutBox", mem_ty: Method, var_name: "layout_box"},
  "layout_pos" => BuiltinMember { host_ty: "LayoutBox", mem_ty: Method, var_name: "layout_box"},
  "layout_size" => BuiltinMember { host_ty: "LayoutBox", mem_ty: Method, var_name: "layout_box"},
  "layout_left" => BuiltinMember { host_ty: "LayoutBox", mem_ty: Method, var_name: "layout_box"},
  "layout_top" => BuiltinMember { host_ty: "LayoutBox", mem_ty: Method, var_name: "layout_box"},
  "layout_width" => BuiltinMember { host_ty: "LayoutBox", mem_ty: Method, var_name: "layout_box"},
  "layout_height" => BuiltinMember { host_ty: "LayoutBox", mem_ty: Method, var_name: "layout_box"},
  // GlobalAnchor
  "global_anchor" => BuiltinMember { host_ty: "GlobalAnchor", mem_ty: Field, var_name: "global_anchor" },
  // Cursor
  "cursor" => BuiltinMember { host_ty: "Cursor", mem_ty: Field, var_name: "cursor" },
  // Margin
  "margin" => BuiltinMember { host_ty: "Margin", mem_ty: Field, var_name: "margin" },
  // ScrollableWidget
  "scrollable" => BuiltinMember { host_ty: "ScrollableWidget", mem_ty: Field, var_name: "scrollable"},
  "scroll_pos" => BuiltinMember { host_ty: "ScrollableWidget", mem_ty: Field, var_name: "scrollable"},
  "scroll_view_size" => BuiltinMember { host_ty: "ScrollableWidget", mem_ty: Method, var_name: "scrollable"},
  "scroll_content_size" => BuiltinMember { host_ty: "ScrollableWidget", mem_ty: Method, var_name: "scrollable"},
  "jump_to" => BuiltinMember { host_ty: "ScrollableWidget", mem_ty: Method, var_name: "scrollable"},
  // TransformWidget
  "transform" => BuiltinMember { host_ty: "TransformWidget", mem_ty: Field, var_name: "transform" },
  // HAlignWidget
  "h_align" => BuiltinMember { host_ty: "HAlignWidget", mem_ty: Field, var_name: "h_align" },
  // VAlignWidget
  "v_align" => BuiltinMember { host_ty: "VAlignWidget", mem_ty: Field, var_name: "v_align" },
  // RelativeAnchor
  "anchor" => BuiltinMember { host_ty: "RelativeAnchor", mem_ty: Field, var_name: "relative_anchor" },
  // Visibility
  "visible" => BuiltinMember { host_ty: "Visibility", mem_ty: Field, var_name: "visibility" },
  // Opacity
  "opacity" => BuiltinMember { host_ty: "Opacity", mem_ty: Field, var_name: "opacity" },
  // KeepAlive
  "keep_alive" => BuiltinMember { host_ty: "KeepAlive", mem_ty: Field, var_name: "keep_alive" },
};
