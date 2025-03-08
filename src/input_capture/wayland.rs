// ISSUES WITH THIS:
// scale factor is not handeled
// trnsformations are not handeled
// not the best error handling

// TODO:
// add logging. instead of just ignoring th events, log them

use crate::input_capture::shmem;

use std::os::fd::AsFd;

use wayland_client::{
    delegate_noop,
    protocol::{wl_buffer, wl_output, wl_registry, wl_shm, wl_shm_pool},
    Connection, Dispatch, EventQueue, QueueHandle,
};
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_frame_v1;
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_manager_v1;
use wayland_protocols_wlr::virtual_pointer::v1::client::zwlr_virtual_pointer_manager_v1;
use wayland_protocols_wlr::virtual_pointer::v1::client::zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1;

use anyhow::Result;

#[derive(Debug)]
pub struct Output {
    pub wl_output: Option<wl_output::WlOutput>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub scale: Option<i32>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub done: bool,

    pub frame: Option<zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1>,
    pub buffer: Option<wl_buffer::WlBuffer>,
    pub readable_buffer: Option<&'static [u8]>,

    pub ready_for_copy: bool,
    pub ready_for_read: bool,

    pub vp: Option<ZwlrVirtualPointerV1>,
}

impl Output {
    pub fn new() -> Self {
        Self {
            wl_output: None,
            name: None,
            description: None,
            scale: None,
            width: None,
            height: None,
            done: false,

            frame: None,
            buffer: None,
            readable_buffer: None,

            ready_for_copy: false,
            ready_for_read: false,

            vp: None,
        }
    }
}

#[derive(Debug)]
pub struct State {
    // stuff we bind to
    pub wl_shm: Option<wl_shm::WlShm>,
    pub zwlr_screencopy_manager: Option<zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1>,
    pub zwlr_virtual_pointer_manager:
        Option<zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1>,

    pub outputs: Vec<Output>,
}

impl State {
    pub fn new() -> Self {
        Self {
            wl_shm: None,
            zwlr_screencopy_manager: None,
            zwlr_virtual_pointer_manager: None,
            outputs: Vec::new(),
        }
    }

    pub fn create_new_vp(
        &mut self,
        output_index: usize,
        qh: &QueueHandle<Self>,
        eq: &mut EventQueue<Self>,
    ) -> Result<()> {
        let vp_manager = self
            .zwlr_virtual_pointer_manager
            .as_ref()
            .expect("zwlr_virtual_pointer_manager not bound");
        let vp = Some(vp_manager.create_virtual_pointer_with_output(
            None,
            self.outputs[output_index].wl_output.as_ref(),
            qh,
            (),
        ));
        (&mut self.outputs[output_index]).vp = vp;

        eq.roundtrip(self)?;
        Ok(())
    }

    pub fn request_frame(
        &mut self,
        output_index: usize,
        qh: &QueueHandle<Self>,
        eq: &mut EventQueue<Self>,
    ) -> Result<()> {
        let output = &mut self.outputs[output_index];
        output.ready_for_copy = false;
        output.ready_for_read = false;

        // create a new frame
        if let Some(screencopy_manager) = &self.zwlr_screencopy_manager {
            output.frame = Some(screencopy_manager.capture_output(
                0,
                output.wl_output.as_ref().expect("wl_output not bound"),
                qh,
                output_index,
            ));
        }

        // wait for the frame to be ready for sending the copy request
        while self.outputs.first().map_or(true, |o| !o.ready_for_copy) {
            eq.blocking_dispatch(self)?;
        }

        let output = &mut self.outputs[output_index];
        let frame = output.frame.as_ref().expect("frame not bound");
        let buffer = output.buffer.as_ref().expect("buffer not bound");

        // send the copy request
        frame.copy(buffer);

        // wait for the frame to be ready for reading
        while self.outputs.first().map_or(true, |o| !o.ready_for_read) {
            eq.blocking_dispatch(self)?;
        }

        Ok(())
    }
}

delegate_noop!(State: ignore wl_shm::WlShm);
delegate_noop!(State: ignore wl_shm_pool::WlShmPool);
delegate_noop!(State: ignore wl_buffer::WlBuffer);
delegate_noop!(State: ignore zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1);
delegate_noop!(State: ignore zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1);
delegate_noop!(State: ignore ZwlrVirtualPointerV1);

impl Dispatch<zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1, usize> for State {
    fn event(
        state: &mut Self,
        _: &zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1,
        event: zwlr_screencopy_frame_v1::Event,
        index: &usize,
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        let output = &mut state.outputs[*index];

        match event {
            zwlr_screencopy_frame_v1::Event::Buffer {
                format,
                width,
                height,
                stride,
            } => {
                // only accept Xbgr8888 format. This may cause some issues in the future.
                if format.into_result().unwrap() != wl_shm::Format::Xbgr8888 {
                    return;
                }

                // create a new buffer if we don't have one.
                if output.wl_output.is_none() || output.buffer.is_none() {
                    let buffer_size = (stride * height) as usize;
                    let shm_fd = unsafe {
                        let (shm_fd, ptr) = shmem::create_shmem(buffer_size);
                        let readable_buffer = std::slice::from_raw_parts_mut(ptr, buffer_size);
                        output.readable_buffer = Some(readable_buffer);
                        shm_fd
                    };

                    let pool = state.wl_shm.as_ref().unwrap().create_pool(
                        shm_fd.as_fd(),
                        buffer_size as i32,
                        qh,
                        (),
                    );

                    output.buffer = Some(pool.create_buffer(
                        0,
                        width as i32,
                        height as i32,
                        stride as i32,
                        wl_shm::Format::Xbgr8888,
                        qh,
                        (),
                    ));
                }

                output.ready_for_copy = true;
            }
            zwlr_screencopy_frame_v1::Event::Ready { .. } => {
                output.ready_for_read = true;
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_output::WlOutput, usize> for State {
    fn event(
        state: &mut Self,
        _: &wl_output::WlOutput,
        event: wl_output::Event,
        index: &usize,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let output = &mut state.outputs[*index];
        match event {
            wl_output::Event::Mode { width, height, .. } => {
                output.width = Some(width);
                output.height = Some(height)
            }
            wl_output::Event::Scale { factor } => output.scale = Some(factor),
            wl_output::Event::Name { name } => output.name = Some(name.to_string()),
            wl_output::Event::Description { description } => {
                output.description = Some(description.to_string())
            }
            wl_output::Event::Done => output.done = true,
            _ => {}
        }
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for State {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match &interface[..] {
                "zwlr_virtual_pointer_manager_v1" => {
                    state.zwlr_virtual_pointer_manager = Some(registry.bind(name, version, qh, ()));
                }
                "zwlr_screencopy_manager_v1" => {
                    state.zwlr_screencopy_manager = Some(registry.bind(name, version, qh, ()));
                }
                "wl_output" => {
                    let wl_output =
                        Some(registry.bind(name, version, qh, state.outputs.len() as usize));
                    let mut output = Output::new();
                    output.wl_output = wl_output;
                    state.outputs.push(output);
                }
                "wl_shm" => {
                    state.wl_shm = Some(registry.bind(name, version, qh, ()));
                }
                _ => {}
            }
        }
    }
}
