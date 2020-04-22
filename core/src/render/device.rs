pub(crate) trait AbstractDevice {
  type D;
  fn native_device(&self) -> Self::D;
}

#[cfg(target_os = "macos")]
mod osx_device;
#[cfg(target_os = "macos")]
pub(crate) use osx_device::DeviceOSX as Device;

#[cfg(all(not(target_os = "macos"),))]
compile_error!(
  "The platform you're compiling for is not supported by Holiday now!"
);
