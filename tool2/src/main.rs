/*
Notes:
{val} is used in some places to copy an unaligned field to make it referenceable.
*/

mod as_bytes;
mod gui;
mod make;
mod keys;
mod vec_tail;
mod render_model;
mod double_end_cursor;
mod multi_cursor;

use std::{
	collections::HashMap, f32::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU}, fs::File, mem::{size_of, MaybeUninit}, time::Duration
};
use double_end_cursor::DoubleEndBuffer;
use keys::{KeyGroup, KeyStates};
use egui_file_dialog::FileDialog;
use as_bytes::AsBytes;
use glam::{DVec2, EulerRot, IVec3, Mat4, UVec2, Vec3, Vec3Swizzles};
use gui::Gui;
use render_model::{FaceBuffers, FaceArray, FaceInstance, Faces, Mesh, ModelRef};
use shared::min_max::{MinMax, VecMinMaxFromIterator};
use tr_model::{tr1, Readable};
use wgpu::{
	util::{DeviceExt, TextureDataOrder}, BindGroup, BindGroupLayout, BindingResource, BindingType, Buffer,
	BufferUsages, Color, ColorTargetState, ColorWrites, CommandEncoder, CompareFunction, DepthBiasState,
	DepthStencilState, Device, Extent3d, Face, FragmentState, FrontFace, LoadOp, MultisampleState,
	Operations, PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology, Queue,
	RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor, RenderPipeline,
	RenderPipelineDescriptor, ShaderModule, ShaderStages, StencilState, StoreOp, TextureDescriptor,
	TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureView, TextureViewDescriptor,
	TextureViewDimension, VertexFormat, VertexState, VertexStepMode,
};
use winit::{
	event::{ElementState, MouseButton, MouseScrollDelta}, event_loop::EventLoopWindowTarget,
	keyboard::{KeyCode, ModifiersState}, window::{CursorGrabMode, Icon, Window},
};

struct ActionMap {
	forward: KeyGroup,
	backward: KeyGroup,
	left: KeyGroup,
	right: KeyGroup,
	up: KeyGroup,
	down: KeyGroup,
	boost: KeyGroup,
}

struct TrTool {
	modifiers: ModifiersState,
	_file_dialog: FileDialog,
	error: Option<String>,
	depth_view: TextureView,
	transform_buffer: Buffer,
	face_vertex_index_buffer: Buffer,
	face_buffers: FaceBuffers,
	textured_pipeline: RenderPipeline,
	solid_pipeline: RenderPipeline,
	bind_group: BindGroup,
	mouse_control: bool,
	pos: Vec3,
	yaw: f32,
	pitch: f32,
	keys_states: KeyStates,
	action_map: ActionMap,
	print: bool,
}

fn camera_transform(pos: Vec3, yaw: f32, pitch: f32) -> Mat4 {
	Mat4::from_euler(EulerRot::XYZ, pitch, yaw, PI) * Mat4::from_translation(-pos)
}

fn perspective_transform(window_size: UVec2) -> Mat4 {
	Mat4::perspective_rh(FRAC_PI_4, window_size.x as f32 / window_size.y as f32, 100.0, 100000.0)
}

fn get_transform(window_size: UVec2, pos: Vec3, yaw: f32, pitch: f32) -> Mat4 {
	perspective_transform(window_size) * camera_transform(pos, yaw, pitch)
}

const FORWARD: Vec3 = Vec3::NEG_Z;
const BACKWARD: Vec3 = Vec3::Z;
const LEFT: Vec3 = Vec3::X;
const RIGHT: Vec3 = Vec3::NEG_X;
const DOWN: Vec3 = Vec3::Y;
const UP: Vec3 = Vec3::NEG_Y;

const FACE_VERT_INDICES: [u32; 4] = [1, 2, 0, 3];
const NUM_QUAD_VERTS: u32 = 4;
const NUM_TRI_VERTS: u32 = 3;

