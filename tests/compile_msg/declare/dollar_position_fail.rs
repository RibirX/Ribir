use ribir::prelude::*;

fn main() {
  let not_identify_after_dollar = fn_widget! {
    rdl! { Row { x: $1 } }
  };

  let field_name_not_support_dollar = fn_widget! {
    rdl! { Row { $x: 1} }
  };
}
