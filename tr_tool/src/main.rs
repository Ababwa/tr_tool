mod ext;
mod load;
mod make;

use std::{
	collections::{HashMap, HashSet}, env::args, f32::consts::FRAC_PI_2, mem::size_of, num::NonZeroU32, slice,
	time::Instant,
};
use egui_file_dialog::{DialogState, FileDialog, FileDialogConfig};
use glam::{Mat4, UVec2, Vec3};
use wgpu::{
	BindGroup, BindGroupLayout, BlendComponent, BlendFactor, BlendOperation, BlendState, Buffer,
	Color, CommandEncoderDescriptor, Device, DeviceDescriptor, Extent3d, Features,
	Instance, Limits, LoadOp, Operations, PowerPreference, Queue, RenderPassColorAttachment,
	RenderPassDepthStencilAttachment, RenderPassDescriptor, RequestAdapterOptions, StoreOp,
	TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension, VertexFormat,
};
use winit::{
	dpi::PhysicalSize,
	event::{DeviceEvent, ElementState, Event, MouseButton, RawKeyEvent, WindowEvent},
	event_loop::EventLoop,
	keyboard::{KeyCode, PhysicalKey},
	window::{CursorGrabMode, Window},
};
use shared::{MinMax, VecMinMax};
use ext::{AsBytes, Wait};
use load::{
	load_level_render_data, FlipGroup, LevelRenderData, RoomVertexIndices, SolidData, SolidVertex, SpriteVertex, TexturedVertex
};

fn fill_dark(window: &Window) -> Result<(), softbuffer::SoftBufferError> {
	let size = window.inner_size();
	let mut surface = softbuffer::Surface::new(&softbuffer::Context::new(window)?, window)?;
	surface.resize(NonZeroU32::new(size.width).unwrap(), NonZeroU32::new(size.height).unwrap())?;
	let mut buffer = surface.buffer_mut()?;
	buffer.fill(0);
	Ok(buffer.present()?)
}

fn write_transform(queue: &Queue, buffer: &Buffer, transform: &Mat4) {
	queue.write_buffer(buffer, 0, transform.as_bytes());
}

fn get_room_indices<'a>(static_room_indices: &'a [usize], flip_groups: &'a [FlipGroup]) -> impl Iterator<Item = usize> + 'a {
	static_room_indices.iter().copied().chain(flip_groups.iter().flat_map(|flip_group| flip_group.get_room_indices()))
}

fn swap_br(data: &mut [u8]) {
	for pixel in data.chunks_exact_mut(4) {
		pixel.swap(0, 2);
	}
}

struct LevelViewState {
	path: String,
	atlas_size: UVec2,
	atlas_data: Box<[u8]>,
	perspective_buffer: Buffer,
	camera_buffer: Buffer,
	textured_vertex_buffer: Buffer,
	textured_bind_group: BindGroup,
	solid: Option<(Buffer, BindGroup)>,
	sprite_vertex_buffer: Option<Buffer>,
	room_vertex_indices: Vec<RoomVertexIndices>,
	static_room_indices: Vec<usize>,
	flip_groups: Vec<FlipGroup>,
	cam_pos: Vec3,
	yaw: f32,
	pitch: f32,
}