fn window<R, F: FnOnce(&mut egui::Ui) -> R>(ctx: &egui::Context, title: &str, contents: F) -> R {
	egui::Window::new(title).collapsible(false).resizable(false).show(ctx, contents).unwrap().inner.unwrap()
}

impl TrTool {
	fn update_transform(&self, window_size: UVec2, queue: &Queue) {
		let transform = get_transform(window_size, self.pos, self.yaw, self.pitch);
		queue.write_buffer(&self.transform_buffer, 0, transform.as_bytes());
	}
	
	fn frame_update(&mut self, window_size: UVec2, queue: &Queue, delta_time: Duration) {
		let movement = [
			(self.action_map.forward, FORWARD),
			(self.action_map.backward, BACKWARD),
			(self.action_map.left, LEFT),
			(self.action_map.right, RIGHT),
			(self.action_map.up, UP),
			(self.action_map.down, DOWN),
		]
			.into_iter()
			.filter_map(|(key_group, vector)| self.keys_states.any(key_group).then_some(vector))
			.reduce(|a, b| a + b);
		if let Some(movement) = movement {
			self.pos += 5000.0
				* (self.keys_states.any(self.action_map.boost) as u8 * 4 + 1) as f32
				* delta_time.as_secs_f32()
				* Mat4::from_rotation_y(self.yaw).transform_point3(movement);
			self.update_transform(window_size, queue);
		}
	}
}

impl Gui for TrTool {
	fn resize(&mut self, window_size: UVec2, device: &Device, queue: &Queue) {
		self.depth_view = make::depth_view(device, window_size);
		self.update_transform(window_size, queue);
	}
	
	fn modifiers(&mut self, modifers: ModifiersState) {
		self.modifiers = modifers;
	}
	
	fn key(
		&mut self, window_size: UVec2, device: &Device, queue: &Queue, target: &EventLoopWindowTarget<()>,
		keycode: KeyCode, state: ElementState, repeat: bool,
	) {
		_ = (window_size, device, queue);
		self.keys_states.set(keycode, state.is_pressed());
		match (self.modifiers, state, keycode, repeat) {
			(_, ElementState::Pressed, KeyCode::Escape, _) => target.exit(),
			(_, ElementState::Pressed, KeyCode::KeyP, _) => self.print = true,
			_ => {},
		}
	}
	
	fn mouse_button(&mut self, window: &Window, button: MouseButton, state: ElementState) {
		match (state, button) {
			(ElementState::Pressed, MouseButton::Right) => {
				if self.mouse_control {
					window.set_cursor_visible(true);
					window.set_cursor_grab(CursorGrabMode::None).expect("cursor ungrab");
				} else {
					window.set_cursor_visible(false);
					window
						.set_cursor_grab(CursorGrabMode::Confined)
						.or_else(|_| window.set_cursor_grab(CursorGrabMode::Locked))
						.expect("cursor grab");
				}
				self.mouse_control ^= true;
			},
			_ => {},
		}
	}
	
	fn mouse_moved(&mut self, window_size: UVec2, queue: &Queue, delta: DVec2) {
		if self.mouse_control {
			self.yaw += delta.x as f32 / 150.0;
			self.pitch = (self.pitch + delta.y as f32 / 150.0).clamp(-FRAC_PI_2, FRAC_PI_2);
			self.update_transform(window_size, queue);
		}
	}
	
	fn mouse_wheel(&mut self, queue: &Queue, delta: MouseScrollDelta) {
		_ = (queue, delta);
	}
	
