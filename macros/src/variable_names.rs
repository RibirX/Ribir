use std::sync::LazyLock;

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

pub static BUILTIN_INFOS: LazyLock<ahash::HashMap<&'static str, BuiltinMember>> =
  LazyLock::new(|| {
    let mut m = ahash::HashMap::default();
    // Class
    m.insert("class", builtin_member! {"Class", Field, "class"});
    m.insert("reuse_id", builtin_member! {"Reuse", Field, "reuse_id"});
    // MixFlags
    m.insert("is_focused", builtin_member! {"MixFlags", Method, "mix_flags", "trace_focus" });
    m.insert(
      "focus_changed_reason",
      builtin_member! {"MixFlags", Method, "mix_flags", "trace_focus" },
    );
    m.insert("is_hovered", builtin_member! {"MixFlags", Method, "mix_flags", "trace_hover" });
    m.insert(
      "is_pointer_pressed",
      builtin_member! {"MixFlags", Method, "mix_flags", "trace_pointer_pressed" },
    );
    m.insert("auto_focus", builtin_member! {"MixFlags", Method, "mix_flags"});
    m.insert("set_auto_focus", builtin_member! {"MixFlags", Method, "mix_flags"});
    m.insert("tab_index", builtin_member! {"MixFlags", Method, "mix_flags"});
    m.insert("set_tab_index", builtin_member! {"MixFlags", Method, "mix_flags"});
    // MixBuiltin
    m.insert("on_event", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("on_mounted", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("on_disposed", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("on_performed_layout", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("on_pointer_down", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("on_pointer_down_capture", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("on_pointer_up", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("on_pointer_up_capture", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("on_pointer_move", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("on_pointer_move_capture", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("on_pointer_cancel", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("on_pointer_enter", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("on_pointer_leave", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("on_tap", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("on_tap_capture", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("on_double_tap", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("on_double_tap_capture", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("on_triple_tap", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("on_triple_tap_capture", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("on_x_times_tap", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("on_x_times_tap_capture", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("on_ime_pre_edit", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("on_ime_pre_edit_capture", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("on_wheel", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("on_wheel_capture", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("on_chars", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("on_chars_capture", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("on_key_down", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("on_key_down_capture", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("on_key_up", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("on_key_up_capture", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("on_focus", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("on_blur", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("on_focus_in", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("on_focus_in_capture", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("on_focus_out", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("on_focus_out_capture", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("on_custom_concrete_event", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("on_custom_event", builtin_member! {"MixBuiltin", Method, "mix_builtin"});
    m.insert("events_stream", builtin_member! {"MixBuiltin", Method, "mix_builtin"});

    // RequestFocus
    m.insert("request_focus", builtin_member! {"RequestFocus", Method, "request_focus"});
    m.insert("unfocus", builtin_member! {"RequestFocus", Method, "request_focus"});
    // FittedBox
    m.insert("box_fit", builtin_member! {"FittedBox", Field, "fitted_box"});
    // Background
    m.insert("background", builtin_member! {"Background", Field, "background"});
    // BorderWidget
    m.insert("border", builtin_member! {"BorderWidget", Field, "border"});
    // RadiusWidget
    m.insert("radius", builtin_member! {"RadiusWidget", Field, "radius"});
    // Foreground
    m.insert("foreground", builtin_member! { "Foreground", Field, "foreground"});
    // PaintingStyleWidget
    m.insert("painting_style", builtin_member! { "PaintingStyleWidget", Field, "painting_style" });
    // TextAlign
    m.insert("text_align", builtin_member! { "TextAlignWidget", Field, "text_align" });
    // TextStyle
    m.insert("text_style", builtin_member! { "TextStyleWidget", Field, "text_style" });
    m.insert("font_size", builtin_member! { "TextStyleWidget", Method, "text_style" });
    m.insert("font_face", builtin_member! { "TextStyleWidget", Method, "text_style" });
    m.insert("letter_space", builtin_member! { "TextStyleWidget", Method, "text_style" });
    m.insert("text_line_height", builtin_member! { "TextStyleWidget", Method, "text_style" });
    m.insert("text_overflow", builtin_member! { "TextStyleWidget", Method, "text_style" });
    // Padding
    m.insert("padding", builtin_member! {"Padding", Field, "padding"});
    // LayoutBox
    m.insert("layout_rect", builtin_member! {"LayoutBox", Method, "layout_box"});
    m.insert("layout_pos", builtin_member! {"LayoutBox", Method, "layout_box"});
    m.insert("layout_size", builtin_member! {"LayoutBox", Method, "layout_box"});
    m.insert("layout_left", builtin_member! {"LayoutBox", Method, "layout_box"});
    m.insert("layout_top", builtin_member! {"LayoutBox", Method, "layout_box"});
    m.insert("layout_width", builtin_member! {"LayoutBox", Method, "layout_box"});
    m.insert("layout_height", builtin_member! {"LayoutBox", Method, "layout_box"});
    // GlobalAnchor
    m.insert("global_anchor_x", builtin_member! {"GlobalAnchor", Field, "global_anchor_x"});
    m.insert("global_anchor_y", builtin_member! {"GlobalAnchor", Field, "global_anchor_y"});
    // Cursor
    m.insert("cursor", builtin_member! {"Cursor", Field, "cursor"});
    // Margin
    m.insert("margin", builtin_member! {"Margin", Field, "margin"});
    // ScrollableWidget
    m.insert("scrollable", builtin_member! {"ScrollableWidget", Field, "scrollable"});
    m.insert("get_scroll_pos", builtin_member! {"ScrollableWidget", Method, "scrollable"});
    m.insert("scroll_view_size", builtin_member! {"ScrollableWidget", Method, "scrollable"});
    m.insert("scroll_content_size", builtin_member! {"ScrollableWidget", Method, "scrollable"});
    m.insert("jump_to", builtin_member! {"ScrollableWidget", Method, "scrollable"});
    // ConstrainedBox
    m.insert("clamp", builtin_member! {"ConstrainedBox", Field, "constrained_box"});
    // TransformWidget
    m.insert("transform", builtin_member! {"TransformWidget", Field, "transform"});
    // HAlignWidget
    m.insert("h_align", builtin_member! {"HAlignWidget", Field, "h_align"});
    // VAlignWidget
    m.insert("v_align", builtin_member! {"VAlignWidget", Field, "v_align"});
    // RelativeAnchor
    m.insert("anchor", builtin_member! {"RelativeAnchor", Field, "relative_anchor"});
    // Visibility
    m.insert("visible", builtin_member! {"Visibility", Field, "visibility"});
    // Opacity
    m.insert("opacity", builtin_member! {"Opacity", Field, "opacity"});
    // KeepAlive
    m.insert("keep_alive", builtin_member! {"KeepAlive", Field, "keep_alive"});
    // Tooltips
    m.insert("tooltips", builtin_member! {"Tooltips", Field, "tooltips"});
    // TrackWidgetId
    m.insert("track_id", builtin_member! {"TrackWidgetId", Method, "track_id"});
    // ClipBoundary
    m.insert("clip_boundary", builtin_member! {"ClipBoundary", Field, "clip_boundary"});
    // Providers
    m.insert("providers", builtin_member! {"Providers", Field, "providers"});
    // Disabled
    m.insert("disabled", builtin_member! {"Disabled", Field, "disabled"});
    m
  });
