#![cfg(target_os = "macos")]
use super::AbstractDevice;
use cocoa::{appkit::NSView, base::id as cocoa_id};
use metal::*;
use objc::runtime::YES;
use pathfinder_metal::MetalDevice;
use std::mem::transmute;
use winit::{platform::macos::WindowExtMacOS, window::Window as NativeWindow};

pub struct DeviceOSX {
  _device: Device,
  layer: CoreAnimationLayer,
}

impl DeviceOSX {
  pub(crate) fn new() -> Self {
    let device = Device::system_default().expect("no device found");

    let layer = CoreAnimationLayer::new();
    layer.set_device(&device);
    layer.set_pixel_format(MTLPixelFormat::BGRA8Unorm);
    layer.set_presents_with_transaction(false);

    DeviceOSX {
      layer,
      _device: device,
    }
  }

  pub(crate) fn attach(&self, window: &NativeWindow) {
    unsafe {
      let view = window.ns_view() as cocoa_id;
      view.setWantsLayer(YES);
      view.setLayer(transmute(self.layer.as_ref()));
    }
  }
}

impl AbstractDevice for DeviceOSX {
  type D = MetalDevice;
  #[inline]
  fn native_device(&self) -> Self::D { MetalDevice::new(self.layer.as_ref()) }
}
