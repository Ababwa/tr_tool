mod ext;
mod gui;
mod make;

use std::{f32::consts::FRAC_PI_2, fs::File};
use egui_file_dialog::{DialogMode, FileDialog};
use ext::{AsBytes, IntoValIter};
use glam::{ivec2, DVec2, IVec2, Mat4, UVec2, Vec3};
use gui::Gui;
use wgpu::{util::{DeviceExt, TextureDataOrder}, BindGroup, BindGroupLayout, BindingResource, BindingType, Buffer, BufferUsages, Color, ColorTargetState, ColorWrites, CommandEncoder, CompareFunction, DepthBiasState, DepthStencilState, Device, Extent3d, Face, FragmentState, LoadOp, MultisampleState, Operations, PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology, Queue, RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, ShaderStages, StencilState, StoreOp, TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension, VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode};
use winit::{
	event::{ElementState, MouseButton, MouseScrollDelta}, event_loop::EventLoopWindowTarget,
	keyboard::{KeyCode, ModifiersState}, window::{CursorGrabMode, Icon, Window},
};
use tr_model::{tr1, Readable};

const QUAD_VERTS: [u32; 4] = [0, 1, 2, 3];
const NUM_QUAD_VERTS: u32 = QUAD_VERTS.len() as u32;
const OBJ_TEX_BUF_LEN: usize = 816;

type VertexBuffer = [tr1::RoomVertex; 2048];
type ObjectTextureBuffer = [tr1::ObjectTexture; OBJ_TEX_BUF_LEN];

#[repr(C)]
struct PaddedQuad {
	quad: tr1::Quad,
	pad: u16,
}

impl From<tr1::Quad> for PaddedQuad {
	fn from(quad: tr1::Quad) -> Self {
		Self { quad, pad: 0 }
	}
}

struct RoomData {
	bind_group: BindGroup,
	quads: Buffer,
	num_quads: u32,
}

struct LoadedLevel {
	rooms: Vec<RoomData>,
	pos: Vec3,
	yaw: f32,
	pitch: f32,
	mouse_control: bool,
	transform_buffer: Buffer,
}

fn load_level(
	window_size: UVec2,
	device: &Device,
	queue: &Queue,
	room_quad_layout: &BindGroupLayout,
	level: tr1::Level,
) -> LoadedLevel {
	let pos = Vec3::ZERO;
	let yaw = 0.0;
	let pitch = 0.0;
	let transform = make::transform(window_size, pos, yaw, pitch);
	let transform_buffer = make::buffer(device, transform.as_bytes(), BufferUsages::UNIFORM | BufferUsages::COPY_DST);
	let mut palette = level.palette
		.into_iter()
		.map(|color| [color.r, color.b, color.g, 255])
		.collect::<Vec<_>>();//upgrade palette to rgba
	palette[0][3] = 0;//index 0 is transparent
	let num_atlases = level.atlases.len();
	let image = level.atlases
		.into_val_iter()
		.flatten()
		.map(|color_index| palette[color_index as usize])
		.flatten()
		.collect::<Vec<_>>();//build rgba image
	let texture_view = device
		.create_texture_with_data(
			queue,
			&TextureDescriptor {
				label: None,
				size: Extent3d {
					width: tr1::ATLAS_SIZE as u32,
					height: tr1::ATLAS_SIZE as u32,
					depth_or_array_layers: num_atlases as u32,
				},
				mip_level_count: 1,
				sample_count: 1,
				dimension: TextureDimension::D2,
				format: TextureFormat::Rgba8UnormSrgb,
				usage: TextureUsages::TEXTURE_BINDING,
				view_formats: &[],
			},
			TextureDataOrder::default(),
			&image,
		)
		.create_view(&TextureViewDescriptor::default());
	let (object_textures1, object_textures2) = if level.object_textures.len() > OBJ_TEX_BUF_LEN {
		(&level.object_textures[..OBJ_TEX_BUF_LEN], &level.object_textures[OBJ_TEX_BUF_LEN..])
	} else {
		(&level.object_textures[..], [].as_slice())
	};
	let object_textures_buffer1 = make::buffer_fixed::<ObjectTextureBuffer>(device, object_textures1.as_bytes(), BufferUsages::UNIFORM);
	let object_textures_buffer2 = make::buffer_fixed::<ObjectTextureBuffer>(device, object_textures2.as_bytes(), BufferUsages::UNIFORM);
	let rooms = level.rooms
		.into_val_iter()
		.map(|room| {
			RoomData {
				bind_group: make::bind_group(
					device,
					room_quad_layout,
					&[
						transform_buffer.as_entire_binding(),
						make::buffer(device, ivec2(room.x, room.z).as_bytes(), BufferUsages::UNIFORM).as_entire_binding(),
						make::buffer_fixed::<VertexBuffer>(device, room.vertices.as_bytes(), BufferUsages::UNIFORM).as_entire_binding(),
						object_textures_buffer1.as_entire_binding(),
						object_textures_buffer2.as_entire_binding(),
						BindingResource::TextureView(&texture_view),
					],
				),
				num_quads: room.quads.len() as u32,
				quads: make::buffer(device, room.quads.into_val_iter().map(|quad| PaddedQuad::from(quad)).collect::<Vec<_>>().as_slice().as_bytes(), BufferUsages::VERTEX),
			}
		})
		.collect::<Vec<_>>();
	LoadedLevel {
		rooms,
		pos,
		yaw,
		pitch,
		mouse_control: false,
		transform_buffer,
	}
}

