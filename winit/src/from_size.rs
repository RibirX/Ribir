use ribir_geometry::{
  DeviceOffset as RibirPhysicalOffset, DevicePoint as RibirPhysicalPosition,
  DeviceSize as RibirPhysicalSize, Point as RibirLogicalPosition, Size as RibirLogicalSize,
};

use std::fmt::Debug;
use winit::dpi::Pixel;

pub use winit::dpi::LogicalPosition as WinitLogicalPosition;
pub use winit::dpi::LogicalSize as WinitLogicalSize;
pub use winit::dpi::PhysicalPosition as WinitPhysicalPosition;
pub use winit::dpi::PhysicalSize as WinitPhysicalSize;

#[derive(Debug)]
pub struct WrappedPhysicalSize<T: Pixel>(WinitPhysicalSize<T>);

impl<T: Pixel> From<WinitPhysicalSize<T>> for WrappedPhysicalSize<T> {
  fn from(value: WinitPhysicalSize<T>) -> Self { WrappedPhysicalSize(value) }
}

impl<T: Pixel> From<WrappedPhysicalSize<T>> for WinitPhysicalSize<T> {
  fn from(value: WrappedPhysicalSize<T>) -> Self { value.0 }
}

impl<T: Pixel> From<WrappedPhysicalSize<T>> for RibirPhysicalSize {
  fn from(value: WrappedPhysicalSize<T>) -> Self {
    RibirPhysicalSize::new(value.0.width.cast(), value.0.height.cast())
  }
}

impl<T: Pixel> From<RibirPhysicalSize> for WrappedPhysicalSize<T> {
  fn from(value: RibirPhysicalSize) -> Self {
    WrappedPhysicalSize::<T>::from(WinitPhysicalSize::<T>::new(
      value.width.cast(),
      value.height.cast(),
    ))
  }
}

// impl<'a, T: Pixel> From<&'a mut WinitPhysicalSize<T>> for
// WrappedPhysicalSize<T> {   fn from(value: &'a mut WinitPhysicalSize<T>) ->
// Self { WrappedPhysicalSize::from(value).into() } }

// impl<'a, T: Pixel> From<Box<dyn MutDeviceSize>> for
// WrappedMutPhysicalSize<'a, T> {   fn from(value: Box<dyn MutDeviceSize>) ->
// Self { todo!() } }

pub struct WrappedLogicalSize<T: Pixel>(WinitLogicalSize<T>);

impl<T: Pixel> From<WinitLogicalSize<T>> for WrappedLogicalSize<T> {
  fn from(value: WinitLogicalSize<T>) -> Self { WrappedLogicalSize(value) }
}

impl<T: Pixel> From<WrappedLogicalSize<T>> for WinitLogicalSize<T> {
  fn from(value: WrappedLogicalSize<T>) -> Self { value.0 }
}

impl<T: Pixel> From<WrappedLogicalSize<T>> for RibirLogicalSize {
  fn from(value: WrappedLogicalSize<T>) -> Self {
    RibirLogicalSize::new(value.0.width.cast(), value.0.height.cast())
  }
}

impl<T: Pixel> From<RibirLogicalSize> for WrappedLogicalSize<T> {
  fn from(value: RibirLogicalSize) -> Self {
    WinitLogicalSize::new(
      Pixel::from_f64(value.width as f64),
      Pixel::from_f64(value.height as f64),
    )
    .into()
  }
}

pub struct WrappedPhysicalPosition<T: Pixel>(WinitPhysicalPosition<T>);

impl<T: Pixel> From<WinitPhysicalPosition<T>> for WrappedPhysicalPosition<T> {
  fn from(value: WinitPhysicalPosition<T>) -> Self { WrappedPhysicalPosition(value) }
}

impl<T: Pixel> From<WrappedPhysicalPosition<T>> for WinitPhysicalPosition<T> {
  fn from(value: WrappedPhysicalPosition<T>) -> Self { value.0 }
}

impl<T: Pixel> From<WrappedPhysicalPosition<T>> for RibirPhysicalPosition {
  fn from(value: WrappedPhysicalPosition<T>) -> Self {
    RibirPhysicalPosition::new(value.0.x.cast(), value.0.y.cast())
  }
}

impl<T: Pixel> From<WrappedPhysicalPosition<T>> for RibirPhysicalOffset {
  fn from(value: WrappedPhysicalPosition<T>) -> Self {
    RibirPhysicalOffset::new(value.0.x.cast(), value.0.y.cast())
  }
}

impl<T: Pixel> From<RibirPhysicalPosition> for WrappedPhysicalPosition<T> {
  fn from(value: RibirPhysicalPosition) -> Self {
    WinitPhysicalPosition::new(
      Pixel::from_f64(value.x as f64),
      Pixel::from_f64(value.y as f64),
    )
    .into()
  }
}

pub struct WrappedLogicalPosition<T: Pixel>(WinitLogicalPosition<T>);

impl<T: Pixel> From<WinitLogicalPosition<T>> for WrappedLogicalPosition<T> {
  fn from(value: WinitLogicalPosition<T>) -> Self { WrappedLogicalPosition(value) }
}

impl<T: Pixel> From<WrappedLogicalPosition<T>> for WinitLogicalPosition<T> {
  fn from(value: WrappedLogicalPosition<T>) -> Self { value.0 }
}

impl<T: Pixel> From<WrappedLogicalPosition<T>> for RibirLogicalPosition {
  fn from(value: WrappedLogicalPosition<T>) -> Self {
    RibirLogicalPosition::new(value.0.x.cast(), value.0.y.cast())
  }
}

impl<T: Pixel> From<RibirLogicalPosition> for WrappedLogicalPosition<T> {
  fn from(value: RibirLogicalPosition) -> Self {
    WinitLogicalPosition::new(
      Pixel::from_f64(value.x as f64),
      Pixel::from_f64(value.y as f64),
    )
    .into()
  }
}

#[cfg(test)]
mod tests {

  use super::*;

  #[test]
  fn from_winit() {
    let width = 3;
    let height = 4;
    let winit_phy_size = WinitPhysicalSize::new(width, height);
    assert_eq!(winit_phy_size.width, width);
    assert_eq!(winit_phy_size.height, height);
  }
}
