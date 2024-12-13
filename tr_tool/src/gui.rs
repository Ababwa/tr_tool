use glam::DVec2;
use pollster::block_on;
use std::{
	future::Future, num::NonZeroU32, sync::{mpsc::{channel, TryRecvError}, Arc}, thread::{sleep, spawn},
	time::{Duration, Instant},
};
use wgpu::{
	CommandEncoder, CommandEncoderDescriptor, Device, DeviceDescriptor, Features, Instance, Limits,
	LoadOp, Operations, PowerPreference, Queue, RenderPassColorAttachment, RenderPassDescriptor,
	RequestAdapterOptions, StoreOp, TextureFormat, TextureView, TextureViewDescriptor,
};
use winit::{
	dpi::{PhysicalPosition, PhysicalSize},
	event::{DeviceEvent, ElementState, Event, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent},
	event_loop::{EventLoop, EventLoopWindowTarget},
	keyboard::{KeyCode, ModifiersState, PhysicalKey},
	platform::windows::WindowBuilderExtWindows,
	window::{Icon, Window, WindowBuilder},
};
use crate::geom_buffer::GEOM_BUFFER_SIZE;

const TEXTURE_FORMAT: TextureFormat = TextureFormat::Bgra8Unorm;

trait Wait: Future {
	fn wait(self) -> Self::Output;
}

impl<T: Future> Wait for T {
	fn wait(self) -> Self::Output {
		block_on(self)
	}
}

fn sb_surface(window: &Window, size: PhysicalSize<u32>) -> softbuffer::Surface<&Window, &Window> {
	let mut surface = softbuffer::Surface::new(
		&softbuffer::Context::new(window).expect("sb context"), window,
	).expect("sb surface");
	surface.resize(
		NonZeroU32::new(size.width).expect("nonzero window width"),
		NonZeroU32::new(size.width).expect("nonzero window height"),
	).expect("sb resize");
	surface
}

pub trait Gui {
	fn resize(&mut self, window_size: PhysicalSize<u32>);
	fn modifiers(&mut self, modifers: ModifiersState);
	fn mouse_button(&mut self, button: MouseButton, state: ElementState);
	fn mouse_motion(&mut self, delta: DVec2);
	fn mouse_wheel(&mut self, delta: MouseScrollDelta);
	fn cursor_moved(&mut self, pos: PhysicalPosition<f64>);
	fn gui(&mut self, ctx: &egui::Context);
	fn key(
		&mut self, target: &EventLoopWindowTarget<()>, key_code: KeyCode, state: ElementState, repeat: bool,
	);
	fn render(
		&mut self, encoder: &mut CommandEncoder, view: &TextureView, delta_time: Duration,
		last_render_time: Duration,
	);
}