struct TrTool {
	modifiers: ModifiersState,
	file_dialog: FileDialog,
	error: Option<String>,
	quad_verts: Buffer,
	depth_view: TextureView,
	room_quad_layout: BindGroupLayout,
	room_quad_pipeline: RenderPipeline,
	loaded_level: Option<LoadedLevel>,
}

fn window<R, F: FnOnce(&mut egui::Ui) -> R>(ctx: &egui::Context, title: &str, contents: F) -> R {
	egui::Window::new(title).collapsible(false).resizable(false).show(ctx, contents).unwrap().inner.unwrap()
}

fn update_transform(window_size: UVec2, queue: &Queue, pos: Vec3, yaw: f32, pitch: f32, transform_buffer: &Buffer) {
	let transform = make::transform(window_size, pos, yaw, pitch);
	queue.write_buffer(transform_buffer, 0, transform.as_bytes());
}

impl Gui for TrTool {
	fn resize(&mut self, window_size: UVec2, device: &Device, queue: &Queue) {
		_ = queue;
		self.depth_view = make::depth_view(device, window_size);
	}
	
	fn modifiers(&mut self, modifers: ModifiersState) {
		self.modifiers = modifers;
	}
	
	fn key(
		&mut self, window_size: UVec2, device: &Device, queue: &Queue, target: &EventLoopWindowTarget<()>,
		keycode: KeyCode, state: ElementState, repeat: bool,
	) {
		_ = device;
		match (&mut self.loaded_level, self.modifiers, state, keycode, repeat) {
			(_, _, ElementState::Pressed, KeyCode::Escape, _) => target.exit(),
			(_, ModifiersState::CONTROL, ElementState::Pressed, KeyCode::KeyO, false) => self.file_dialog.select_file(),
			(Some(LoadedLevel { pos, yaw, pitch, transform_buffer, .. }), _, ElementState::Pressed, KeyCode::KeyW, _) => {
				pos.x += 10.0;
				update_transform(window_size, queue, *pos, *yaw, *pitch, transform_buffer);
			},
			(Some(LoadedLevel { pos, yaw, pitch, transform_buffer, .. }), _, ElementState::Pressed, KeyCode::KeyS, _) => {
				pos.x -= 10.0;
				update_transform(window_size, queue, *pos, *yaw, *pitch, transform_buffer);
			},
			(Some(LoadedLevel { pos, yaw, pitch, transform_buffer, .. }), _, ElementState::Pressed, KeyCode::KeyA, _) => {
				pos.y += 10.0;
				update_transform(window_size, queue, *pos, *yaw, *pitch, transform_buffer);
			},
			(Some(LoadedLevel { pos, yaw, pitch, transform_buffer, .. }), _, ElementState::Pressed, KeyCode::KeyD, _) => {
				pos.y -= 10.0;
				update_transform(window_size, queue, *pos, *yaw, *pitch, transform_buffer);
			},
			(Some(LoadedLevel { pos, yaw, pitch, transform_buffer, .. }), _, ElementState::Pressed, KeyCode::KeyQ, _) => {
				pos.z += 10.0;
				update_transform(window_size, queue, *pos, *yaw, *pitch, transform_buffer);
			},
			(Some(LoadedLevel { pos, yaw, pitch, transform_buffer, .. }), _, ElementState::Pressed, KeyCode::KeyE, _) => {
				pos.z -= 10.0;
				update_transform(window_size, queue, *pos, *yaw, *pitch, transform_buffer);
			},
			_ => {},
		}
	}
	
