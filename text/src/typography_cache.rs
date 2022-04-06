// use std::sync::{Arc, RwLock};

// use algo::FrameCache;
// use arcstr::Substr;

// use crate::layouter::{LayoutConfig, VisualInfos};

// #[derive(Clone, PartialEq, Eq, Hash)]
//  struct TypographyKey {
//   pub font_size: f32,
//   pub line_height: Option<f32>,
//   pub letter_space: Option<f32>,
//   pub h_align: Option<HAlign>,
//   pub v_align: Option<VAlign>,
//   pub line_dir: PlaceLineDirection,
//   pub overflow: Overflow,
//   pub text: Substr,
// }

// struct TypograpyRustult {
//    // The rect glyphs can place, and hint `TypographyMan` where to early
// return.   // the result of typography may over bounds.
//   pub bounds: Rect<f32>,
//   pub infos: VisualInfos,
// }

// /// Frame cache to store simple text layout result.
// #[derive(Clone, Default)]
// pub struct TypographyFrameCache {
//   cache: Arc<RwLock<FrameCache<TypographyKey, Arc<VisualInfos>>>>,
// }

// impl TypographyFrameCache {
//   pub fn end_frame(&self) {
// self.cache.write().unwrap().end_frame("Typography"); } }

// #[derive(Clone)]
// pub struct CfgKey(LayoutConfig);

// impl std::hash::Hash for CfgKey {
//   fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
//     let LayoutConfig {
//       font_size,
//       line_height,
//       letter_space,
//       h_align,
//       v_align,
//       bounds: _bounds,
//       line_dir,
//       overflow: _overflow,
//     } = self.0;
//   }
// }

// impl PartialEq for CfgKey {
//   fn eq(&self, other: &Self) -> bool { self.0 == other.0 }
// }

// impl Eq for CfgKey {
//   fn assert_receiver_is_total_eq(&self) {}
// }
