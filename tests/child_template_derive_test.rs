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

  fn compose_child(_: impl StateWriter<Value = Self>, _: Self::Child) -> impl WidgetBuilder {
    fn_widget!(Void)
  }
}

#[derive(Declare)]
struct P2;

#[derive(Template)]
struct TupleStructTemplate(ChildA, Option<ChildB>, Option<ChildC>);

impl ComposeChild for P2 {
  type Child = TupleStructTemplate;

  fn compose_child(_: impl StateWriter<Value = Self>, _: Self::Child) -> impl WidgetBuilder {
    fn_widget!(Void)
  }
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

  fn compose_child(_: impl StateWriter<Value = Self>, _: Self::Child) -> impl WidgetBuilder {
    fn_widget!(Void)
  }
}

#[test]
fn syntax_pass() {
  let _no_care_order = fn_widget! {
    @P {
      @{self::ChildC}
      @{self::ChildA}
      @{self::ChildB}
    }
  };
  let _omit_option = fn_widget! {
    @P {
      @{self::ChildA}
    }
  };
}

#[test]
fn tuple_struct_template_syntax_pass() {
  let _no_care_order = fn_widget! {
    @P2 {
      @{ self::ChildC }
      @{ self::ChildA }
      @{ self::ChildB }
    }
  };
  let _omit_option = fn_widget! {
    @P2 { @{self::ChildA} }
  };
}

#[test]
fn enum_template() {
  let _a = fn_widget! {
    @P3 { @{ self::ChildA } }
  };
  let _b = fn_widget! {
    @P { @{ self::ChildB } }
  };
  let _c = fn_widget! {
    @P { @{ self::ChildC } }
  };
}
