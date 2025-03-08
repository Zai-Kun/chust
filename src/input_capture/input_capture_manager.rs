#[cfg(target_os = "linux")]
use crate::input_capture::wayland::State;
#[cfg(target_os = "linux")]
use std::time::{SystemTime, UNIX_EPOCH};
#[cfg(target_os = "linux")]
use wayland_client::protocol::wl_pointer::ButtonState;
#[cfg(target_os = "linux")]
use wayland_client::{Connection, EventQueue, QueueHandle};

use crate::input_capture::InputCaptureTrait;
use anyhow::{Context, Result};
use enigo::Button as EnigoButton;
use enigo::Coordinate::Abs;
use enigo::Direction::{Click, Release};
use enigo::Mouse;
use imageproc::image;
use imageproc::image::DynamicImage;
use imageproc::image::{ImageBuffer, Rgb};

use std::process::{Command, Stdio};
use xcap::Monitor;

const LEFT_BUTTON: u32 = 0x110;

// for windows, linux (x11), and macos
pub struct InputCapture {
    pub enigo: enigo::Enigo,
    pub monitor: Monitor,
}

impl InputCapture {
    pub fn new(output_index: usize) -> Result<Self> {
        let enigo = enigo::Enigo::new(&enigo::Settings::default())?;
        let monitor = Monitor::all()?
            .into_iter()
            .nth(output_index)
            .context("No monitor found")?;

        Ok(Self { enigo, monitor })
    }
}

impl InputCaptureTrait for InputCapture {
    fn screenshot(&mut self) -> Result<DynamicImage> {
        Ok(DynamicImage::ImageRgba8(self.monitor.capture_image()?))
    }

    fn click_at(&mut self, x: u32, y: u32) -> Result<()> {
        self.enigo.move_mouse(x as i32, y as i32, Abs)?;
        self.enigo.button(EnigoButton::Left, Click)?;
        self.enigo.button(EnigoButton::Left, Release)?;
        Ok(())
    }
}

// For linux (wayland)
#[cfg(target_os = "linux")]
pub struct InputCaptureWayland {
    pub state: State,
    pub event_queue: EventQueue<State>,
    pub event_queue_handle: QueueHandle<State>,
    pub output_index: usize,
}

#[cfg(target_os = "linux")]
impl InputCaptureWayland {
    pub fn new(output_index: usize) -> Result<Self> {
        let connection = Connection::connect_to_env()?;
        let mut event_queue = connection.new_event_queue::<State>();
        let qhandle = event_queue.handle();
        let mut state = State::new();
        let display = connection.display();
        display.get_registry(&qhandle, ());
        event_queue.roundtrip(&mut state)?;
        state.create_new_vp(output_index, &qhandle, &mut event_queue)?;

        Ok(Self {
            state,
            event_queue,
            event_queue_handle: qhandle,
            output_index,
        })
    }
}

#[cfg(target_os = "linux")]
impl InputCaptureTrait for InputCaptureWayland {
    fn screenshot(&mut self) -> Result<DynamicImage> {
        self.state.request_frame(
            self.output_index,
            &self.event_queue_handle,
            &mut self.event_queue,
        )?;

        let output = &self.state.outputs[self.output_index];
        let buffer = output.readable_buffer.unwrap();
        let width = output.width.unwrap() as u32;
        let height = output.height.unwrap() as u32;

        Ok(from_xbgr8888(width, height, buffer))
    }

    fn click_at(&mut self, x: u32, y: u32) -> Result<()> {
        let output = &self.state.outputs[self.output_index];
        let w = output.width.unwrap();
        let h = output.height.unwrap();
        let vp = output.vp.as_ref().unwrap();

        vp.motion_absolute(time(), x, y, w as u32, h as u32);
        vp.button(time(), LEFT_BUTTON, ButtonState::Pressed);
        vp.button(time(), LEFT_BUTTON, ButtonState::Released);
        self.event_queue.roundtrip(&mut self.state)?;

        Ok(())
    }
}

pub struct CustomInputCapture {
    input_capture: Option<Box<dyn InputCaptureTrait>>,

