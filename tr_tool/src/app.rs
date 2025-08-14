use winit::{
	application::ApplicationHandler, event::{DeviceEvent, DeviceId, KeyEvent, WindowEvent},
	event_loop::{ActiveEventLoop, EventLoop}, keyboard::PhysicalKey, window::WindowId,
};
use crate::core::Core;

macro_rules! key {
	($key_code:pat, $state:pat, $repeat:pat) => {
		WindowEvent::KeyboardInput {
			event: KeyEvent {
				physical_key: PhysicalKey::Code($key_code),
				state: $state,
				repeat: $repeat,
				..
			},
			..
		}
	};
}

struct State {
	egui_ctx: egui::Context,
	core: Core,
}

struct Handler {
	state: Option<State>,
}

impl State {
	fn new(event_loop: &ActiveEventLoop) -> Self {
		let egui_ctx = egui::Context::default();
		let core = Core::new(event_loop, egui_ctx.clone());
		Self {
			egui_ctx,
			core,
		}
	}
	
	fn window_event_not_egui(&mut self, event_loop: &ActiveEventLoop, event: &WindowEvent) {
		match event {
			WindowEvent::ModifiersChanged(modifiers) => self.core.modifiers(modifiers),
			&WindowEvent::Resized(new_size) => self.core.resize(new_size),
			WindowEvent::RedrawRequested => self.core.try_draw(&self.egui_ctx),
			&key!(key_code, state, repeat) => self.core.key(event_loop, key_code, state, repeat),
			&WindowEvent::MouseInput { state, button, .. } => self.core.mouse_button(state, button),
			&WindowEvent::CursorMoved { position, .. } => self.core.cursor_moved(position),
			_ => {},
		}
	}
	
	fn window_event(&mut self, event_loop: &ActiveEventLoop, event: &WindowEvent) {
		if !self.core.feed_egui(event) {
			self.window_event_not_egui(event_loop, event);
		}
	}
	
	fn device_event(&mut self, event: &DeviceEvent) {
		match event {
			&DeviceEvent::MouseMotion { delta: (x, y) } => self.core.mouse_motion(x as f32, y as f32),
			_ => {},
		}
	}
}

impl ApplicationHandler for Handler {
	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
		if let None = self.state {
			self.state = Some(State::new(event_loop));
		}
	}
	
	fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
		if let WindowEvent::CloseRequested = event {
			event_loop.exit();
		} else if let Some(state) = &mut self.state {
			state.window_event(event_loop, &event);
		}
	}
	
	fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
		if let Some(state) = &mut self.state {
			state.device_event(&event);
		}
	}
}

pub fn start() {
	let mut handler = Handler {
		state: None,
	};
	let event_loop = EventLoop::new().expect("new event loop");
	event_loop.run_app(&mut handler).expect("run app");
}
