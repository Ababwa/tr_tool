mod file_dialog;
mod keys;
mod loaded_level;

use std::{
	fs::File, io::{self, BufReader, Read, Seek}, iter, num::NonZero, path::{Path, PathBuf}, sync::{mpsc::{self, Receiver, TryRecvError}, Arc}, thread, time::{Duration, Instant}
};
use wgpu::{
	CommandEncoderDescriptor, Device, DeviceDescriptor, Features, Instance, Limits, MemoryHints,
	PowerPreference, Queue, RequestAdapterOptions, Surface, SurfaceConfiguration, TextureViewDescriptor,
	Trace,
};
use winit::{
	dpi::{PhysicalPosition, PhysicalSize}, event::{DeviceEvent, ElementState, KeyEvent, MouseButton, WindowEvent},
	event_loop::ActiveEventLoop, keyboard::{KeyCode, PhysicalKey}, window::{Icon, Window, WindowAttributes},
};
use crate::{
	gfx, level_parse::LevelData, print_error::{PrintDebug, PrintError}, render_resources::RenderResources, tr_traits::Version, wait::Wait, GEOM_BUFFER_SIZE, WINDOW_ICON_BYTES
};
use file_dialog::FileDialog;
use keys::KeyStates;
use loaded_level::LoadedLevel;

macro_rules! key_event {
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
	file_dialog: FileDialog,
	version_prompt: Option<PathBuf>,
	error: Option<String>,
	key_states: KeyStates,
	loaded_level: Option<LoadedLevel>,
}

type Ui<'a> = &'a mut egui::Ui;

const WINDOW_TITLE: &str = "TR Tool";
const CONTROL_KEYS: [KeyCode; 2] = [KeyCode::ControlLeft, KeyCode::ControlRight];

