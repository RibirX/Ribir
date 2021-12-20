pub trait Declare {
  type Builder: DeclareBuilder;
}

pub trait DeclareBuilder {
  type Target;
  fn build(self) -> Self::Target;
}
