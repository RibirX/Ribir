#[cfg(feature = "wgpu")]
mod wgpu_backend;
#[cfg(feature = "wgpu")]
pub(crate) use wgpu_backend::WgpuBackend as Backend;

#[cfg(not(feature = "wgpu"))]
mod mock_backend;
#[cfg(not(feature = "wgpu"))]
pub(crate) use mock_backend::MockBackend as Backend;