	fn render(
		&mut self, window_size: UVec2, queue: &Queue, encoder: &mut CommandEncoder, color_view: &TextureView,
		delta_time: Duration, last_render_time: Duration,
	) {
		self.frame_update(window_size, queue, delta_time);
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
		rpass.set_bind_group(0, &self.bind_group, &[]);
		rpass.set_vertex_buffer(0, self.face_vertex_index_buffer.slice(..));
		
		rpass.set_pipeline(&self.textured_pipeline);
		rpass.set_vertex_buffer(1, self.face_buffers.textured_quads.buffer.slice(..));
		rpass.draw(0..NUM_QUAD_VERTS, 0..self.face_buffers.textured_quads.len);
		rpass.set_vertex_buffer(1, self.face_buffers.textured_tris.buffer.slice(..));
		rpass.draw(0..NUM_TRI_VERTS, 0..self.face_buffers.textured_tris.len);
		
		rpass.set_pipeline(&self.solid_pipeline);
		rpass.set_vertex_buffer(1, self.face_buffers.solid_quads.buffer.slice(..));
		rpass.draw(0..NUM_QUAD_VERTS, 0..self.face_buffers.solid_quads.len);
		rpass.set_vertex_buffer(1, self.face_buffers.solid_tris.buffer.slice(..));
		rpass.draw(0..NUM_TRI_VERTS, 0..self.face_buffers.solid_tris.len);
		
		if self.print {
			println!("render time: {}us", last_render_time.as_micros());
			self.print = false;
		}
	}
	
