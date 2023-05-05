use ribir::{core::test::expect_layout_result, prelude::*};

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
  type Target = Widget;
  fn compose_child(_: State<Self>, _: Self::Child) -> Widget { Void.into_widget() }
}

#[derive(Declare)]
struct P2;

#[derive(Template)]
struct TupleStructTemplate(ChildA, Option<ChildB>, Option<ChildC>);

impl ComposeChild for P2 {
  type Child = TupleStructTemplate;
  type Target = Widget;
  fn compose_child(_: State<Self>, _: Self::Child) -> Widget { Void.into_widget() }
}

#[derive(Declare)]
struct P3;

#[derive(Template)]
enum EnumTml {
  A(ChildA),
  B(ChildB),
  C(ChildC),
}

impl ComposeChild for P3 {
  type Child = EnumTml;
  type Target = Widget;
  fn compose_child(_: State<Self>, _: Self::Child) -> Widget { Void.into_widget() }
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

#[test]
fn tuple_struct_template_syntax_pass() {
  let _no_care_order = widget! {
    P2 {
      self::ChildC
      self::ChildA
      self::ChildB
    }
  };
  let _omit_option = widget! {
    P2 { self::ChildA }
  };
}

#[test]
fn enum_template() {
  let _a = widget! {
    P3 { self::ChildA }
  };
  let _b = widget! {
    P { self::ChildB }
  };
  let _c = widget! {
    P { self::ChildC }
  };
}

#[test]
#[should_panic = "Try to fill enum template with two variant."]
fn panic_multi_enum_variant() {
  let a = widget! {
    P3 {
      self::ChildA
      self::ChildB
    }
  };
  expect_layout_result(a, None, &[]);
}
