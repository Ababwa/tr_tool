mod as_bytes;
mod boxed_slice;
mod core;
mod gfx;
mod level_parse;
mod object_data;
mod print_error;
mod push_get;
mod render_resources;
mod tr_traits;
mod wait;

use winit::{
	application::ApplicationHandler, error::EventLoopError, event::{DeviceEvent, DeviceId, WindowEvent},
	event_loop::{ActiveEventLoop, EventLoop}, window::WindowId,
};
use std::{env, path::Path};
use core::Core;
use print_error::PrintError;

type UserEvent = ();

struct Handler {
	core: Option<Core>,
}

#[cfg(target_os = "windows")]
const TASKBAR_ICON_BYTES: &[u8; 2304] = include_bytes!("res/icon24.data");
const WINDOW_ICON_BYTES: &[u8; 1024] = include_bytes!("res/icon16.data");

/// 4 MB
const GEOM_BUFFER_SIZE: usize = 4194304;

/// Round up to the nearest multiple of `N`.
const fn round_up<const N: usize>(a: usize) -> usize {
	a + (N - a % N) % N
}

impl ApplicationHandler<UserEvent> for Handler {
	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
		if let None = self.core {
			let mut core = Core::new(event_loop);
			let mut args = env::args_os();
			if let Some(path) = args.nth(1) {
				let path = Path::new(&path);
				core.try_load(path);
			}
			self.core = Some(core);
		}
	}
	
	fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
		if let Some(core) = &mut self.core {
			core.window_event(event_loop, &event);
		} else if let WindowEvent::CloseRequested = event {
			event_loop.exit();
		}
	}
	
	fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
		if let Some(core) = &mut self.core {
			core.device_event(&event);
		}
	}
}

fn start() -> Result<(), EventLoopError> {
	let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;
	let mut handler = Handler {
		core: None,
	};
	event_loop.run_app(&mut handler)
}

fn main() {
	start().print_err("event loop error");
}
