use ribir::prelude::*;

#[derive(Declare)]
struct P;

struct ChildA;
struct ChildB;
struct ChildC;

#[derive(Template)]
struct ChildTemplateOfP {
  _a: ChildA,
  _b: Option<ChildB>,
  _c: Option<ChildC>,
}

impl ComposeChild for P {
  type Child = ChildTemplateOfP;

  fn compose_child(_: StateWidget<Self>, _: Self::Child) -> Widget { Void.into_widget() }
}

#[test]
fn syntax_pass() {
  let _no_care_order = widget! {
    P {
      self::ChildC
      self::ChildA
      self::ChildB
    }
  };
  let _omit_option = widget! {
    P {
      self::ChildA
    }
  };
}