#[cfg(target_os = "windows")]
mod ww {
	use wgpu::rwh::{DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle};
	use winit::{platform::windows::WindowExtWindows, window::Window};
	#[derive(Clone, Copy)]
	pub struct WindowWrapper<'a>(pub &'a Window);
	impl<'a> HasWindowHandle for WindowWrapper<'a> {
		fn window_handle(&self) -> Result<WindowHandle<'a>, HandleError> {
			//Safety: Safe to draw to window from another thread on Windows.
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

fn paint_setup(window: Arc<Window>, window_size: PhysicalSize<u32>, rx: Receiver<()>) {
	if let (Some(w), Some(h)) = (NonZero::new(window_size.width), NonZero::new(window_size.height)) {
		let window = &*window;
		#[cfg(target_os = "windows")]
		let window = ww::WindowWrapper(window);
		let sb_ctx = softbuffer::Context::new(window).expect("sb surface");
		let mut sb_surface = softbuffer::Surface::new(&sb_ctx, window).expect("sb surface");
		sb_surface.resize(w, h).expect("sb resize");
		let mut t = 0;
		loop {
			match rx.try_recv() {
				Ok(()) => break,
				Err(TryRecvError::Empty) => {},
				Err(e) => panic!("setup painter polling: {}", e),
			}
			let mut buffer = sb_surface.buffer_mut().expect("sb buffer_mut");
			for i in 0..buffer.len() as u32 {
				let pixel = (((i % w) + (i / w) + 100000000 - t) % 46 / 23) * 0x111111 + 0x222222;
				buffer[i as usize] = pixel;
			}
			buffer.present().expect("sb present");
			t += 1;
			thread::sleep(Duration::from_millis(10));
		}
	} else {
		rx.recv().expect("setup painter blocking");
	}
}

fn draw_window<F: FnOnce(Ui)>(
	ctx: &egui::Context,
	title: &str,
	resizable: bool,
	contents: F,
) -> bool {
	let mut open = true;
	egui::Window::new(title).resizable(resizable).open(&mut open).show(ctx, contents);
	open
}

fn file_name(path: &Path) -> Option<&str> {
	path.file_name()?.to_str()
}

fn get_version(path: &Path, version_num: u32) -> Option<Version> {
	const EXT_LEN: usize = 3;
	let extension = path.extension()?;
	let extension = extension.as_encoded_bytes();
	if extension.len() != EXT_LEN {
		return None;
	}
	let mut ext_lower = [0u8; EXT_LEN];
	ext_lower.copy_from_slice(extension);
	ext_lower.make_ascii_lowercase();
	let version = match (version_num, &ext_lower) {
		(0x00000020, b"phd") => Version::Tr1,
		(0x0000002D, b"tr2") => Version::Tr2,
		(0xFF180038 | 0xFF080038 | 0xFF180034, b"tr2") => Version::Tr3,//most, title.tr2, vict.tr2
		(0x00345254, b"tr4") => Version::Tr4,
		(0x00345254, b"trc") => Version::Tr5,
		_ => return None,
	};
	Some(version)
}

// {
	// let file = File::open(path)?;
	// let mut reader = BufReader::new(file);
	// let mut version_bytes = [0; 4];
	// reader.read_exact(&mut version_bytes)?;
	// let version_num = u32::from_le_bytes(version_bytes);
	// let Some(version) = get_version(path, version_num) else {
	// 	return Ok(None);
	// };
	// reader.rewind()?;
	// let level_data = read_level_as(device, queue, rr, &mut reader, version)?;
	// Ok(Some(level_data))
// }

fn version_button(ui: Ui, version_dest: &mut Option<Version>, text: &str, version: Version) {
	if ui.button(text).clicked() {
		*version_dest = Some(version);
	}
}

impl Core {
	pub fn new(event_loop: &ActiveEventLoop) -> Self {
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
		let window_size = window.inner_size();
		let (setup_painter_tx, rx) = mpsc::channel();
		let painter_window = window.clone();
		let setup_painter = thread::spawn(move || paint_setup(painter_window, window_size, rx));//something to look at while wgpu gets setup
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
		let egui_ctx = egui::Context::default();
		let viewport_id = egui_ctx.viewport_id();
		let egui_input_state = egui_winit::State::new(egui_ctx, viewport_id, &window, None, None, None);
		let egui_renderer = egui_wgpu::Renderer::new(&device, gfx::TEXTURE_FORMAT, None, 1, false);//don't know what `dithering` does here
		let rr = RenderResources::new(&device);
		let file_dialog = FileDialog::new();
		let last_frame = Instant::now();
		setup_painter_tx.send(()).print_err("stop setup painter");
		setup_painter.join().print_err_dbg("join setup painter");
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
			file_dialog,
			version_prompt: None,
			error: None,
			key_states: KeyStates::new(),
			loaded_level: None,
		}
	}
	
	// fn load_level(&mut self, path: &Path, version: Version) -> () {
	// 	let file = File::open(path)?;
	// 	let mut reader = BufReader::new(file);
	// 	let mut version_bytes = [0; 4];
	// 	reader.read_exact(&mut version_bytes)?;
	// 	let version_num = u32::from_le_bytes(version_bytes);
	// 	let Some(version) = get_version(path, version_num) else {
	// 		return Ok(None);
	// 	};
	// 	reader.rewind()?;
	// 	match LevelData::load(&self.device, &self.queue, &self.rr, &path) {
	// 		Ok(Some(level_data)) => {
	// 			let loaded_level = LoadedLevel::new(
	// 				self.window_size,
	// 				&self.device,
	// 				&self.queue,
	// 				&self.rr,
	// 				level_data,
	// 			);
	// 			self.loaded_level = Some(loaded_level);
				// let title = match file_name(&path) {
				// 	Some(name) => &format!("{} - {}", WINDOW_TITLE, name),
				// 	None => WINDOW_TITLE,
				// };
				// self.window.set_title(title);
	// 		},
	// 		Ok(None) => self.version_prompt = Some(path),
	// 		Err(e) => self.error = Some(e.to_string()),
	// 	}
	// }
	
	fn read_level(&mut self, path: &Path, reader: &mut BufReader<File>, v: Version) -> io::Result<()> {
		let level_data = LevelData::load(&self.device, &self.queue, &self.rr, reader, v)?;
		let level = LoadedLevel::new(self.window_size, &self.device, &self.queue, &self.rr, level_data);
		self.loaded_level = Some(level);
		let title = match file_name(&path) {
			Some(name) => &format!("{} - {}", WINDOW_TITLE, name),
			None => WINDOW_TITLE,
		};
		self.window.set_title(title);
		Ok(())
	}
	
	fn load_level_as(&mut self, path: &Path, version: Version) -> io::Result<()> {
		let file = File::open(path)?;
		let mut reader = BufReader::new(file);
		self.read_level(path, &mut reader, version)?;
		Ok(())
	}
	
	fn load_level_detect_version(&mut self, path: PathBuf) -> io::Result<()> {
		let file = File::open(&path)?;
		let mut reader = BufReader::new(file);
		let mut version_bytes = [0; 4];
		reader.read_exact(&mut version_bytes)?;
		let version_num = u32::from_le_bytes(version_bytes);
		let Some(version) = get_version(&path, version_num) else {
			self.version_prompt = Some(path);
			return Ok(());
		};
		reader.rewind()?;
		self.read_level(&path, &mut reader, version)
	}
	
	pub fn try_load(&mut self, path: PathBuf) {
		if let Err(e) = self.load_level_detect_version(path) {
			self.error = Some(e.to_string());
		}
	}
	
	fn resize(&mut self, window_size: PhysicalSize<u32>) {
		self.draw = window_size.width > 0 && window_size.height > 0;
		if self.draw {
			self.window_size = window_size;
			self.config.width = window_size.width;
			self.config.height = window_size.height;
			self.surface.configure(&self.device, &self.config);
			if let Some(loaded_level) = &mut self.loaded_level {
				let ptb = &self.rr.binding_buffers.perspective_transform_buffer;
				loaded_level.resize(&self.device, &self.queue, ptb, window_size);
			}
		}
	}
	
	fn egui(&mut self, ctx: &egui::Context) {
		self.file_dialog.update(ctx);
		if let Some(path) = self.file_dialog.get_level_path() {
			self.try_load(path);
		}
		if let Some(error) = &self.error {
			if !draw_window(ctx, "Error", false, |ui| _ = ui.label(error)) {
				self.error = None;
			}
		}
		if let Some(path) = &self.version_prompt {
			let msg = format!("Could not determine the TR version of\n{}.", path.to_string_lossy());
			let mut version = None;
			let version_prompt = |ui: Ui| {
				ui.label(msg);
				let buttons = |ui: Ui| {
					version_button(ui, &mut version, "TR1", Version::Tr1);
					version_button(ui, &mut version, "TR2", Version::Tr2);
					version_button(ui, &mut version, "TR3", Version::Tr3);
					version_button(ui, &mut version, "TR4", Version::Tr4);
					version_button(ui, &mut version, "TR5", Version::Tr5);
				};
				ui.horizontal(buttons);
			};
			let open = draw_window(ctx, "Select TR Version", false, version_prompt);
			if let Some(version) = version {
				let path = self.version_prompt.take().unwrap();//annoying
				if let Err(e) = self.load_level_as(&path, version) {
					self.error = Some(e.to_string());
				}
			} else if !open {
				self.version_prompt = None;
			}
		}
		match &mut self.loaded_level {
 			Some(loaded_level) => {
				loaded_level.egui(&self.queue, &self.rr, ctx, &mut self.file_dialog, &mut self.error);
			},
			None => {
				let open_button = |ui: Ui| {
					if ui.label("Ctrl+O or click to open file").clicked() {
						self.file_dialog.pick_level_file();
					}
				};
				let centered_open_button = |ui: Ui| {
					ui.centered_and_justified(open_button);
				};
				egui::panel::CentralPanel::default().show(ctx, centered_open_button);
			},
		}
	}
	
	fn draw(&mut self) {
		let now = Instant::now();
		let delta_time = (now - self.last_frame).as_secs_f32();
		self.last_frame = now;
		let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor::default());
		let frame = self.surface.get_current_texture().expect("get current texture");
		let view = frame.texture.create_view(&TextureViewDescriptor::default());
		if let Some(loaded_level) = &mut self.loaded_level {
			loaded_level.render(
				&self.queue,
				&self.rr,
				&self.key_states,
				&mut encoder,
				&view,
				delta_time,
				!self.file_dialog.is_open(),
			);
		}
		let egui_input = self.egui_input_state.take_egui_input(&self.window);
		let egui_ctx = self.egui_input_state.egui_ctx().clone();
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
	
	fn try_draw(&mut self) {
		if self.draw {
			self.draw();
		}
	}
	
	fn close(&mut self, event_loop: &ActiveEventLoop) {
		event_loop.exit();
	}
	
	fn key(&mut self, el: &ActiveEventLoop, state: ElementState, key: KeyCode, repeat: bool) -> bool {
		if repeat {
			return false;
		}
		if let Some(loaded_level) = &mut self.loaded_level {
			if loaded_level.key(key, state) {
				return true;
			}
		}
		match (state, key) {
			(ElementState::Pressed, KeyCode::KeyO) => {
				let ctrl_o = self.key_states.any(&CONTROL_KEYS);
				if ctrl_o {
					if let Some(loaded_level) = &mut self.loaded_level {
						loaded_level.set_mouse_control(&self.window, false);
					}
					self.file_dialog.pick_level_file();
				}
				ctrl_o
			},
			(ElementState::Pressed, KeyCode::Escape) => {
				self.close(el);
				true
			},
			_ => false,
		}
	}
	
	fn mouse_button(&mut self, state: ElementState, button: MouseButton) -> bool {
		match &mut self.loaded_level {
			Some(loaded_level) => loaded_level.mouse_button(&self.window, state, button),
			None => false,
		}
	}
	
	fn cursor_moved(&mut self, pos: PhysicalPosition<f64>) -> bool {
		match &mut self.loaded_level {
			Some(loaded_level) => loaded_level.cursor_moved(&self.window, pos),
			None => false,
		}
	}
	
	pub fn window_event(&mut self, event_loop: &ActiveEventLoop, event: &WindowEvent) {
		let mut delegated_event = false;
		match event {
			&WindowEvent::Resized(window_size) => self.resize(window_size),
			WindowEvent::RedrawRequested => self.try_draw(),
			WindowEvent::CloseRequested => self.close(event_loop),
			&key_event!(key, state, _) => {
				self.key_states.set(key, matches!(state, ElementState::Pressed));
				delegated_event = true;
			},
			_ => delegated_event = true,
		}
		let consumed = if delegated_event && !self.file_dialog.is_open() {
			match event {
				&key_event!(key, state, repeat) => self.key(event_loop, state, key, repeat),
				&WindowEvent::MouseInput { state, button, .. } => self.mouse_button(state, button),
				&WindowEvent::CursorMoved { position, .. } => self.cursor_moved(position),
				_ => false,
			}
		} else {
			false
		};
		if !consumed {
			_ = self.egui_input_state.on_window_event(&self.window, event);
		}
	}
	
	pub fn device_event(&mut self, event: &DeviceEvent) {
		match event {
			&DeviceEvent::MouseMotion { delta: (x, y) } => {
				if !self.file_dialog.is_open() {
					if let Some(loaded_level) = &mut self.loaded_level {
						loaded_level.mouse_motion(x as f32, y as f32);
					}
				}
			},
			_ => {},
		}
	}
}
