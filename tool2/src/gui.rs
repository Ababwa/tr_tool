use glam::{dvec2, DVec2, UVec2};
use pollster::block_on;
use std::{
	future::Future, num::NonZeroU32, sync::{mpsc::{channel, TryRecvError}, Arc}, thread::{sleep, spawn},
	time::{Duration, Instant},
};
use wgpu::{
	CommandEncoder, CommandEncoderDescriptor, Device, DeviceDescriptor, Features, Instance, Limits, LoadOp,
	Operations, PowerPreference, Queue, RenderPassColorAttachment, RenderPassDescriptor,
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

trait Wait: Future {
	fn wait(self) -> Self::Output;
}

impl<T: Future> Wait for T {
	fn wait(self) -> Self::Output {
		block_on(self)
	}
}

trait ToVec {
	type Output;
	fn to_vec(self) -> Self::Output;
}

macro_rules! impl_to_vec {
	($type:ty, $output:ty, $x:ident, $y:ident) => {
		impl ToVec for $type {
			type Output = $output;
			fn to_vec(self) -> Self::Output {
				Self::Output::new(self.$x, self.$y)
			}
		}
	};
}

impl_to_vec!(PhysicalSize<u32>, UVec2, width, height);
impl_to_vec!(PhysicalPosition<f64>, DVec2, x, y);

fn sb_surface(window: &Window, size: UVec2) -> softbuffer::Surface<&Window, &Window> {
	let mut surface = softbuffer::Surface::new(
		&softbuffer::Context::new(window).expect("sb context"), window,
	).expect("sb surface");
	surface.resize(
		NonZeroU32::new(size.x).expect("nonzero window width"),
		NonZeroU32::new(size.y).expect("nonzero window height"),
	).expect("sb resize");
	surface
}

pub trait Gui {
	fn resize(&mut self, window_size: UVec2, device: &Device, queue: &Queue);
	fn modifiers(&mut self, modifers: ModifiersState);
	fn mouse_button(&mut self, window: &Window, button: MouseButton, state: ElementState);
	fn mouse_moved(&mut self, delta: DVec2);
	fn mouse_wheel(&mut self, delta: MouseScrollDelta);
	fn gui(&mut self, device: &Device, queue: &Queue, ctx: &egui::Context);
	fn key(
		&mut self, window: &Window, target: &EventLoopWindowTarget<()>, key_code: KeyCode,
		state: ElementState, repeat: bool,
	);
	fn render(
		&mut self, queue: &Queue, encoder: &mut CommandEncoder, view: &TextureView, delta_time: Duration,
		last_render_time: Duration,
	);
}

pub fn run<T, G, F>(title: T, window_icon: Icon, taskbar_icon: Icon, make_gui: F)
where T: Into<String>, G: Gui, F: FnOnce(&Device, UVec2) -> G {
	env_logger::init();
	let event_loop = EventLoop::new().expect("new event loop");
	let window = WindowBuilder::new()
		.with_title(title)
		.with_min_inner_size(PhysicalSize::new(1, 1))
		.with_window_icon(Some(window_icon))
		.with_taskbar_icon(Some(taskbar_icon))
		.build(&event_loop)
		.expect("build window");
	let mut window_size = window.inner_size().to_vec();
	let window = Arc::new(window);
	let painter_window = window.clone();
	let (tx, rx) = channel();
	let painter = spawn(move || {
		let mut surface = sb_surface(&painter_window, window_size);
		let w = window_size.x;
		let mut t = 0;
		while let Err(TryRecvError::Empty) = rx.try_recv() {
			let mut buffer = surface.buffer_mut().expect("sb buffer_mut loop");
			for i in 0..buffer.len() as u32 {
				buffer[i as usize] = (((i % w) + (i / w) + 100000000 - t) % 46 / 23) * 0x111111 + 0x222222;
			}
			buffer.present().expect("sb present loop");
			t += 1;
			sleep(Duration::from_millis(10));
		}
	});//something to look at during setup
	let instance = Instance::default();
	let surface = instance.create_surface(&window).expect("create surface");//2000ms
	let adapter = instance
		.request_adapter(&RequestAdapterOptions {
			power_preference: PowerPreference::HighPerformance,
			force_fallback_adapter: false,
			compatible_surface: Some(&surface),
		})
		.wait()
		.expect("request adapter");//430ms
	let (device, queue) = adapter
		.request_device(
			&DeviceDescriptor {
				label: None,
				required_features: Features::empty(),
				required_limits: Limits::downlevel_defaults().using_resolution(adapter.limits()),
			},
			None,
		)
		.wait()
		.expect("request device");//250ms
	let mut config = surface
		.get_default_config(&adapter, window_size.x, window_size.y)
		.expect("get default config");
	config.format = TextureFormat::Bgra8Unorm;
	surface.configure(&device, &config);//250ms
	let egui_ctx = egui::Context::default();
	let mut egui_input_state = egui_winit::State::new(
		egui_ctx.clone(), egui_ctx.viewport_id(), &window, None, None,
	);
	let mut egui_renderer = egui_wgpu::Renderer::new(&device, TextureFormat::Bgra8Unorm, None, 1);
	let mut gui = make_gui(&device, window_size);
	tx.send(()).expect("signal painter");
	painter.join().expect("join painter");
	let mut last_frame = Instant::now();
	let mut last_render_time = Duration::ZERO;
	event_loop.run(|event, target| match event {
		Event::DeviceEvent { event: DeviceEvent::MouseMotion { delta: (x, y) }, .. } => {
			gui.mouse_moved(dvec2(x, y));
		},
		Event::WindowEvent { event, .. } => if !egui_input_state.on_window_event(&window, &event).consumed {
			match event {
				WindowEvent::CloseRequested => target.exit(),
				WindowEvent::ModifiersChanged(modifiers) => gui.modifiers(modifiers.state()),
				WindowEvent::MouseInput { button, state, .. } => gui.mouse_button(&window, button, state),
				WindowEvent::MouseWheel { delta, .. } => gui.mouse_wheel(delta),
				WindowEvent::KeyboardInput {
					event: KeyEvent { repeat, physical_key: PhysicalKey::Code(key_code), state, .. },
					..
				} => gui.key(&window, target, key_code, state, repeat),
				WindowEvent::Resized(new_size) => {
					window_size = new_size.to_vec();
					config.width = window_size.x;
					config.height = window_size.y;
					surface.configure(&device, &config);
					gui.resize(window_size, &device, &queue);
				},
				WindowEvent::RedrawRequested => {
					let start = Instant::now();
					let delta_time = start - last_frame;
					let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());
					let frame = surface.get_current_texture().expect("get current texture");
					let view = &frame.texture.create_view(&TextureViewDescriptor::default());
					
					gui.render(&queue, &mut encoder, view, delta_time, last_render_time);
					
					let egui_input = egui_input_state.take_egui_input(&window);
					let egui::FullOutput {
						platform_output,
						textures_delta: egui::TexturesDelta { set, free },
						shapes,
						pixels_per_point,
						..
					} = egui_ctx.run(egui_input, |ctx| gui.gui(&device, &queue, ctx));
					let screen_desc = egui_wgpu::ScreenDescriptor {
						size_in_pixels: window_size.into(),
						pixels_per_point,
					};
					egui_input_state.handle_platform_output(&window, platform_output);
					for (id, delta) in &set {
						egui_renderer.update_texture(&device, &queue, *id, delta);
					}
					let egui_tris = egui_ctx.tessellate(shapes, pixels_per_point);
					egui_renderer.update_buffers(&device, &queue, &mut encoder, &egui_tris, &screen_desc);
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
		},
		_ => {},
	}).expect("run event loop");
}