fn load_level(
	path: String,
	device: &Device,
	queue: &Queue,
	window_size: PhysicalSize<u32>,
	textured_layout: &BindGroupLayout,
	solid_layout: &BindGroupLayout,
) -> LevelViewState {
	let LevelRenderData {
		atlas_size,
		atlas_data,
		textured_vertices,
		solid,
		sprite_vertices,
		room_vertex_indices,
		static_room_indices,
		flip_groups,
	} = load_level_render_data(&path).expect("read level file");
	let bounds = MinMax::from_iter(textured_vertices.iter().map(|v| v.pos)).unwrap();
	let cam_pos = bounds.min;
	let (yaw, pitch) = make::yaw_pitch(bounds.max - bounds.min);
	let perspective_transform = make::perspective_transform(window_size);
	let camera_transform = make::camera_transform(cam_pos, yaw, pitch);
	let perspective_buffer = make::uniform_buffer(device, perspective_transform.as_bytes());
	let camera_buffer = make::uniform_buffer(device, camera_transform.as_bytes());
	let textured_vertex_buffer = make::vertex_buffer(device, textured_vertices.as_bytes());
	let textured_bind_group = make::bind_group(
		device,
		queue,
		textured_layout,
		&perspective_buffer,
		&camera_buffer,
		Extent3d { width: atlas_size.x, height: atlas_size.y, depth_or_array_layers: 1 },
		TextureDimension::D2,
		TextureFormat::Bgra8UnormSrgb,
		&atlas_data,
	);
	let solid = solid.map(|SolidData { palette, solid_vertices }| {
		let solid_vertex_buffer = make::vertex_buffer(device, solid_vertices.as_bytes());
		let solid_bind_group = make::bind_group(
			device,
			queue,
			solid_layout,
			&perspective_buffer,
			&camera_buffer,
			Extent3d { width: tr_model::shared::PALETTE_SIZE as u32, height: 1, depth_or_array_layers: 1 },
			TextureDimension::D1,
			TextureFormat::Rgba8UnormSrgb,
			palette.as_slice(),
		);
		(solid_vertex_buffer, solid_bind_group)
	});
	let sprite_vertex_buffer = (!sprite_vertices.is_empty()).then(||
		make::vertex_buffer(device, sprite_vertices.as_bytes())
	);
	let mut atlas_data = atlas_data;
	swap_br(&mut atlas_data);
	LevelViewState {
		path,
		atlas_size,
		atlas_data,
		perspective_buffer,
		camera_buffer,
		textured_vertex_buffer,
		textured_bind_group,
		solid,
		sprite_vertex_buffer,
		room_vertex_indices,
		static_room_indices,
		flip_groups,
		cam_pos,
		yaw,
		pitch,
	}
}

macro_rules! keys_decl {
	($($symbol:ident: [$($keycode:expr),*],)*) => {
		#[derive(Clone, Copy, PartialEq, Eq, Hash)]
		enum TrackedKey { $($symbol),* }
		
		const NUM_TRACKED_KEYS: usize = [$($symbol),*].len();
		
		fn get_key_map() -> HashMap<KeyCode, HashSet<TrackedKey>> {
			let mut keymap = HashMap::<KeyCode, HashSet<TrackedKey>>::new();
			for (keycode, symbol) in [$($(($keycode, $symbol),)*)*] {
				keymap.entry(keycode).or_default().insert(symbol);
			}
			keymap
		}
	};
}

keys_decl!(
	Exit: [KeyCode::Escape, KeyCode::KeyF],
	Forward: [KeyCode::KeyW, KeyCode::ArrowUp],
	Left: [KeyCode::KeyA, KeyCode::ArrowLeft],
	Backward: [KeyCode::KeyS, KeyCode::ArrowDown],
	Right: [KeyCode::KeyD, KeyCode::ArrowRight],
	Up: [KeyCode::KeyQ, KeyCode::PageUp],
	Down: [KeyCode::KeyE, KeyCode::PageDown],
	Shift: [KeyCode::ShiftLeft, KeyCode::ShiftRight],
	Control: [KeyCode::ControlLeft, KeyCode::ControlRight],
	Open: [KeyCode::KeyO],
	SaveTexture: [KeyCode::KeyT],
);

#[derive(Default)]
struct TrackedKeyState {
	down: bool,
	just_pressed: bool,
}

#[derive(Default)]
struct TrackedKeysState([TrackedKeyState; NUM_TRACKED_KEYS]);

impl TrackedKeysState {
	fn down(&self, key: TrackedKey) -> bool {
		self.0[key as u8 as usize].down
	}
	
