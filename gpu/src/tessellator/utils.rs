use guillotiere::Allocation;
use ribir_painter::DeviceRect;

pub fn allocation_to_rect(alloc: &Allocation) -> DeviceRect {
  alloc.rectangle.to_rect().to_u32().cast_unit()
}
