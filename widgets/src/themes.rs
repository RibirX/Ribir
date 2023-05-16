mod styles_sheet;
use ribir_core::{fill_svgs, prelude::*};
pub use styles_sheet::*;

pub(crate) fn add_to_theme(theme: &mut FullTheme) {
  fill_svgs! {
    theme.icon_theme,
    svgs::ADD: "./themes/icons/add_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::ARROW_BACK: "./themes/icons/arrow_back_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::ARROW_DROP_DOWN: "./themes/icons/arrow_drop_down_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::ARROW_FORWARD: "./themes/icons/arrow_forward_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::CANCEL: "./themes/icons/cancel_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::CHECK_BOX: "./themes/icons/check_box_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::CHECK_BOX_OUTLINE_BLANK: "./themes/icons/check_box_outline_blank_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::CHEVRON_RIGHT: "./themes/icons/chevron_right_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::CLOSE: "./themes/icons/close_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::DELETE: "./themes/icons/delete_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::DONE: "./themes/icons/done_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::EXPAND_MORE: "./themes/icons/expand_more_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::FAVORITE: "./themes/icons/favorite_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::HOME: "./themes/icons/home_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::INDETERMINATE_CHECK_BOX: "./themes/icons/indeterminate_check_box_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::LOGIN: "./themes/icons/login_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::LOGOUT: "./themes/icons/logout_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::MENU: "./themes/icons/menu_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::MORE_HORIZ: "./themes/icons/more_horiz_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::MORE_VERT: "./themes/icons/more_vert_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::OPEN_IN_NEW: "./themes/icons/open_in_new_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::SEARCH: "./themes/icons/search_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::SETTINGS: "./themes/icons/settings_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::STAR: "./themes/icons/star_FILL0_wght400_GRAD0_opsz48.svg",
    svgs::TEXT_CARET: "./themes/icons/text_caret.svg"
  }
}
