mod ext;
mod load;
mod make;

use std::{
	collections::HashMap, env::args, f32::consts::FRAC_PI_2, mem::size_of, num::NonZeroU32, slice,
	time::Instant,
};
use egui_file_dialog::{DialogState, FileDialog, FileDialogConfig};
use glam::{Mat4, Vec3};
use wgpu::{
	util::{BufferInitDescriptor, DeviceExt},
	BindGroup, BindGroupLayout, BlendComponent, BlendFactor, BlendOperation, BlendState, Buffer,
	BufferUsages, Color, CommandEncoderDescriptor, Device, DeviceDescriptor, Extent3d, Features,
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
use shared::{
	geom::{MinMax, VecMinMax},
	reinterpret,
};
use ext::Wait;
use load::{
	load_level_render_data, ColoredData, ColoredVertex, FlipGroup, LevelRenderData, RoomVertexIndices, TexturedVertex
};

fn fill_dark(window: &Window) -> Result<(), softbuffer::SoftBufferError> {
	let size = window.inner_size();
	let mut surface = softbuffer::Surface::new(&softbuffer::Context::new(window)?, window)?;
	surface.resize(NonZeroU32::new(size.width).unwrap(), NonZeroU32::new(size.height).unwrap())?;
	let mut buffer = surface.buffer_mut()?;
	buffer.fill(0);
	Ok(buffer.present()?)
}

fn write_look_matrix(queue: &Queue, look_matrix_uniform: &Buffer, look_matrix: &Mat4) {
	queue.write_buffer(look_matrix_uniform, 0, unsafe { reinterpret::ref_to_slice(look_matrix) });//floats to bytes
}

fn get_room_indices<'a>(static_room_indices: &'a [usize], flip_groups: &'a [FlipGroup]) -> impl Iterator<Item = usize> + 'a {
	static_room_indices.iter().copied().chain(flip_groups.iter().flat_map(|flip_group| flip_group.get_room_indices()))
}

struct LevelViewState {
	look_matrix_uniform: Buffer,
	textured_vertex_buffer: Buffer,
	textured_bind_group: BindGroup,
	colored: Option<(Buffer, BindGroup)>,
	room_vertex_indices: Vec<RoomVertexIndices>,
	static_room_indices: Vec<usize>,
	flip_groups: Vec<FlipGroup>,
	cam_pos: Vec3,
	yaw: f32,
	pitch: f32,
}

fn load_level(
	path: &str,
	device: &Device,
	queue: &Queue,
	window_size: PhysicalSize<u32>,
	textured_layout: &BindGroupLayout,
	colored_layout: &BindGroupLayout,
) -> LevelViewState {
	let LevelRenderData {
		atlas_size,
		atlas_data,
		textured_vertices,
		colored,
		room_vertex_indices,
		static_room_indices,
		flip_groups,
	} = load_level_render_data(&path).expect("read level file");
	let bounds = MinMax::from_iter(textured_vertices.iter().map(|v| v.pos)).unwrap();
	let cam_pos = bounds.min;
	let (yaw, pitch) = make::yaw_pitch(bounds.max - bounds.min);
	let look_matrix_uniform = device.create_buffer_init(&BufferInitDescriptor {
		label: None,
		usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
		contents: unsafe { reinterpret::ref_to_slice(&make::look_matrix(window_size, cam_pos, yaw, pitch)) },//floats to bytes
	});
	let textured_vertex_buf = make::vertex_buffer(device, unsafe { reinterpret::slice(&textured_vertices) });//contiguous primitives to bytes
	let textured_bind_group = make::bind_group(
		device,
		queue,
		textured_layout,
		&look_matrix_uniform,
		Extent3d { width: atlas_size.x, height: atlas_size.y, depth_or_array_layers: 1 },
		TextureDimension::D2,
		TextureFormat::Bgra8UnormSrgb,
		&atlas_data,
	);
	let colored = colored.map(|ColoredData { palette, colored_vertices }| {
		let colored_vertex_buf = make::vertex_buffer(device, unsafe { reinterpret::slice(&colored_vertices) });//contiguous primitives to bytes
		let colored_bind_group = make::bind_group(
			device,
			queue,
			colored_layout,
			&look_matrix_uniform,
			Extent3d { width: tr_reader::model::PALETTE_SIZE as u32, height: 1, depth_or_array_layers: 1 },
			TextureDimension::D1,
			TextureFormat::Rgba8UnormSrgb,
			palette.as_slice(),
		);
		(colored_vertex_buf, colored_bind_group)
	});
	LevelViewState {
		look_matrix_uniform,
		textured_vertex_buffer: textured_vertex_buf,
		textured_bind_group,
		colored,
		room_vertex_indices,
		static_room_indices,
		flip_groups,
		cam_pos,
		yaw,
		pitch,
	}
}

macro_rules! keys_decl {
	($($symbol:ident, $($keycode:expr),*;)*) => {
		#[derive(Clone, Copy)]
		enum TrackedKey { $($symbol),* }
		
		const NUM_TRACKED_KEYS: usize = [$(TrackedKey::$symbol),*].len();
		
		fn get_key_map() -> HashMap<KeyCode, TrackedKey> {
			HashMap::from([$($(($keycode, $symbol),)*)*])
		}
	};
}

keys_decl!(
	Exit, KeyCode::Escape, KeyCode::KeyF;
	Forward, KeyCode::KeyW, KeyCode::ArrowUp;
	Left, KeyCode::KeyA, KeyCode::ArrowLeft;
	Backward, KeyCode::KeyS, KeyCode::ArrowDown;
	Right, KeyCode::KeyD, KeyCode::ArrowRight;
	Up, KeyCode::KeyQ, KeyCode::PageUp;
	Down, KeyCode::KeyE, KeyCode::PageDown;
	Shift, KeyCode::ShiftLeft, KeyCode::ShiftRight;
	Control, KeyCode::ControlLeft, KeyCode::ControlRight;
	Open, KeyCode::KeyO;
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
	let textured_bind_group_layout_ref = [&textured_bind_group_layout];
	let textured_shader = make::shader(&device, include_str!("shader/textured.wgsl"));
	let textured_pipeline_layout_descriptor = make::pipeline_layout_descriptor(&textured_bind_group_layout_ref);
	let textured_vertex_attributes = make::vertex_attributes(VertexFormat::Float32x2);
	let textured_vertex_buffer_layout = make::vertex_buffer_layout(size_of::<TexturedVertex>() as u64, &textured_vertex_attributes);
	let textured_vertex_state = make::vertex_state(&textured_shader, slice::from_ref(&textured_vertex_buffer_layout));
	
	let colored_bind_group_layout = make::bind_group_layout(&device, TextureViewDimension::D1);
	let colored_bind_group_layout_ref = [&colored_bind_group_layout];
	let colored_shader = make::shader(&device, include_str!("shader/colored.wgsl"));
	let colored_pipeline_layout_descriptor = make::pipeline_layout_descriptor(&colored_bind_group_layout_ref);
	let colored_vertex_attributes = make::vertex_attributes(VertexFormat::Uint32);
	let colored_vertex_buffer_layout = make::vertex_buffer_layout(size_of::<ColoredVertex>() as u64, &colored_vertex_attributes);
	let colored_vertex_state = make::vertex_state(&colored_shader, slice::from_ref(&colored_vertex_buffer_layout));
	
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
	let colored_pipeline = make::render_pipeline(
		&device,
		&colored_pipeline_layout_descriptor,
		colored_vertex_state,
		&colored_shader,
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
			&level_path,
			&device,
			&queue,
			window_size,
			&textured_bind_group_layout,
			&colored_bind_group_layout,
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
						if let Some(LevelViewState {
							look_matrix_uniform,
							cam_pos,
							yaw,
							pitch,
							..
						}) = &level_view_state {
							write_look_matrix(&queue, look_matrix_uniform, &make::look_matrix(window_size, *cam_pos, *yaw, *pitch));
						}
					},
					WindowEvent::RedrawRequested => {
						let now = Instant::now();
						let delta_time = now - last_frame;
						let frame = surface.get_current_texture().expect("Failed to acquire next swap chain texture");
						let color_view = frame.texture.create_view(&TextureViewDescriptor::default());
						let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());
						
						if let Some(LevelViewState {
							look_matrix_uniform,
							textured_vertex_buffer,
							textured_bind_group,
							colored,
							room_vertex_indices,
							static_room_indices,
							flip_groups,
							cam_pos,
							yaw,
							pitch,
						}) = &mut level_view_state {
							if let Some(movement) = MOVEMENT_MAP.into_iter().filter_map(|(key, vec)| keys.down(key).then_some(vec)).reduce(|a, b| a + b) {
								*cam_pos +=
									if keys.down(Shift) { 5.0 } else { 1.0 } *
									if keys.down(Control) { 0.2 } else { 1.0 } *
									5.0 *
									delta_time.as_secs_f32() *
									Mat4::from_rotation_y(*yaw).inverse().transform_point3(movement);
								write_look_matrix(&queue, look_matrix_uniform, &make::look_matrix(window_size, *cam_pos, *yaw, *pitch));
							}
							let room_indices = get_room_indices(static_room_indices, flip_groups).collect::<Vec<_>>();
							{
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
							}
							if let Some((colored_vertex_buffer, colored_bind_group)) = colored {
								let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
									label: None,
									color_attachments: &[Some(RenderPassColorAttachment {
										ops: Operations { load: LoadOp::Load, store: StoreOp::Store },
										resolve_target: None,
										view: &color_view,
									})],
									depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
										depth_ops: Some(Operations { load: LoadOp::Load, store: StoreOp::Store }),
										stencil_ops: None,
										view: &depth_view,
									}),
									timestamp_writes: None,
									occlusion_query_set: None,
								});
								rpass.set_bind_group(0, colored_bind_group, &[]);
								rpass.set_vertex_buffer(0, colored_vertex_buffer.slice(..));
								rpass.set_pipeline(&colored_pipeline);
								for &room_index in &room_indices {
									rpass.draw(room_vertex_indices[room_index].colored.clone(), 0..1);
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
									&path.to_string_lossy(),
									&device,
									&queue,
									window_size,
									&textured_bind_group_layout,
									&colored_bind_group_layout,
								));
							}
						}
						
						keys.clear_just_pressed();
						last_frame = now;
						window.request_redraw();
					},
					WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Right, .. } => if camera_control {
						window.set_cursor_visible(true);
						window.set_cursor_grab(CursorGrabMode::None).unwrap();
						camera_control = false;
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
			if let Some(LevelViewState { look_matrix_uniform, cam_pos, yaw, pitch, .. }) = &mut level_view_state {
				*yaw = *yaw + delta_x as f32 / 150.0;
				*pitch = (*pitch + delta_y as f32 / 150.0).clamp(-FRAC_PI_2, FRAC_PI_2);
				write_look_matrix(&queue, look_matrix_uniform, &make::look_matrix(window_size, *cam_pos, *yaw, *pitch));
			}
		},
		Event::DeviceEvent { event: DeviceEvent::Key(RawKeyEvent { physical_key: PhysicalKey::Code(keycode), state }), .. } => {
			if let Some(&tracked_key) = key_map.get(&keycode) {
				match state {
					ElementState::Pressed => keys.press(tracked_key),
					ElementState::Released => keys.release(tracked_key),
				}
			}
			if keys.just_pressed(Exit) {
				target.exit();
			}
			if keys.down(Control) && keys.just_pressed(Open) {
				choosing_file = true;
				file_dialog.select_file();
			}
		},
		_ => {},
	}).expect("run event loop");
}