	fn mouse_button(&mut self, window: &Window, button: MouseButton, state: ElementState) {
		if let (Some(LoadedLevel { mouse_control, .. }), ElementState::Pressed, MouseButton::Right) = (&mut self.loaded_level, state, button) {
			if *mouse_control {
				window.set_cursor_visible(true);
				window.set_cursor_grab(CursorGrabMode::None).expect("cursor ungrab");
			} else {
				window.set_cursor_visible(false);
				window.set_cursor_grab(CursorGrabMode::Confined).or_else(|_| window.set_cursor_grab(CursorGrabMode::Locked)).expect("cursor grab");
			}
			*mouse_control ^= true;
		}
	}
	
	fn mouse_moved(&mut self, window_size: UVec2, queue: &Queue, delta: DVec2) {
		if let Some(LoadedLevel { pos, yaw, pitch, mouse_control: true, transform_buffer, .. }) = &mut self.loaded_level {
			*yaw += delta.x as f32 / 150.0;
			*pitch = (*pitch + delta.y as f32 / 150.0).clamp(-FRAC_PI_2, FRAC_PI_2);
			update_transform(window_size, queue, *pos, *yaw, *pitch, transform_buffer);
		}
	}
	
	fn mouse_wheel(&mut self, queue: &Queue, delta: MouseScrollDelta) {
		_ = (queue, delta);
	}
	
	fn render(&mut self, encoder: &mut CommandEncoder, color_view: &TextureView) {
		if let Some(LoadedLevel { rooms, .. }) = &self.loaded_level {
			let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
				label: None,
				color_attachments: &[
					Some(RenderPassColorAttachment {
						ops: Operations {
							load: LoadOp::Clear(Color::BLACK),
							store: StoreOp::Store,
						},
						resolve_target: None,
						view: color_view,
					}),
				],
				depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
					depth_ops: Some(Operations {
						load: LoadOp::Clear(1.0),
						store: StoreOp::Store,
					}),
					stencil_ops: None,
					view: &self.depth_view,
				}),
				timestamp_writes: None,
				occlusion_query_set: None,
			});
			rpass.set_pipeline(&self.room_quad_pipeline);
			rpass.set_vertex_buffer(0, self.quad_verts.slice(..));
			for room in rooms {
				rpass.set_vertex_buffer(1, room.quads.slice(..));
				rpass.set_bind_group(0, &room.bind_group, &[]);
				rpass.draw(0..NUM_QUAD_VERTS, 0..room.num_quads);
			}
		}
	}
	
	fn gui(&mut self, window_size: UVec2, device: &Device, queue: &Queue, ctx: &egui::Context) {
		_ = (window_size, device, queue);
		self.file_dialog.update(ctx);
		if let DialogMode::SelectFile = self.file_dialog.mode() {
			if let Some(path) = self.file_dialog.take_selected() {
				match File::open(path) {
					Ok(mut file) => {
						match tr1::Level::read(&mut file) {
							Ok(level) => self.loaded_level = Some(load_level(
								window_size, device, queue, &self.room_quad_layout, level,
							)),
							Err(e) => self.error = Some(e.to_string()),
						}
					},
					Err(e) => self.error = Some(e.to_string()),
				}
			}
		}
		match &self.loaded_level {
			Some(loaded_level) => {
				window(ctx, "Level", |ui| {
					ui.label(format!("{} rooms", loaded_level.rooms.len()));
				});
			},
			None => {
				egui::panel::CentralPanel::default().show(ctx, |ui| {
					ui.centered_and_justified(|ui| {
						if ui.label("Ctrl+O or click to open file").clicked() {
							self.file_dialog.select_file();
						}
					});
				});
			},
		}
		if let Some(error) = &self.error {
			if window(ctx, "Error", |ui| {
				ui.label(error);
				ui.button("OK").clicked()
			}) {
				self.error = None;
			}
		}
	}
}

macro_rules! shader {
	($device:expr, $path:literal) => {
		make::shader($device, include_str!($path))
	};
}

