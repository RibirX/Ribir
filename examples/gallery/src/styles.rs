use ribir::prelude::*;

class_names! {
  GALLERY_CONTENT_SHELL,
  GALLERY_BREADCRUMB_SEPARATOR,
  GALLERY_PAGE,
  GALLERY_PAGE_HEADER,
  GALLERY_PAGE_TITLE,
  GALLERY_PAGE_LEAD,
  GALLERY_STATUS_PANEL,
  GALLERY_STATUS_BADGE,
  GALLERY_STATUS_TITLE,
  GALLERY_STATUS_BODY,
}

pub fn styles() -> Vec<Provider> {
  vec![
    Class::provider(
      GALLERY_CONTENT_SHELL,
      style_class! {
        clip_boundary: true,
        radius: Radius::all(32.),
        background: Palette::of(BuildCtx::get()).surface_container_low(),
      },
    ),
    Class::provider(
      GALLERY_BREADCRUMB_SEPARATOR,
      style_class! {
        foreground: Palette::of(BuildCtx::get()).on_surface(),
        opacity: 0.48,
      },
    ),
    Class::provider(
      GALLERY_PAGE,
      style_class! {
        padding: EdgeInsets::all(48.),
      },
    ),
    Class::provider(
      GALLERY_PAGE_HEADER,
      style_class! {
        margin: EdgeInsets::only_bottom(32.),
      },
    ),
    Class::provider(
      GALLERY_PAGE_TITLE,
      style_class! {
        text_style: TypographyTheme::of(BuildCtx::get()).headline_large.text.clone(),
        foreground: Palette::of(BuildCtx::get()).on_surface(),
        text_overflow: TextOverflow::AutoWrap,
      },
    ),
    Class::provider(
      GALLERY_PAGE_LEAD,
      style_class! {
        text_style: TypographyTheme::of(BuildCtx::get()).title_medium.text.clone(),
        foreground: Palette::of(BuildCtx::get()).on_surface_variant(),
        text_overflow: TextOverflow::AutoWrap,
      },
    ),
    Class::provider(
      GALLERY_STATUS_PANEL,
      style_class! {
        min_height: 320.,
        padding: EdgeInsets::all(48.),
        radius: Radius::all(48.),
        background: Palette::of(BuildCtx::get()).surface_container_highest(),
      },
    ),
    Class::provider(
      GALLERY_STATUS_BADGE,
      style_class! {
        padding: EdgeInsets::symmetrical(6., 16.),
        radius: Radius::all(20.),
        background: Palette::of(BuildCtx::get()).tertiary_container(),
        text_style: TypographyTheme::of(BuildCtx::get()).label_large.text.clone(),
        foreground: Palette::of(BuildCtx::get()).on_tertiary_container(),
        text_align: TextAlign::Center,
      },
    ),
    Class::provider(
      GALLERY_STATUS_TITLE,
      style_class! {
        margin: EdgeInsets::only_top(16.),
        text_style: TypographyTheme::of(BuildCtx::get()).headline_small.text.clone(),
        foreground: Palette::of(BuildCtx::get()).on_surface(),
        text_align: TextAlign::Center,
        max_width: 560.,
        text_overflow: TextOverflow::AutoWrap,
      },
    ),
    Class::provider(
      GALLERY_STATUS_BODY,
      style_class! {
        margin: EdgeInsets::only_top(16.),
        text_style: TypographyTheme::of(BuildCtx::get()).body_large.text.clone(),
        foreground: Palette::of(BuildCtx::get()).on_surface_variant(),
        text_align: TextAlign::Center,
        max_width: 560.,
        text_overflow: TextOverflow::AutoWrap,
      },
    ),
  ]
}
