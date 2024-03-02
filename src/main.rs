mod ext;
mod load;

use std::{
	borrow::Cow,
	collections::HashMap,
	env::args,
	f32::consts::{FRAC_PI_2, FRAC_PI_4},
	mem::size_of,
	num::NonZeroU32,
	time::Instant,
};
use egui_file_dialog::{DialogState, FileDialog, FileDialogConfig};
use glam::{vec3, EulerRot, Mat4, Vec3, Vec3Swizzles};
use wgpu::{
	util::{BufferInitDescriptor, DeviceExt, TextureDataOrder},
	BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
	BindGroupLayoutEntry, BindingResource, BindingType, BlendComponent, BlendFactor,
	BlendOperation, BlendState, Buffer, BufferBindingType, BufferSize, BufferUsages, Color,
	ColorTargetState, ColorWrites, CommandEncoderDescriptor, CompareFunction, DepthBiasState,
	DepthStencilState, Device, DeviceDescriptor, Extent3d, Face, Features, FragmentState, Instance,
	Limits, LoadOp, MultisampleState, Operations, PipelineLayoutDescriptor, PowerPreference,
	PrimitiveState, Queue, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
	RenderPassDescriptor, RenderPipelineDescriptor, RequestAdapterOptions, ShaderModuleDescriptor,
	ShaderSource, ShaderStages, StencilState, StoreOp, TextureDescriptor, TextureDimension,
	TextureFormat, TextureSampleType, TextureUsages, TextureView, TextureViewDescriptor,
	TextureViewDimension, VertexAttribute, VertexBufferLayout, VertexFormat, VertexState,
	VertexStepMode,
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
use load::{load_level_render_data, FlipGroup, LevelRenderData, RoomVertexIndices, Vertex};

const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;

fn fill_dark(window: &Window) -> Result<(), softbuffer::SoftBufferError> {
	let size = window.inner_size();
	let mut surface = softbuffer::Surface::new(&softbuffer::Context::new(window)?, window)?;
	surface.resize(NonZeroU32::new(size.width).unwrap(), NonZeroU32::new(size.height).unwrap())?;
	let mut buffer = surface.buffer_mut()?;
	buffer.fill(0);
	Ok(buffer.present()?)
}

fn create_look_matrix(window_size: PhysicalSize<u32>, cam_pos: Vec3, yaw: f32, pitch: f32) -> Mat4 {
	Mat4::perspective_rh(FRAC_PI_4, window_size.width as f32 / window_size.height as f32, 0.1, 200.0) *
	Mat4::from_euler(EulerRot::XYZ, pitch, yaw, 0.0) *
	Mat4::from_translation(cam_pos) *
	Mat4::from_scale(vec3(1.0, -1.0, -1.0))
}

fn get_yaw_pitch(v: Vec3) -> (f32, f32) {
	(v.z.atan2(v.x), v.y.atan2(v.xz().length()))
}

fn write_look_matrix(queue: &Queue, look_matrix_uniform: &Buffer, look_matrix: &Mat4) {
	queue.write_buffer(look_matrix_uniform, 0, unsafe { reinterpret::ref_to_slice(look_matrix) });//floats to bytes
}

fn create_depth_texture(device: &Device, window_size: PhysicalSize<u32>) -> TextureView {
	device.create_texture(&TextureDescriptor {
		label: None,
		size: Extent3d { width: window_size.width, height: window_size.height, depth_or_array_layers: 1 },
		mip_level_count: 1,
		sample_count: 1,
		dimension: TextureDimension::D2,
		format: DEPTH_FORMAT,
		usage: TextureUsages::RENDER_ATTACHMENT,
		view_formats: &[],
	}).create_view(&TextureViewDescriptor::default())
}

fn room_indices<'a>(static_room_indices: &'a [usize], flip_groups: &'a [FlipGroup]) -> impl Iterator<Item = usize> + 'a {
	static_room_indices.iter().copied().chain(flip_groups.iter().flat_map(|flip_group| flip_group.get_room_indices()))
}

struct LevelViewState {
	look_matrix_uniform: Buffer,
	cam_pos: Vec3,
	yaw: f32,
	pitch: f32,
	flip_groups: Vec<FlipGroup>,
	bind_group: BindGroup,
	vertex_buf: Buffer,
	static_room_indices: Vec<usize>,
	room_vertex_indices: Vec<RoomVertexIndices>,
}

fn load_level(
	path: &str,
	device: &Device,
	queue: &Queue,
	window_size: PhysicalSize<u32>,
	layout: &BindGroupLayout,
) -> LevelViewState {
	let LevelRenderData {
		atlas_size,
		atlas_data,
		vertices,
		room_vertex_indices,
		static_room_indices,
		flip_groups,
	} = load_level_render_data(&path);
	let bounds = MinMax::from_iter(vertices.iter().map(|v| v.pos)).unwrap();
	let (yaw, pitch) = get_yaw_pitch(bounds.max - bounds.min);
	let cam_pos = bounds.min;
	let vertex_buf = device.create_buffer_init(&BufferInitDescriptor {
		label: None,
		usage: BufferUsages::VERTEX,
		contents: unsafe { reinterpret::slice(&vertices) },//contiguous primitives to bytes
	});
	let look_matrix_uniform = device.create_buffer_init(&BufferInitDescriptor {
		label: None,
		usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
		contents: unsafe { reinterpret::ref_to_slice(&create_look_matrix(window_size, cam_pos, yaw, pitch)) },//floats to bytes
	});
	let bind_group = device.create_bind_group(&BindGroupDescriptor {
		label: None,
		layout,
		entries: &[
			BindGroupEntry {
				binding: 0,
				resource: look_matrix_uniform.as_entire_binding(),
			},
			BindGroupEntry {
				binding: 1,
				resource: BindingResource::TextureView(&device.create_texture_with_data(
					queue,
					&TextureDescriptor {
						label: None,
						size: Extent3d { width: atlas_size.x, height: atlas_size.y, depth_or_array_layers: 1 },
						mip_level_count: 1,
						sample_count: 1,
						dimension: TextureDimension::D2,
						format: TextureFormat::Bgra8UnormSrgb,
						usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
						view_formats: &[],
					},
					TextureDataOrder::default(),
					&atlas_data,
				).create_view(&TextureViewDescriptor::default()))
			},
		],
	});
	LevelViewState {
		look_matrix_uniform,
		cam_pos,
		yaw,
		pitch,
		flip_groups,
		bind_group,
		vertex_buf,
		static_room_indices,
		room_vertex_indices,
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
	let mut level_view_state: Option<LevelViewState>;
	
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
	let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
		label: None,
		entries: &[
			BindGroupLayoutEntry {
				binding: 0,
				visibility: ShaderStages::VERTEX,
				count: None,
				ty: BindingType::Buffer {
					ty: BufferBindingType::Uniform,
					has_dynamic_offset: false,
					min_binding_size: BufferSize::new(size_of::<Mat4>() as u64),
				},
			},
			BindGroupLayoutEntry {
				binding: 1,
				visibility: ShaderStages::FRAGMENT,
				count: None,
				ty: BindingType::Texture {
					sample_type: TextureSampleType::Float { filterable: false },
					view_dimension: TextureViewDimension::D2,
					multisampled: false,
				},
			},
		],
	});
	
	level_view_state = args().skip(1).next().map(|level_path| load_level(&level_path, &device, &queue, window_size, &bind_group_layout));
	
	let shader = device.create_shader_module(ShaderModuleDescriptor {
		label: None,
		source: ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
	});
	let pipeline_layout_descriptor = PipelineLayoutDescriptor {
		label: None,
		bind_group_layouts: &[&bind_group_layout],
		push_constant_ranges: &[],
	};
	let vertex_state = VertexState {
		module: &shader,
		entry_point: "vs_main",
		buffers: &[VertexBufferLayout {
			array_stride: size_of::<Vertex>() as u64,
			step_mode: VertexStepMode::Vertex,
			attributes: &[
				VertexAttribute {
					offset: 0,
					format: VertexFormat::Float32x3,
					shader_location: 0,
				},
				VertexAttribute {
					offset: VertexFormat::Float32x3.size(),
					format: VertexFormat::Float32x2,
					shader_location: 1,
				},
			],
		}],
	};
	let primitive_state = PrimitiveState { cull_mode: Some(Face::Front), ..PrimitiveState::default() };
	let opaque_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
		label: None,
		layout: Some(&device.create_pipeline_layout(&pipeline_layout_descriptor)),
		vertex: vertex_state.clone(),
		fragment: Some(FragmentState {
			module: &shader,
			entry_point: "fs_main",
			targets:
			&[Some(ColorTargetState {
				blend: None,
				format: TextureFormat::Bgra8UnormSrgb,
				write_mask: ColorWrites::all(),
			})],
		}),
		primitive: primitive_state,
		depth_stencil: Some(DepthStencilState {
			bias: DepthBiasState::default(),
			depth_compare: CompareFunction::Less,
			depth_write_enabled: true,
			format: DEPTH_FORMAT,
			stencil: StencilState::default(),
		}),
		multisample: MultisampleState::default(),
		multiview: None,
	});
	let additive_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
		label: None,
		layout: Some(&device.create_pipeline_layout(&pipeline_layout_descriptor)),
		vertex: vertex_state,
		fragment: Some(FragmentState {
			module: &shader,
			entry_point: "fs_main",
			targets: &[Some(ColorTargetState {
				blend: Some(BlendState {
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
				format: TextureFormat::Bgra8UnormSrgb,
				write_mask: ColorWrites::all(),
			})],
		}),
		primitive: primitive_state,
		depth_stencil: Some(DepthStencilState {
			bias: DepthBiasState::default(),
			depth_compare: CompareFunction::Less,
			depth_write_enabled: false,
			format: DEPTH_FORMAT,
			stencil: StencilState::default(),
		}),
		multisample: MultisampleState::default(),
		multiview: None,
	});
	
	let egui_ctx = egui::Context::default();
	let mut egui_input_state = egui_winit::State::new(egui_ctx.clone(), egui_ctx.viewport_id(), window, None, None);
	let mut egui_renderer = egui_wgpu::Renderer::new(&device, TextureFormat::Bgra8UnormSrgb, None, 1);
	let mut file_dialog = FileDialog::with_config(FileDialogConfig {
		initial_directory: std::fs::read_to_string("dir").ok().unwrap_or(".".into()).into(),
		..Default::default()
	});
	let mut choosing_file = false;
	
	let mut depth_texture = create_depth_texture(&device, window_size);
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
						depth_texture = create_depth_texture(&device, window_size);
						if let Some(LevelViewState {
							look_matrix_uniform,
							cam_pos,
							yaw,
							pitch,
							..
						}) = &level_view_state {
							write_look_matrix(&queue, look_matrix_uniform, &create_look_matrix(window_size, *cam_pos, *yaw, *pitch));
						}
					},
					WindowEvent::RedrawRequested => {
						let now = Instant::now();
						let delta_time = now - last_frame;
						
						let frame = surface.get_current_texture().expect("Failed to acquire next swap chain texture");
						let view = &frame.texture.create_view(&TextureViewDescriptor::default());
						let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());
						
						if let Some(LevelViewState {
							look_matrix_uniform,
							cam_pos,
							yaw,
							pitch,
							flip_groups,
							bind_group,
							vertex_buf,
							static_room_indices,
							room_vertex_indices,
						}) = &mut level_view_state {
							let movement = [
								(keys.down(Forward), Vec3::Z),
								(keys.down(Backward), Vec3::NEG_Z),
								(keys.down(Left), Vec3::X),
								(keys.down(Right), Vec3::NEG_X),
								(keys.down(Down), Vec3::Y),
								(keys.down(Up), Vec3::NEG_Y),
							].into_iter().filter_map(|(dir, vec)| dir.then_some(vec)).reduce(|a, b| a + b);
							if let Some(movement) = movement {
								*cam_pos +=
									if keys.down(Shift) { 5.0 } else { 1.0 } *
									if keys.down(Control) { 0.2 } else { 1.0 } *
									5.0 *
									delta_time.as_secs_f32() *
									Mat4::from_rotation_y(*yaw).inverse().transform_point3(movement);
								write_look_matrix(&queue, look_matrix_uniform, &create_look_matrix(window_size, *cam_pos, *yaw, *pitch));
							}
							let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
								label: None,
								color_attachments: &[Some(RenderPassColorAttachment {
									ops: Operations { load: LoadOp::Clear(Color::BLACK), store: StoreOp::Store },
									resolve_target: None,
									view,
								})],
								depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
									depth_ops: Some(Operations { load: LoadOp::Clear(1.0), store: StoreOp::Store }),
									stencil_ops: None,
									view: &depth_texture,
								}),
								timestamp_writes: None,
								occlusion_query_set: None,
							});
							rpass.set_bind_group(0, bind_group, &[]);
							rpass.set_vertex_buffer(0, vertex_buf.slice(..));
							rpass.set_pipeline(&opaque_pipeline);
							for room_index in room_indices(static_room_indices, flip_groups) {
								rpass.draw(room_vertex_indices[room_index].opaque.clone(), 0..1);
							}
							rpass.set_pipeline(&additive_pipeline);
							for room_index in room_indices(static_room_indices, flip_groups) {
								rpass.draw(room_vertex_indices[room_index].additive.clone(), 0..1);
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
						
						if choosing_file {
							let file_dialog_state = file_dialog.state();
							choosing_file = matches!(file_dialog_state, DialogState::Open);
							if let DialogState::Selected(path) = file_dialog_state {
								level_view_state = Some(load_level(path.to_str().expect("file path UTF-8"), &device, &queue, window_size, &bind_group_layout));
							}
						}
						
						let screen_descriptor = egui_wgpu::ScreenDescriptor { size_in_pixels: window_size.into(), pixels_per_point };
						egui_input_state.handle_platform_output(window, platform_output);
						for (id, delta) in &tex_set {
							egui_renderer.update_texture(&device, &queue, *id, delta)
						}
						let egui_tris = egui_ctx.tessellate(shapes, pixels_per_point);
						egui_renderer.update_buffers(&device, &queue, &mut encoder, &egui_tris, &screen_descriptor);
						let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
							label: None,
							color_attachments: &[Some(RenderPassColorAttachment {
								view,
								resolve_target: None,
								ops: Operations { load: LoadOp::Load, store: StoreOp::Store },
							})],
							depth_stencil_attachment: None,
							timestamp_writes: None,
							occlusion_query_set: None,
						});
						egui_renderer.render(&mut rpass, &egui_tris, &screen_descriptor);
						drop(rpass);
						for id in &tex_free {
							egui_renderer.free_texture(id);
						}
						
						queue.submit([encoder.finish()]);
						frame.present();
						
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
				write_look_matrix(&queue, look_matrix_uniform, &create_look_matrix(window_size, *cam_pos, *yaw, *pitch));
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