fn make_gui(device: &Device, window_size: UVec2) -> TrTool {
	let room_quad_shader = shader!(device, "shader/room_quad.wgsl");
	let room_quad_layout = make::bind_group_layout(
		device,
		&[
			(make::uniform_layout_entry::<Mat4>(), ShaderStages::VERTEX),
			(make::uniform_layout_entry::<IVec2>(), ShaderStages::VERTEX),
			(make::uniform_layout_entry::<VertexBuffer>(), ShaderStages::VERTEX),
			(make::uniform_layout_entry::<ObjectTextureBuffer>(), ShaderStages::VERTEX),
			(make::uniform_layout_entry::<ObjectTextureBuffer>(), ShaderStages::VERTEX),
			(
				BindingType::Texture {
					sample_type: TextureSampleType::Float { filterable: false },
					view_dimension: TextureViewDimension::D2Array,
					multisampled: false,
				},
				ShaderStages::FRAGMENT,
			),
		],
	);
	let room_quad_pipeline = device.create_render_pipeline(
		&RenderPipelineDescriptor {
			label: None,
			layout: Some(&device.create_pipeline_layout(
				&PipelineLayoutDescriptor {
					label: None,
					bind_group_layouts: &[&room_quad_layout],
					push_constant_ranges: &[],
				},
			)),
			vertex: VertexState {
				module: &room_quad_shader,
				entry_point: "vs_main",
				buffers: &[
					VertexBufferLayout {
						array_stride: 4,
						step_mode: VertexStepMode::Vertex,
						attributes: &[
							VertexAttribute {
								format: VertexFormat::Uint32,
								offset: 0,
								shader_location: 0,
							},
						],
					},
					VertexBufferLayout {
						array_stride: 12,
						step_mode: VertexStepMode::Instance,
						attributes: &[
							VertexAttribute {
								format: VertexFormat::Uint16x4,
								offset: 0,
								shader_location: 1,
							},
							VertexAttribute {
								format: VertexFormat::Uint16x2,
								offset: 8,
								shader_location: 2,
							},
						],
					},
				],
			},
			primitive: PrimitiveState {
				topology: PrimitiveTopology::TriangleStrip,
				cull_mode: Some(Face::Front),
				..PrimitiveState::default()
			},
			depth_stencil: Some(DepthStencilState {
				bias: DepthBiasState::default(),
				depth_compare: CompareFunction::Less,
				depth_write_enabled: true,
				format: TextureFormat::Depth32Float,
				stencil: StencilState::default(),
			}),
			multisample: MultisampleState::default(),
			fragment: Some(FragmentState {
				entry_point: "fs_main",
				module: &room_quad_shader,
				targets: &[
					Some(ColorTargetState {
						format: TextureFormat::Bgra8UnormSrgb,
						blend: None,
						write_mask: ColorWrites::ALL,
					}),
				],
			}),
			multiview: None,
		},
	);
	TrTool {
		modifiers: ModifiersState::empty(),
		file_dialog: FileDialog::new().initial_directory("C:\\Program Files (x86)\\Steam\\steamapps\\common\\Tomb Raider (I)\\extracted\\DATA".into()),
		quad_verts: make::buffer(device, QUAD_VERTS.as_bytes(), BufferUsages::VERTEX),
		error: None,
		depth_view: make::depth_view(device, window_size),
		room_quad_layout,
		room_quad_pipeline,
		loaded_level: None,
	}
}

fn main() {
	// for entry in std::fs::read_dir("C:\\Program Files (x86)\\Steam\\steamapps\\common\\Tomb Raider (I)\\extracted\\DATA").unwrap() {
	// 	let entry = entry.unwrap();
	// 	let path = entry.path();
	// 	if entry.file_type().unwrap().is_file() && path.extension().map_or(false, |e| e.eq_ignore_ascii_case("phd")) {
	// 		let level = tr1::Level::read(&mut File::open(path).unwrap()).unwrap();
	// 		let ots = level.object_textures.len();
	// 		println!("{:?} object_textures: {}", entry.file_name(), ots);
	// 	}
	// }
	let window_icon = Icon::from_rgba(include_bytes!("res/icon16.data").to_vec(), 16, 16).expect("window icon");
	let taskbar_icon = Icon::from_rgba(include_bytes!("res/icon24.data").to_vec(), 24, 24).expect("taskbar icon");
	gui::run("TR Tool", window_icon, taskbar_icon, make_gui);
}
