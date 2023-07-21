mod app;
mod debug;
mod renderer;
mod requirements_filters;
mod surface;
mod swapchain;
mod shaders;

use crate::app::App;

use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use std::{borrow::{Cow, Borrow}, ffi::CStr, fs::{File, OpenOptions}, io::Write};
use anyhow::{Result, Context};

fn main() -> Result<()> {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut app = App::new(
        &window,
        window.inner_size().width,
        window.inner_size().height,
    ).with_context(|| "Could not create app.")?;
    let mut dirty_swapchain = false;

    event_loop.run(move |event, _, control_flow| {
        let app = &mut app;

        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event,
                window_id: _,
            } => {
                match event {
                    WindowEvent::Resized(_) => {
                        dirty_swapchain = true;
                    }
                    WindowEvent::ScaleFactorChanged { .. } => {
                        //
                    }
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    _ => (),
                }
            }
            Event::MainEventsCleared => {
                // render
                {
                    if dirty_swapchain {
                        let size = window.inner_size();
                        if size.width > 0 && size.height > 0 {
                            app.recreate_swapchain(size.width, size.height);
                        } else {
                            return;
                        }
                    }
                    dirty_swapchain = app.render();
                }
            }
            // Event::LoopDestroyed => app.wait_gpu_idle(),
            _ => (),
        }
    });
}
