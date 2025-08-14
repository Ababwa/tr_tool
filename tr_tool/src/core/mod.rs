mod file_dialog;
mod keys;
mod loaded_level;

use std::{any::{type_name_of_val, Any}, fmt::Display, iter, num::NonZero, path::Path, sync::{mpsc, Arc}, thread, time::Instant};
use wgpu::{
	CommandEncoderDescriptor, Device, DeviceDescriptor, Features, Instance, Limits, MemoryHints,
	PowerPreference, Queue, RequestAdapterOptions, Surface, SurfaceConfiguration, TextureViewDescriptor,
	Trace,
};
use winit::{
	dpi::{PhysicalPosition, PhysicalSize}, event::{ElementState, Modifiers, MouseButton, WindowEvent},
	event_loop::ActiveEventLoop, keyboard::{KeyCode, ModifiersState},
	window::{Icon, Window, WindowAttributes},
};
use crate::{
	gfx, level::LevelData, render_resources::RenderResources, wait::Wait, GEOM_BUFFER_SIZE,
	WINDOW_ICON_BYTES,
};
use file_dialog::FileDialog;
use loaded_level::LoadedLevel;

pub struct Core {
	window: Arc<Window>,
	device: Device,
	queue: Queue,
	window_size: PhysicalSize<u32>,
	surface: Surface<'static>,
	config: SurfaceConfiguration,
	egui_input_state: egui_winit::State,
	egui_renderer: egui_wgpu::Renderer,
	draw: bool,
	last_frame: Instant,
	rr: RenderResources,
	modifiers: ModifiersState,
	file_dialog: FileDialog,
	error: Option<String>,
	loaded_level: Option<LoadedLevel>,
}

type Ui<'a> = &'a mut egui::Ui;

const WINDOW_TITLE: &str = "TR Tool";

fn draw_window<R, F: FnOnce(&mut egui::Ui) -> R>(
	ctx: &egui::Context,
	title: &str,
	resizable: bool,
	open: &mut bool,
	contents: F,
) -> Option<R> {
	egui::Window::new(title).resizable(resizable).open(open).show(ctx, contents)?.inner
}

fn file_name(path: &Path) -> Option<&str> {
	path.file_name()?.to_str()
}

#[cfg(target_os = "windows")]
mod ww {
	use wgpu::rwh::{DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle};
	use winit::{platform::windows::WindowExtWindows, window::Window};
	#[derive(Clone, Copy)]
	pub struct WindowWrapper<'a>(pub &'a Window);
	impl<'a> HasWindowHandle for WindowWrapper<'a> {
		fn window_handle(&self) -> Result<WindowHandle<'a>, HandleError> {
			unsafe {
				self.0.window_handle_any_thread()
			}
		}
	}
	impl<'a> HasDisplayHandle for WindowWrapper<'a> {
		fn display_handle(&self) -> Result<DisplayHandle<'a>, HandleError> {
			self.0.display_handle()
		}
	}
}

fn paint_setup(window: Arc<Window>, window_size: PhysicalSize<u32>, rx: mpsc::Receiver<()>) {
	#[cfg(target_os = "windows")]
	let window = ww::WindowWrapper(&window);
	#[cfg(not(target_os = "windows"))]
	let window = &*window;
	let sb_ctx = softbuffer::Context::new(window).expect("sb surface");
	let mut sb_surface = softbuffer::Surface::new(&sb_ctx, window).expect("sb surface");
	if let (Some(w), Some(h)) = (NonZero::new(window_size.width), NonZero::new(window_size.height)) {
		sb_surface.resize(w, h).expect("sb resize");
		let mut t = 0;
		while let Err(mpsc::TryRecvError::Empty) = rx.try_recv() {
			let mut buffer = sb_surface.buffer_mut().expect("sb buffer_mut");
			for i in 0..buffer.len() as u32 {
				let pixel = (((i % w) + (i / w) + 100000000 - t) % 46 / 23) * 0x111111 + 0x222222;
				buffer[i as usize] = pixel;
			}
			buffer.present().expect("sb present");
			t += 1;
		}
	} else {
		rx.recv().expect("setup painter block recv");
	}
}