	fn gui(&mut self, window_size: UVec2, device: &Device, queue: &Queue, ctx: &egui::Context) {
		_ = (window_size, device, queue);
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

fn yaw_pitch(v: Vec3) -> (f32, f32) {
	((-v.x).atan2(-v.z), v.y.atan2(v.xz().length()))
}

fn room_pos(room: &tr1::Room) -> IVec3 {
	IVec3::new(room.x, 0, room.z)
}

fn get_rotation(rot: tr1::FrameRotation) -> Mat4 {
	let Vec3 { x, y, z } = rot.get_angles().as_vec3() / 1024.0 * TAU;
	Mat4::from_rotation_y(y) * Mat4::from_rotation_x(x) * Mat4::from_rotation_z(z)
}

fn make_pipeline(
	device: &Device, bind_group_layout: &BindGroupLayout, module: &ShaderModule, vs_entry: &str,
	fs_entry: &str,
) -> RenderPipeline {
	device.create_render_pipeline(
		&RenderPipelineDescriptor {
			label: None,
			layout: Some(&device.create_pipeline_layout(
				&PipelineLayoutDescriptor {
					label: None,
					bind_group_layouts: &[bind_group_layout],
					push_constant_ranges: &[],
				},
			)),
			vertex: VertexState {
				module,
				entry_point: vs_entry,
				buffers: &make::vertex_buffer_layouts(
					&mut vec![],
					&[
						(
							VertexStepMode::Vertex,
							&[
								VertexFormat::Uint32,
							],
						),
						(
							VertexStepMode::Instance,
							&[
								VertexFormat::Uint32x3,
							],
						),
					],
				),
			},
			primitive: PrimitiveState {
				topology: PrimitiveTopology::TriangleStrip,
				cull_mode: Some(Face::Back),
				front_face: FrontFace::Cw,
				strip_index_format: None,
				..PrimitiveState::default()//other fields require features
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
				entry_point: fs_entry,
				module,
				targets: &[
					Some(ColorTargetState {
						format: TextureFormat::Bgra8Unorm,
						blend: None,
						write_mask: ColorWrites::ALL,
					}),
				],
			}),
			multiview: None,
		},
	)
}

const PALETTE_SIZE: usize = tr1::PALETTE_LEN * size_of::<tr1::Color6Bit>();
const DATA_SIZE: usize = 1048576;
const QUAD_FACE_SIZE: u8 = 5;
const TRI_FACE_SIZE: u8 = 4;
const ROOM_VERTEX_SIZE: u8 = 4;
const MESH_VERTEX_SIZE: u8 = 3;

fn write_get_u16_offset(data: &mut DoubleEndBuffer, bytes: &[u8]) -> u32 {
	assert!(bytes.len() % 2 == 0, "write must be a multiple of 2");
	let offset = (data.start_pos() / 2) as u32;
	data.write_start(bytes);
	offset
}

fn write_face_array<const N: usize>(data: &mut DoubleEndBuffer, faces: &[tr1::Face<N>]) -> FaceArray {
	FaceArray {
		offset: write_get_u16_offset(data, faces.as_bytes()),
		len: faces.len() as u32,
	}
}

fn make_gui(window_size: UVec2, device: &Device, queue: &Queue) -> TrTool {
	//pipelines
	let bind_group_layout = make::bind_group_layout(
		device,
		&[
			(make::storage_layout_entry(DATA_SIZE), ShaderStages::VERTEX),//data 1MB
			(make::uniform_layout_entry(size_of::<Mat4>()), ShaderStages::VERTEX),//transform
			(make::uniform_layout_entry(PALETTE_SIZE), ShaderStages::FRAGMENT),//palette
			(
				BindingType::Texture {
					sample_type: TextureSampleType::Uint,
					view_dimension: TextureViewDimension::D2Array,
					multisampled: false,
				},
				ShaderStages::FRAGMENT,
			),//atlases
		],
	);
	let shader = shader!(device, "shader/mesh copy.wgsl");
	let [textured_pipeline, solid_pipeline] = [
		("textured_vs_main", "textured_fs_main"),
		("solid_vs_main", "solid_fs_main"),
	].map(|(vs_entry, fs_entry)| make_pipeline(device, &bind_group_layout, &shader, vs_entry, fs_entry));
	
	//read level
	let level_path = r"C:\Program Files (x86)\Steam\steamapps\common\Tomb Raider (I)\extracted\DATA\LEVEL1.PHD";
	let mut level_file = File::open(level_path).expect("open file");
	let mut level = MaybeUninit::<tr1::Level>::uninit();
	let level = unsafe {
		tr1::Level::read(&mut level_file, level.as_mut_ptr()).expect("read level");
		level.assume_init_ref()
	};
	
	//data
	let mut data = DoubleEndBuffer::new(DATA_SIZE);
	data.write_start(level.object_textures.as_bytes());
	let mut transform_index = 0;
	let mut faces = Faces::default();
	
	//add mesh faces to data, map tr mesh offets to meshes indices
	let mut meshes = vec![];
	let mut mesh_offset_map = HashMap::new();
	for &mesh_offset in level.mesh_offsets.iter() {
		mesh_offset_map.entry(mesh_offset).or_insert_with(|| {
			let mesh = level.get_mesh(mesh_offset);
			let index = meshes.len();
			meshes.push(Mesh {
				vertices_offset: write_get_u16_offset(&mut data, mesh.vertices.as_bytes()),
				textured_quads: write_face_array(&mut data, mesh.textured_quads),
				textured_tris: write_face_array(&mut data, mesh.textured_tris),
				solid_quads: write_face_array(&mut data, mesh.solid_quads),
				solid_tris: write_face_array(&mut data, mesh.solid_tris),
			});
			index
		});
	}
	
	//map static mesh ids to static mesh refs
	let mut static_mesh_id_map = HashMap::new();
	for static_mesh in level.static_meshes.iter() {
		static_mesh_id_map.insert(static_mesh.id as u16, static_mesh);
	}
	
	//rooms
	for room in level.rooms.iter() {
		let tr1::RoomGeom { vertices, quads, tris, .. } = room.get_geom_data();
		let vertices_offset = write_get_u16_offset(&mut data, vertices.as_bytes());
		for (face_list, bytes, num_faces, face_size) in [
			(&mut faces.textured_quads, quads.as_bytes(), quads.len(), QUAD_FACE_SIZE),
			(&mut faces.textured_tris, tris.as_bytes(), tris.len(), TRI_FACE_SIZE),
		] {
			let faces_offset = write_get_u16_offset(&mut data, bytes);
			for face_index in 0..num_faces as u32 {
				face_list.push(FaceInstance {
					face_offset: faces_offset + face_index * face_size as u32,
					vertices_offset,
					transform_index,
					face_size,
					vertex_size: ROOM_VERTEX_SIZE,
				});
			}
		}
		let transform = Mat4::from_translation(room_pos(room).as_vec3());
		data.write_end(transform.as_bytes());
		transform_index += 1;
		for room_static_mesh in room.room_static_meshes.iter() {
			let mesh = &meshes[
				mesh_offset_map[
					&level.mesh_offsets[
						static_mesh_id_map[&room_static_mesh.static_mesh_id].mesh_offset_index as usize
					]
				]
			];
			faces.add_mesh(mesh, transform_index);
			let transform = Mat4::from_translation({room_static_mesh.pos}.as_vec3())
				* Mat4::from_rotation_y(room_static_mesh.angle as f32 / 65536.0 * TAU);
			data.write_end(transform.as_bytes());
			transform_index += 1;
		}
	}
	
	//map model and sprite sequence ids to model and sprite sequence refs
	let mut model_id_map = HashMap::new();
	for model in level.models.iter() {
		model_id_map.insert(model.id as u16, ModelRef::Model(model));
	}
	for sprite_sequence in level.sprite_sequences.iter() {
		model_id_map.insert(sprite_sequence.id as u16, ModelRef::SpriteSequence(sprite_sequence));
	}
	
	//entities
	// for entity in level.entities.iter() {
	// 	match model_id_map[&entity.model_id] {
	// 		ModelRef::Model(model) => {
	// 			let entity_transform = Mat4::from_translation({entity.pos}.as_vec3())
	// 				* Mat4::from_rotation_y(entity.angle as f32 / 65536.0 * TAU);
	// 			let frame = level.get_frame(model.frame_byte_offset);
	// 			let mut last_transform = Mat4::from_translation(frame.offset.as_vec3())
	// 				* get_rotation(frame.rotations[0]);
				
	// 			let mesh_index = mesh_offset_map[&level.mesh_offsets[model.mesh_offset_index as usize]];
	// 			meshes.push(PlacedMesh { transform_bind_group, mesh_index });
				
	// 			data.write_end((entity_transform * last_transform).as_bytes());
	// 			transform_index += 1;
				
	// 			let mut parent_stack = vec![];
	// 			let mesh_nodes = level.get_mesh_nodes(model.mesh_node_offset, model.num_meshes - 1);
	// 			for mesh_node_index in 0..mesh_nodes.len() {
	// 				let mesh_node = &mesh_nodes[mesh_node_index];
	// 				let parent = if mesh_node.flags.pop() {
	// 					parent_stack.pop().expect("parent stack empty")
	// 				} else {
	// 					last_transform
	// 				};
	// 				if mesh_node.flags.push() {
	// 					parent_stack.push(parent);
	// 				}
	// 				last_transform = parent
	// 					* Mat4::from_translation(mesh_node.offset.as_vec3())
	// 					* get_rotation(frame.rotations[mesh_node_index + 1]);
	// 				let transform_bind_group = make::bind_group_single_uniform(
	// 					device, &mesh_transform_layout, (entity_transform * last_transform).as_bytes(),
	// 				);
	// 				let mesh_index = mesh_offset_map[
	// 					&level.mesh_offsets[model.mesh_offset_index as usize + mesh_node_index + 1]
	// 				];
	// 				meshes.push(PlacedMesh { transform_bind_group, mesh_index });
	// 			}
	// 			rooms[entity.room_index as usize].entities.push(Entity { meshes });
	// 		},
	// 		ModelRef::SpriteSequence(sprite_sequence) => _ = sprite_sequence,
	// 	}
	// }
	
	//level bind group
	let data = data.take_buffer();
	std::fs::write("data0", &data[..DATA_SIZE / 2]).unwrap();
	let data_buffer = make::buffer(device, &data, BufferUsages::STORAGE);
	let MinMax { min, max } = level
		.rooms[0]
		.get_geom_data()
		.vertices
		.iter()
		.map(|v| v.pos)
		.min_max()
		.unwrap();
	let (yaw, pitch) = yaw_pitch((max - min).as_vec3());
	let pos = (min.as_ivec3() + room_pos(&level.rooms[0])).as_vec3();
	let transform = get_transform(window_size, pos, yaw, pitch);
	let transform_buffer = make::buffer(
		device, transform.as_bytes(), BufferUsages::UNIFORM | BufferUsages::COPY_DST,
	);
	let palette_buffer = make::buffer(device, level.palette.as_bytes(), BufferUsages::UNIFORM);
	let atlases_texture = device.create_texture_with_data(
		queue,
		&TextureDescriptor {
			label: None,
			size: Extent3d {
				width: tr1::ATLAS_SIDE_LEN as u32,
				height: tr1::ATLAS_SIDE_LEN as u32,
				depth_or_array_layers: level.atlases.len() as u32,
			},
			mip_level_count: 1,
			sample_count: 1,
			dimension: TextureDimension::D2,
			format: TextureFormat::R8Uint,
			usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
			view_formats: &[],
		},
		TextureDataOrder::default(),
		level.atlases.as_bytes(),
	);
	let atlases_texture_view = atlases_texture.create_view(&TextureViewDescriptor::default());
	let bind_group = make::bind_group(
		device,
		&bind_group_layout,
		&[
			data_buffer.as_entire_binding(),
			transform_buffer.as_entire_binding(),
			palette_buffer.as_entire_binding(),
			BindingResource::TextureView(&atlases_texture_view),
		],
	);
	
	let action_map = ActionMap {
		forward: KeyGroup::new(&[KeyCode::KeyW, KeyCode::ArrowUp]),
		backward: KeyGroup::new(&[KeyCode::KeyS, KeyCode::ArrowDown]),
		left: KeyGroup::new(&[KeyCode::KeyA, KeyCode::ArrowLeft]),
		right: KeyGroup::new(&[KeyCode::KeyD, KeyCode::ArrowRight]),
		up: KeyGroup::new(&[KeyCode::KeyQ, KeyCode::PageUp]),
		down: KeyGroup::new(&[KeyCode::KeyE, KeyCode::PageDown]),
		boost: KeyGroup::new(&[KeyCode::ShiftLeft, KeyCode::ShiftRight]),
	};
	
	TrTool {
		modifiers: ModifiersState::empty(),
		_file_dialog: FileDialog::new().initial_directory(r"C:\Program Files (x86)\Steam\steamapps\common\Tomb Raider (I)\extracted\DATA".into()),
		error: None,
		depth_view: make::depth_view(device, window_size),
		face_vertex_index_buffer: make::buffer(device, FACE_VERT_INDICES.as_bytes(), BufferUsages::VERTEX),
		transform_buffer,
		textured_pipeline,
		solid_pipeline,
		bind_group,
		face_buffers: faces.into_buffers(device),
		mouse_control: false,
		pos,
		yaw,
		pitch,
		keys_states: KeyStates::new(),
		action_map,
		print: false,
	}
}

fn main() {
	// let mut data = vec![];
	// for entry in std::fs::read_dir(r"C:\Program Files (x86)\Steam\steamapps\common\Tomb Raider (I)\extracted\DATA").unwrap() {
	// 	let entry = entry.unwrap();
	// 	let path = entry.path();
	// 	if entry.file_type().unwrap().is_file() && path.extension().map_or(false, |e| e.eq_ignore_ascii_case("phd")) {
	// 		let mut level = MaybeUninit::<tr1::Level>::uninit();
	// 		let level = unsafe {
	// 			tr1::Level::read(&mut File::open(path).unwrap(), level.as_mut_ptr()).unwrap();
	// 			level.assume_init_ref()
	// 		};
	// 		let room_geom_data: usize = level.rooms.iter().map(|room| std::mem::size_of_val(&*room.geom_data)).sum();
	// 		data.push(
	// 			std::mem::size_of_val(&*level.object_textures) + std::mem::size_of_val(&*level.mesh_data) + room_geom_data
	// 		);
	// 	}
	// }
	// let a = data.into_iter().max().unwrap();
	// println!("max size: {:?}", a);
	// return;
	let window_icon = Icon::from_rgba(include_bytes!("res/icon16.data").to_vec(), 16, 16)
		.expect("window icon");
	let taskbar_icon = Icon::from_rgba(include_bytes!("res/icon24.data").to_vec(), 24, 24)
		.expect("taskbar icon");
	gui::run("TR Tool", window_icon, taskbar_icon, make_gui);
}
