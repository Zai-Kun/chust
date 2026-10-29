#[cfg(target_os = "linux")]
mod shmem;
#[cfg(target_os = "linux")]
mod wayland;

pub mod input_capture_manager;

use anyhow::Result;
use imageproc::image::DynamicImage;

pub trait InputCaptureTrait {
    fn screenshot(&mut self) -> Result<DynamicImage>;
    fn click_at(&mut self, x: u32, y: u32) -> Result<()>;
    fn drag(&mut self, x1: u32, y1: u32, x2: u32, y2: u32) -> Result<()>;
}