	fn just_pressed(&self, key: TrackedKey) -> bool {
		self.0[key as u8 as usize].just_pressed
	}
	
	fn press(&mut self, key: TrackedKey) {
		let entry = &mut self.0[key as u8 as usize];
		entry.just_pressed = !entry.down;
		entry.down = true;
	}
	
	fn release(&mut self, key: TrackedKey) {
		let entry = &mut self.0[key as u8 as usize];
		entry.down = false;
		entry.just_pressed = false;
	}
	
	fn clear_just_pressed(&mut self) {
		for entry in &mut self.0 {
			entry.just_pressed = false;
		}
	}
}

use TrackedKey::*;

const MOVEMENT_MAP: [(TrackedKey, Vec3); 6] = [
	(Forward, Vec3::Z),
	(Backward, Vec3::NEG_Z),
	(Left, Vec3::X),
	(Right, Vec3::NEG_X),
	(Down, Vec3::Y),
	(Up, Vec3::NEG_Y),
];

fn set_camera_control_off(window: &Window, camera_control: &mut bool) {
	window.set_cursor_visible(true);
	window.set_cursor_grab(CursorGrabMode::None).unwrap();
	*camera_control = false;
}

fn main() {
	env_logger::init();
	let event_loop = EventLoop::new().expect("new event loop");
	let window = &winit::window::WindowBuilder::new()
		.with_title("TR Tool")
		.with_min_inner_size(PhysicalSize::new(1, 1))
		.build(&event_loop)
		.expect("build window");
	fill_dark(window).expect("fill window");
	
	//state
	let mut window_size = window.inner_size();
	let mut camera_control = false;
	let mut keys = TrackedKeysState::default();
	let key_map = get_key_map();
	let mut level_view_state: Option<LevelViewState> = None;
	
	let instance = Instance::default();
	let surface = instance.create_surface(window).expect("create surface");
	let adapter = instance.request_adapter(&RequestAdapterOptions {
		power_preference: PowerPreference::HighPerformance,
		force_fallback_adapter: false,
		compatible_surface: Some(&surface),
	}).wait().expect("request adapter");
	let (device, queue) = adapter.request_device(&DeviceDescriptor {
		label: None,
		required_features: Features::empty(),
		required_limits: Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
	}, None).wait().expect("request device");
	let mut config = surface.get_default_config(&adapter, window_size.width, window_size.height).expect("get default config");
	surface.configure(&device, &config);
	
	let textured_bind_group_layout = make::bind_group_layout(&device, TextureViewDimension::D2);
	let textured_bind_group_layout_ref = &textured_bind_group_layout;
	let textured_pipeline_layout_descriptor = make::pipeline_layout_descriptor(slice::from_ref(&textured_bind_group_layout_ref));
	
	let textured_shader = make::shader(&device, include_str!("shader/textured.wgsl"));
	let textured_vertex_attributes = make::vertex_attributes(&[VertexFormat::Float32x3, VertexFormat::Float32x2]);
	let textured_vertex_buffer_layout = make::vertex_buffer_layout(size_of::<TexturedVertex>() as u64, &textured_vertex_attributes);
	let textured_vertex_state = make::vertex_state(&textured_shader, slice::from_ref(&textured_vertex_buffer_layout));
	
	let solid_bind_group_layout = make::bind_group_layout(&device, TextureViewDimension::D1);
	let solid_bind_group_layout_ref = &solid_bind_group_layout;
	let solid_pipeline_layout_descriptor = make::pipeline_layout_descriptor(slice::from_ref(&solid_bind_group_layout_ref));
	
	let solid_shader = make::shader(&device, include_str!("shader/solid.wgsl"));
	let solid_vertex_attributes = make::vertex_attributes(&[VertexFormat::Float32x3, VertexFormat::Uint32]);
	let solid_vertex_buffer_layout = make::vertex_buffer_layout(size_of::<SolidVertex>() as u64, &solid_vertex_attributes);
	let solid_vertex_state = make::vertex_state(&solid_shader, slice::from_ref(&solid_vertex_buffer_layout));
	
	let sprite_shader = make::shader(&device, include_str!("shader/sprite.wgsl"));
	let sprite_vertex_attributes = make::vertex_attributes(&[VertexFormat::Float32x3, VertexFormat::Float32x2, VertexFormat::Float32x2]);
	let sprite_vertex_buffer_layout = make::vertex_buffer_layout(size_of::<SpriteVertex>() as u64, &sprite_vertex_attributes);
	let sprite_vertex_state = make::vertex_state(&sprite_shader, slice::from_ref(&sprite_vertex_buffer_layout));
	
	let opaque_pipeline = make::render_pipeline(
		&device,
		&textured_pipeline_layout_descriptor,
		textured_vertex_state.clone(),
		&textured_shader,
		None,
		true,
	);
	let additive_pipeline = make::render_pipeline(
		&device,
		&textured_pipeline_layout_descriptor,
		textured_vertex_state,
		&textured_shader,
		Some(BlendState {
			alpha: BlendComponent {
				src_factor: BlendFactor::One,
				dst_factor: BlendFactor::One,
				operation: BlendOperation::Add,
			},
			color: BlendComponent {
				src_factor: BlendFactor::One,
				dst_factor: BlendFactor::One,
				operation: BlendOperation::Add,
			},
		}),
		false,
	);
	let solid_pipeline = make::render_pipeline(
		&device,
		&solid_pipeline_layout_descriptor,
		solid_vertex_state,
		&solid_shader,
		None,
		true,
	);
	let sprite_pipeline = make::render_pipeline(
		&device,
		&textured_pipeline_layout_descriptor,
		sprite_vertex_state,
		&sprite_shader,
		None,
		true,
	);
	
	let egui_ctx = egui::Context::default();
	let mut egui_input_state = egui_winit::State::new(egui_ctx.clone(), egui_ctx.viewport_id(), window, None, None);
	let mut egui_renderer = egui_wgpu::Renderer::new(&device, TextureFormat::Bgra8UnormSrgb, None, 1);
	let mut file_dialog = FileDialog::with_config(FileDialogConfig {
		initial_directory: std::fs::read_to_string("dir").ok().unwrap_or(".".into()).into(),
		..Default::default()
	});
	let mut choosing_file = false;
	
	let mut depth_view = make::depth_view(&device, window_size);
	
	if let Some(level_path) = args().skip(1).next() {
		level_view_state = Some(load_level(
			level_path,
			&device,
			&queue,
			window_size,
			&textured_bind_group_layout,
			&solid_bind_group_layout,
		));
	}
	
	let mut last_frame = Instant::now();
	event_loop.run(move |event, target| match event {
		Event::WindowEvent { event, .. } => {
			//don't send CursorMoved event to egui if camera is being controlled
			//don't process event if consumed by egui
			let process_event = (camera_control && matches!(event, WindowEvent::CursorMoved { .. }))
				|| !egui_input_state.on_window_event(window, &event).consumed;
			if process_event {
				match event {
					WindowEvent::Resized(new_size) => {
						window_size = new_size;
						config.width = window_size.width;
						config.height = window_size.height;
						surface.configure(&device, &config);
						depth_view = make::depth_view(&device, window_size);
						if let Some(LevelViewState { perspective_buffer, .. }) = &level_view_state {
							write_transform(&queue, perspective_buffer, &make::perspective_transform(window_size));
						}
					},
					WindowEvent::RedrawRequested => {
						let now = Instant::now();
						let delta_time = now - last_frame;
						let frame = surface.get_current_texture().expect("Failed to acquire next swap chain texture");
						let color_view = frame.texture.create_view(&TextureViewDescriptor::default());
						let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());
						if let Some(LevelViewState {
							camera_buffer,
							textured_vertex_buffer,
							textured_bind_group,
							solid,
							sprite_vertex_buffer,
							room_vertex_indices,
							static_room_indices,
							flip_groups,
							cam_pos,
							yaw,
							pitch,
							..
						}) = &mut level_view_state {
							if let Some(movement) = MOVEMENT_MAP.into_iter().filter_map(|(key, vec)| keys.down(key).then_some(vec)).reduce(|a, b| a + b) {
								*cam_pos +=
									if keys.down(Shift) { 5.0 } else { 1.0 } *
									if keys.down(Control) { 0.1 } else { 1.0 } *
									5.0 *
									delta_time.as_secs_f32() *
									Mat4::from_rotation_y(*yaw).inverse().transform_point3(movement);
								write_transform(&queue, camera_buffer, &make::camera_transform(*cam_pos, *yaw, *pitch));
							}
							let room_indices = get_room_indices(static_room_indices, flip_groups).collect::<Vec<_>>();
							let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
								label: None,
								color_attachments: &[Some(RenderPassColorAttachment {
									ops: Operations { load: LoadOp::Clear(Color::BLACK), store: StoreOp::Store },
									resolve_target: None,
									view: &color_view,
								})],
								depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
									depth_ops: Some(Operations { load: LoadOp::Clear(1.0), store: StoreOp::Store }),
									stencil_ops: None,
									view: &depth_view,
								}),
								timestamp_writes: None,
								occlusion_query_set: None,
							});
							rpass.set_bind_group(0, textured_bind_group, &[]);
							rpass.set_vertex_buffer(0, textured_vertex_buffer.slice(..));
							rpass.set_pipeline(&opaque_pipeline);
							for &room_index in &room_indices {
								rpass.draw(room_vertex_indices[room_index].opaque.clone(), 0..1);
							}
							rpass.set_pipeline(&additive_pipeline);
							for &room_index in &room_indices {
								rpass.draw(room_vertex_indices[room_index].additive.clone(), 0..1);
							}
							if let Some(sprite_vertex_buffer) = sprite_vertex_buffer {
								rpass.set_vertex_buffer(0, sprite_vertex_buffer.slice(..));
								rpass.set_pipeline(&sprite_pipeline);
								for &room_index in &room_indices {
									rpass.draw(room_vertex_indices[room_index].sprite.clone(), 0..1);
								}
							}
							if let Some((solid_vertex_buffer, solid_bind_group)) = solid {
								rpass.set_bind_group(0, solid_bind_group, &[]);
								rpass.set_vertex_buffer(0, solid_vertex_buffer.slice(..));
								rpass.set_pipeline(&solid_pipeline);
								for &room_index in &room_indices {
									rpass.draw(room_vertex_indices[room_index].solid.clone(), 0..1);
								}
							}
						}
						let egui_input = egui_input_state.take_egui_input(window);
						let egui::FullOutput {
							platform_output,
							shapes,
							pixels_per_point,
							textures_delta: egui::TexturesDelta { set: tex_set, free: tex_free },
							..
						} = egui_ctx.run(egui_input, |ctx| {
							match &mut level_view_state {
								Some(LevelViewState { flip_groups, .. }) => {
									egui::Window::new("Flip Groups").show(ctx, |ui| {
										if flip_groups.is_empty() {
											ui.label("No flip groups");
										} else {
											for flip_group in flip_groups {
												ui.checkbox(&mut flip_group.flipped, format!("Flip Group {}", flip_group.label));
											}
										}
									});
								},
								None => {
									egui::panel::CentralPanel::default().show(ctx, |ui| {
										ui.centered_and_justified(|ui| {
											ui.label("Ctrl+O to open file");
										});
									});
								},
							}
							file_dialog.update(ctx);
						});
						let screen_descriptor = egui_wgpu::ScreenDescriptor { size_in_pixels: window_size.into(), pixels_per_point };
						egui_input_state.handle_platform_output(window, platform_output);
						for (id, delta) in &tex_set {
							egui_renderer.update_texture(&device, &queue, *id, delta)
						}
						let egui_tris = egui_ctx.tessellate(shapes, pixels_per_point);
						egui_renderer.update_buffers(&device, &queue, &mut encoder, &egui_tris, &screen_descriptor);
						{
							let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
								label: None,
								color_attachments: &[Some(RenderPassColorAttachment {
									ops: Operations { load: LoadOp::Load, store: StoreOp::Store },
									resolve_target: None,
									view: &color_view,
								})],
								depth_stencil_attachment: None,
								timestamp_writes: None,
								occlusion_query_set: None,
							});
							egui_renderer.render(&mut rpass, &egui_tris, &screen_descriptor);
						}
						for id in &tex_free {
							egui_renderer.free_texture(id);
						}
						queue.submit([encoder.finish()]);
						frame.present();
						if choosing_file {
							let file_dialog_state = file_dialog.state();
							choosing_file = matches!(file_dialog_state, DialogState::Open);
							if let DialogState::Selected(path) = file_dialog_state {
								level_view_state = Some(load_level(
									path.into_os_string().into_string().expect("path not UTF-8"),
									&device,
									&queue,
									window_size,
									&textured_bind_group_layout,
									&solid_bind_group_layout,
								));
							}
						}
						keys.clear_just_pressed();
						last_frame = now;
						window.request_redraw();
					},
					WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Right, .. } => if camera_control {
						set_camera_control_off(window, &mut camera_control);
					} else if level_view_state.is_some() {
						window.set_cursor_visible(false);
						window.set_cursor_grab(CursorGrabMode::Confined).or_else(|_| window.set_cursor_grab(CursorGrabMode::Locked)).expect("cursor grab");
						camera_control = true;
					},
					WindowEvent::CloseRequested => target.exit(),
					_ => {},
				}
			}
		},
		Event::DeviceEvent { event: DeviceEvent::MouseMotion { delta: (delta_x, delta_y) }, .. } => if camera_control {
			if let Some(LevelViewState { camera_buffer, cam_pos, yaw, pitch, .. }) = &mut level_view_state {
				*yaw = *yaw + delta_x as f32 / 150.0;
				*pitch = (*pitch + delta_y as f32 / 150.0).clamp(-FRAC_PI_2, FRAC_PI_2);
				write_transform(&queue, camera_buffer, &make::camera_transform(*cam_pos, *yaw, *pitch));
			}
		},
		Event::DeviceEvent { event: DeviceEvent::Key(RawKeyEvent { physical_key: PhysicalKey::Code(keycode), state }), .. } => {
			if let Some(tracked_keys) = key_map.get(&keycode) {
				let key_op = match state {
					ElementState::Pressed => TrackedKeysState::press,
					ElementState::Released => TrackedKeysState::release,
				};
				for &tracked_key in tracked_keys {
					key_op(&mut keys, tracked_key);
				}
			}
			if keys.just_pressed(Exit) {
				target.exit();
			}
			if keys.down(Control) {
				if keys.just_pressed(Open) {
					if camera_control {
						set_camera_control_off(window, &mut camera_control);
					}
					choosing_file = true;
					file_dialog.select_file();
				}
				if keys.just_pressed(SaveTexture) {
					if let Some(LevelViewState { path, atlas_size, atlas_data, .. }) = &mut level_view_state {
						_ = path;
						/*let end = match path.rfind('.') {
							Some(last_period) => last_period,
							None => path.len(),
						};*/
						let texture_path = "texture.png";//format!("{}_texture.png", &path[..end]);
						image::save_buffer(texture_path, atlas_data, atlas_size.x, atlas_size.y, image::ColorType::Rgba8).expect("save texture");
					}
				}
			}
		},
		_ => {},
	}).expect("run event loop");
}