    custom_screenshot_command: Option<String>,
    custom_click_command: Option<String>,
}

// for when the user wishes to use a custom screenshot/click command
impl CustomInputCapture {
    pub fn new(
        input_capture: Option<Box<dyn InputCaptureTrait>>,
        custom_screenshot_command: Option<String>,
        custom_click_command: Option<String>,
    ) -> Result<Self> {
        if custom_click_command.is_none() && custom_screenshot_command.is_none() {
            return Err(anyhow::anyhow!(
                "Either input_capture or custom_screenshot_command must be provided"
            ));
        }

        Ok(Self {
            input_capture,
            custom_screenshot_command,
            custom_click_command,
        })
    }

    fn execute_command(command: &str, return_output: bool) -> Result<Option<Vec<u8>>> {
        let mut parts = command.split_whitespace();
        let cmd = parts.next().context("No command provided")?;
        let args: Vec<&str> = parts.collect(); // Remaining parts are arguments

        let mut process = Command::new(cmd)
            .args(args)
            .stdout(if return_output {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stderr(Stdio::null()) // Suppress stderr
            .spawn()
            .context(format!("Failed to execute {}", command))?; // Handle errors gracefully

        if return_output {
            let output = process.wait_with_output()?;
            Ok(Some(output.stdout))
        } else {
            process.wait()?;
            Ok(None)
        }
    }
}

impl InputCaptureTrait for CustomInputCapture {
    fn screenshot(&mut self) -> Result<DynamicImage> {
        if let Some(ss_command) = &self.custom_screenshot_command {
            let output = Self::execute_command(ss_command, true)?
                .context("No output from custom screenshot command")?;
            Ok(image::load_from_memory(&output)
                .context("Failed to load image from custom screenshot command output")?)
        } else {
            self.input_capture
                .as_mut()
                .context("No input capture provided")?
                .screenshot()
        }
    }

    fn click_at(&mut self, x: u32, y: u32) -> Result<()> {
        if let Some(click_command) = &self.custom_click_command {
            Self::execute_command(
                &click_command
                    .replace("{x}", &x.to_string())
                    .replace("{y}", &y.to_string()),
                false,
            )?;
        } else {
            self.input_capture
                .as_mut()
                .context("No input capture provided")?
                .click_at(x, y)?;
        }
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn from_xbgr8888(width: u32, height: u32, data: &'static [u8]) -> DynamicImage {
    let new_len = (data.len() / 4) * 3;
    let mut rgb_data = Vec::with_capacity(new_len);
    for chunk in data.chunks_exact(4) {
        let r = chunk[0];
        let g = chunk[1];
        let b = chunk[2];
        rgb_data.extend_from_slice(&[r, g, b]);
    }

    let img_buffer: ImageBuffer<Rgb<u8>, Vec<u8>> =
        ImageBuffer::from_raw(width, height, rgb_data).expect("Failed to create ImageBuffer");

    DynamicImage::ImageRgb8(img_buffer)
}

#[cfg(target_os = "linux")]
fn time() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u32
}

#[cfg(target_os = "linux")]
fn on_wayland() -> bool {
    std::env::var("WAYLAND_DISPLAY").is_ok()
}

pub fn create_input_capture(
    output_index: usize,
    custom_click_command: Option<String>,
    custom_screenshot_command: Option<String>,
) -> Result<Box<dyn InputCaptureTrait>> {
    if custom_click_command.is_some() && custom_screenshot_command.is_some() {
        return Ok(Box::new(CustomInputCapture::new(
            None,
            custom_screenshot_command,
            custom_click_command,
        )?));
    }

    let input_capture: Box<dyn InputCaptureTrait> = if cfg!(target_os = "linux") && on_wayland() {
        Box::new(InputCaptureWayland::new(output_index)?)
    } else {
        Box::new(InputCapture::new(output_index)?)
    };

    if custom_screenshot_command.is_some() || custom_click_command.is_some() {
        Ok(Box::new(CustomInputCapture::new(
            Some(input_capture),
            custom_screenshot_command,
            custom_click_command,
        )?))
    } else {
        Ok(input_capture)
    }
}