impl Core {
	pub fn new(event_loop: &ActiveEventLoop, egui_ctx: egui::Context) -> Self {
		//TODO: animated stripes while waiting for everything (softbuffer)
		let window_icon = Icon::from_rgba(WINDOW_ICON_BYTES.to_vec(), 16, 16).expect("window icon");
		let window_attributes = WindowAttributes::default()
			.with_title(WINDOW_TITLE)
			.with_min_inner_size(PhysicalSize::new(1, 1))
			.with_window_icon(Some(window_icon));
		#[cfg(target_os = "windows")]
		let window_attributes = {
			use winit::platform::windows::WindowAttributesExtWindows;
			use crate::TASKBAR_ICON_BYTES;
			let taskbar_icon = Icon::from_rgba(TASKBAR_ICON_BYTES.to_vec(), 24, 24).expect("taskbar icon");
			window_attributes.with_taskbar_icon(Some(taskbar_icon))
		};
		let window = event_loop.create_window(window_attributes).expect("create window");
		let window = Arc::new(window);
		let painter_window = window.clone();
		let window_size = window.inner_size();
		let (tx, rx) = mpsc::channel::<()>();
		let setup_painter = thread::spawn(move || paint_setup(painter_window, window_size, rx));
		let instance = Instance::default();
		let surface = instance.create_surface(window.clone()).expect("create surface");//2000ms
		let req_adapter_options = RequestAdapterOptions {
			power_preference: PowerPreference::HighPerformance,
			force_fallback_adapter: false,
			compatible_surface: Some(&surface),
		};
		let adapter = instance.request_adapter(&req_adapter_options).wait().expect("request adapter");//430ms
		let mut required_limits = Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits());
		required_limits.max_storage_buffers_per_shader_stage = 1;
		required_limits.max_storage_buffer_binding_size = GEOM_BUFFER_SIZE as u32;
		required_limits.max_texture_array_layers = 512;
		let device_desc = DeviceDescriptor {
			label: None,
			required_features: Features::empty(),
			required_limits,
			memory_hints: MemoryHints::Performance,
			trace: Trace::Off,
		};
		let (device, queue) = adapter.request_device(&device_desc).wait().expect("request device");//250ms
		let config_result = surface.get_default_config(&adapter, window_size.width, window_size.height);
		let mut config = config_result.expect("get default config");
		config.format = gfx::TEXTURE_FORMAT;
		surface.configure(&device, &config);//250ms
		let viewport_id = egui_ctx.viewport_id();
		let egui_input_state = egui_winit::State::new(egui_ctx, viewport_id, &window, None, None, None);
		let egui_renderer = egui_wgpu::Renderer::new(&device, gfx::TEXTURE_FORMAT, None, 1, false);//don't know what `dithering` does here
		let rr = RenderResources::new(&device);
		let file_dialog = FileDialog::new();
		let last_frame = Instant::now();
		if let Err(e) = tx.send(()) {
			eprintln!("Error notifying setup painter: {}", e);
		}
		if let Err(e) = setup_painter.join() {
			//TODO: try to extract `Display` from e.
			eprintln!("Error joining setup painter: {:?}", e);
		}
		window.request_redraw();
		Self {
			window,
			device,
			queue,
			window_size,
			surface,
			config,
			egui_input_state,
			egui_renderer,
			draw: window_size.width > 0 && window_size.height > 0,
			last_frame,
			rr,
			modifiers: ModifiersState::empty(),
			file_dialog,
			error: None,
			loaded_level: None,
		}
	}
	
	pub fn feed_egui(&mut self, event: &WindowEvent) -> bool {
		self.egui_input_state.on_window_event(&self.window, event).consumed
	}
	
	pub fn modifiers(&mut self, modifiers: &Modifiers) {
		self.modifiers = modifiers.state();
	}
	
	pub fn resize(&mut self, window_size: PhysicalSize<u32>) {
		self.draw = window_size.width > 0 && window_size.height > 0;
		if self.draw {
			self.window_size = window_size;
			self.config.width = window_size.width;
			self.config.height = window_size.height;
			self.surface.configure(&self.device, &self.config);
			if let Some(loaded_level) = &mut self.loaded_level {
				let perspective_transform_buffer = &self.rr.binding_buffers.perspective_transform_buffer;
				loaded_level.resize(&self.device, &self.queue, perspective_transform_buffer, window_size);
			}
		}
	}
	
	fn egui(&mut self, ctx: &egui::Context) {
		self.file_dialog.update(ctx);
		if let Some(path) = self.file_dialog.get_level_path() {
			match LevelData::new(&self.device, &self.queue, &self.rr, &path) {
				Ok(level_data) => {
					let level = LoadedLevel::new(
						self.window_size,
						&self.device,
						&self.queue,
						&self.rr,
						level_data,
					);
					let title = match file_name(&path) {
						Some(name) => &format!("{} - {}", WINDOW_TITLE, name),
						None => WINDOW_TITLE,
					};
					self.window.set_title(title);
					self.loaded_level = Some(level);
				},
				Err(e) => self.error = Some(e.to_string()),
			}
		}
		if let Some(error) = &self.error {
			let mut show = true;
			draw_window(ctx, "Error", false, &mut show, |ui| ui.label(error));
			if !show {
				self.error = None;
			}
		}
		match &mut self.loaded_level {
 			Some(loaded_level) => loaded_level.egui(ctx),
			None => {
				let cb = |ui: Ui| {
					let cb = |ui: Ui| {
						if ui.label("Ctrl+O or click to open file").clicked() {
							self.file_dialog.pick_level_file();
						}
					};
					ui.centered_and_justified(cb);
				};
				egui::panel::CentralPanel::default().show(ctx, cb);
			},
		}
	}
	
	fn draw(&mut self, egui_ctx: &egui::Context) {
		let now = Instant::now();
		let delta_time = now - self.last_frame;
		self.last_frame = now;
		let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor::default());
		let frame = self.surface.get_current_texture().expect("get current texture");
		let view = frame.texture.create_view(&TextureViewDescriptor::default());
		if let Some(loaded_level) = &mut self.loaded_level {
			loaded_level.render(&self.queue, &self.rr, &mut encoder, &view, delta_time);
		}
		let egui_input = self.egui_input_state.take_egui_input(&self.window);
		let egui_output = egui_ctx.run(egui_input, |ctx| self.egui(ctx));
		self.egui_input_state.handle_platform_output(&self.window, egui_output.platform_output);
		for (id, delta) in &egui_output.textures_delta.set {
			self.egui_renderer.update_texture(&self.device, &self.queue, *id, delta);
		}
		let egui_tris = egui_ctx.tessellate(egui_output.shapes, egui_output.pixels_per_point);
		let screen_desc = egui_wgpu::ScreenDescriptor {
			size_in_pixels: self.window_size.into(),
			pixels_per_point: egui_output.pixels_per_point,
		};
		self.egui_renderer.update_buffers(
			&self.device,
			&self.queue,
			&mut encoder,
			&egui_tris,
			&screen_desc,
		);
		let mut egui_pass = gfx::egui_render_pass(&mut encoder, &view).forget_lifetime();
		self.egui_renderer.render(&mut egui_pass, &egui_tris, &screen_desc);
		drop(egui_pass);
		for id in &egui_output.textures_delta.free {
			self.egui_renderer.free_texture(id);
		}
		self.queue.submit(iter::once(encoder.finish()));
		frame.present();
		self.window.request_redraw();
	}
	
	pub fn try_draw(&mut self, egui_ctx: &egui::Context) {
		if self.draw {
			self.draw(egui_ctx);
		}
	}
	
	pub fn key(
		&mut self,
		event_loop: &ActiveEventLoop,
		key_code: KeyCode,
		state: ElementState,
		repeat: bool,
	) {
		match (self.modifiers, state, key_code, repeat) {
			(_, ElementState::Pressed, KeyCode::Escape, _) => {
				if self.file_dialog.is_closed() {
					event_loop.exit();
				}
			},
			(ModifiersState::CONTROL, ElementState::Pressed, KeyCode::KeyO, false) => {
				self.file_dialog.pick_level_file();
				if let Some(loaded_level) = &mut self.loaded_level {
					loaded_level.set_mouse_control(&self.window, false);
				}
			}
			_ => {},
		}
		if let Some(loaded_level) = &mut self.loaded_level {
			loaded_level.key(key_code, state);
		}
	}
	
	pub fn mouse_button(&mut self, state: ElementState, button: MouseButton) {
		if let Some(loaded_level) = &mut self.loaded_level {
			loaded_level.mouse_button(&self.window, &self.file_dialog, state, button);
		}
	}
	
	pub fn cursor_moved(&mut self, pos: PhysicalPosition<f64>) {
		if let Some(loaded_level) = &mut self.loaded_level {
			loaded_level.cursor_moved(&self.window, pos);
		}
	}
	
	pub fn mouse_motion(&mut self, x: f32, y: f32) {
		if let Some(loaded_level) = &mut self.loaded_level {
			loaded_level.mouse_motion(x, y);
		}
	}
}