pub fn run<G, F>(title: &str, window_icon: Icon, taskbar_icon: Icon, make_gui: F)
where G: Gui, F: FnOnce(Arc<Window>, Arc<Device>, Arc<Queue>, PhysicalSize<u32>) -> G,
{
	env_logger::init();
	let event_loop = EventLoop::new().expect("new event loop");
	let window = WindowBuilder::new()
		.with_title(title)
		.with_min_inner_size(PhysicalSize::new(1, 1))
		.with_window_icon(Some(window_icon))
		.with_taskbar_icon(Some(taskbar_icon))
		.build(&event_loop)
		.expect("build window");
	let window = Arc::new(window);
	let mut window_size = window.inner_size();
	let painter_window = window.clone();
	let (tx, rx) = channel();
	let painter = spawn(move || {
		let mut surface = sb_surface(&painter_window, window_size);
		let w = window_size.width;
		let mut t = 0;
		while let Err(TryRecvError::Empty) = rx.try_recv() {
			let mut buffer = surface.buffer_mut().expect("sb buffer_mut");
			for i in 0..buffer.len() as u32 {
				let pixel = (((i % w) + (i / w) + 100000000 - t) % 46 / 23) * 0x111111 + 0x222222;
				buffer[i as usize] = pixel;
			}
			buffer.present().expect("sb present");
			t += 1;
			sleep(Duration::from_millis(10));
		}
	});//something to look at during setup
	let instance = Instance::default();
	let surface = instance.create_surface(&window).expect("create surface");//2000ms
	let adapter = instance
		.request_adapter(
			&RequestAdapterOptions {
				power_preference: PowerPreference::HighPerformance,
				force_fallback_adapter: false,
				compatible_surface: Some(&surface),
			},
		)
		.wait()
		.expect("request adapter");//430ms
	let mut required_limits = Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits());
	required_limits.max_storage_buffers_per_shader_stage = 1;
	required_limits.max_storage_buffer_binding_size = GEOM_BUFFER_SIZE as u32;
	required_limits.max_texture_array_layers = 512;
	let (device, queue) = adapter
		.request_device(
			&DeviceDescriptor { label: None, required_features: Features::empty(), required_limits },
			None,
		)
		.wait()
		.expect("request device");//250ms
	let device = Arc::new(device);
	let queue = Arc::new(queue);
	let mut config = surface
		.get_default_config(&adapter, window_size.width, window_size.height)
		.expect("get default config");
	config.format = TEXTURE_FORMAT;
	surface.configure(&device, &config);//250ms
	let egui_ctx = egui::Context::default();
	let mut egui_input_state = egui_winit::State::new(
		egui_ctx.clone(), egui_ctx.viewport_id(), &window, None, None,
	);
	let mut egui_renderer = egui_wgpu::Renderer::new(&device, TEXTURE_FORMAT, None, 1);
	let mut gui = make_gui(window.clone(), device.clone(), queue.clone(), window_size);
	tx.send(()).expect("signal painter");
	painter.join().expect("join painter");
	let mut last_frame = Instant::now();
	let mut last_render_time = Duration::ZERO;
	let mut draw = true;
	event_loop.run(|event, target| match event {
		Event::DeviceEvent { event: DeviceEvent::MouseMotion { delta: (x, y) }, .. } => {
			gui.mouse_motion(DVec2 { x, y });
		},
		Event::WindowEvent { event, .. } => {
			if !egui_input_state.on_window_event(&window, &event).consumed {
				match event {
					WindowEvent::CloseRequested => target.exit(),
					WindowEvent::ModifiersChanged(modifiers) => gui.modifiers(modifiers.state()),
					WindowEvent::MouseInput { button, state, .. } => gui.mouse_button(button, state),
					WindowEvent::MouseWheel { delta, .. } => gui.mouse_wheel(delta),
					WindowEvent::CursorMoved { position, .. } => gui.cursor_moved(position),
					WindowEvent::KeyboardInput {
						event: KeyEvent { repeat, physical_key: PhysicalKey::Code(key_code), state, .. },
						..
					} => gui.key(target, key_code, state, repeat),
					WindowEvent::Resized(new_size) => {
						if new_size.width * new_size.height != 0 {
							window_size = new_size;
							config.width = window_size.width;
							config.height = window_size.height;
							surface.configure(&device, &config);
							gui.resize(window_size);
							draw = true;
						} else {
							draw = false
						}
					},
					WindowEvent::RedrawRequested => if draw {
						let start = Instant::now();
						let delta_time = start - last_frame;
						let mut encoder = device
							.create_command_encoder(&CommandEncoderDescriptor::default());
						let frame = surface.get_current_texture().expect("get current texture");
						let view = &frame.texture.create_view(&TextureViewDescriptor::default());
						
						gui.render(&mut encoder, view, delta_time, last_render_time);
						
						let egui_input = egui_input_state.take_egui_input(&window);
						let egui::FullOutput {
							platform_output,
							textures_delta: egui::TexturesDelta { set, free },
							shapes,
							pixels_per_point,
							..
						} = egui_ctx.run(egui_input, |ctx| gui.gui(ctx));
						let screen_desc = egui_wgpu::ScreenDescriptor {
							size_in_pixels: window_size.into(),
							pixels_per_point,
						};
						egui_input_state.handle_platform_output(&window, platform_output);
						for (id, delta) in &set {
							egui_renderer.update_texture(&device, &queue, *id, delta);
						}
						let egui_tris = egui_ctx.tessellate(shapes, pixels_per_point);
						egui_renderer
							.update_buffers(&device, &queue, &mut encoder, &egui_tris, &screen_desc);
						let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
							label: None,
							color_attachments: &[
								Some(RenderPassColorAttachment {
									view,
									resolve_target: None,
									ops: Operations { load: LoadOp::Load, store: StoreOp::Store },
								}),
							],
							depth_stencil_attachment: None,
							timestamp_writes: None,
							occlusion_query_set: None,
						});
						egui_renderer.render(&mut rpass, &egui_tris, &screen_desc);
						drop(rpass);
						for id in &free {
							egui_renderer.free_texture(id);
						}
						
						queue.submit([encoder.finish()]);
						frame.present();
						window.request_redraw();
						last_frame = start;
						last_render_time = Instant::now() - start;
					},
					_ => {},
				}
			}
		},
		_ => {},
	}).expect("run event loop");
}
