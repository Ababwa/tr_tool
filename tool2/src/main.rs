mod as_bytes;
mod gui;
mod make;
mod keys;
// mod version_traits;
mod versions;
mod vec_tail;
mod geom_buffer;
mod face_buffer;
mod multi_cursor;
mod data_writer;

use std::{
	collections::HashMap, f32::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU}, fs::File,
	io::{BufReader, Error, Read, Result, Seek}, mem::{size_of, take, MaybeUninit}, ops::Range, path::PathBuf,
	sync::Arc, thread::{spawn, JoinHandle}, time::Duration,
};
use data_writer::{DataWriter, MeshFaceInstanceRanges, RoomFaceInstanceOffsets};
use geom_buffer::{GeomBuffer, GEOM_BUFFER_SIZE};
use face_buffer::{FaceType, FaceBuffer};
use keys::{KeyGroup, KeyStates};
use egui_file_dialog::{DialogState, FileDialog};
use as_bytes::AsBytes;
use glam::{DVec2, EulerRot, IVec4, Mat4, UVec2, Vec3, Vec3Swizzles};
use gui::Gui;
use versions::{converge, Level, TrVersion};
use shared::min_max::{MinMax, VecMinMaxFromIterator};
use tr_model::{tr1, tr2, tr3, Readable};
use wgpu::{
	util::{DeviceExt, TextureDataOrder}, BindGroup, BindGroupLayout, BindingResource, BindingType, Buffer,
	BufferDescriptor, BufferUsages, Color, ColorTargetState, ColorWrites, CommandEncoder,
	CommandEncoderDescriptor, CompareFunction, DepthBiasState, DepthStencilState, Device, Extent3d,
	FragmentState, FrontFace, ImageCopyBuffer, ImageDataLayout, IndexFormat, LoadOp, Maintain, MapMode,
	MultisampleState, Operations, PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology, Queue,
	RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor, RenderPipeline,
	RenderPipelineDescriptor, ShaderModule, ShaderStages, StencilState, StoreOp, Texture, TextureDescriptor,
	TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureView, TextureViewDescriptor,
	TextureViewDimension, VertexFormat, VertexState, VertexStepMode,
};
use winit::{
	event::{ElementState, MouseButton, MouseScrollDelta}, event_loop::EventLoopWindowTarget,
	keyboard::{KeyCode, ModifiersState}, window::{CursorGrabMode, Icon, Window},
};

const WINDOW_TITLE: &str = "TR Tool";
const PALETTE_SIZE: usize = size_of::<[tr1::Color24Bit; tr1::PALETTE_LEN]>();

/*
This ordering creates a "Z" so triangle strip mode may be used for quads, and the first three indices used
for tris.
*/
const FACE_VERTEX_INDICES: [u32; 4] = [1, 2, 0, 3];
const FLIPPED_INDICES: [u16; 4] = [0, 2, 1, 3];//yields face vertex indices [1, 0, 2, 3]
const NUM_QUAD_VERTICES: u32 = 4;
const NUM_TRI_VERTICES: u32 = 3;

const FORWARD: Vec3 = Vec3::NEG_Z;
const BACKWARD: Vec3 = Vec3::Z;
const LEFT: Vec3 = Vec3::X;
const RIGHT: Vec3 = Vec3::NEG_X;
const DOWN: Vec3 = Vec3::Y;
const UP: Vec3 = Vec3::NEG_Y;

struct WrittenFaceArray {
	index: usize,
	len: usize,
}

struct WrittenMesh {
	textured_quads: WrittenFaceArray,
	textured_tris: WrittenFaceArray,
	solid_quads: WrittenFaceArray,
	solid_tris: WrittenFaceArray,
}

struct RoomData {
	quads: RoomFaceInstanceOffsets,
	tris: RoomFaceInstanceOffsets,
	meshes: Vec<MeshFaceInstanceRanges>,
	sprites: Range<u32>,
	center: Vec3,
	radius: f32,
}

/// `[original_room_index, alt_room_index]`
struct RenderRoom([usize; 2]);

#[derive(Clone, Copy, Debug)]
enum RoomFaceType {
	Quad,
	Tri,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
enum ObjectData {
	RoomFace {
		room_index: u16,
		face_type: RoomFaceType,
		face_index: u16,
	},
	RoomStaticMeshFace {
		room_index: u16,
		room_static_mesh_index: u16,
		face_type: FaceType,
		face_index: u16,
	},
	RoomSprite {
		room_index: u16,
		sprite_index: u16,
	},
	EntityMeshFace {
		entity_index: u16,
		mesh_index: u16,
		face_type: FaceType,
		face_index: u16,
	},
	EntitySprite {
		entity_index: u16,
	},
	Flipped {
		object_data_index: u16,
	},
}

impl ObjectData {
	fn room_face(room_index: usize, face_type: RoomFaceType, face_index: usize) -> Self {
		Self::RoomFace { room_index: room_index as u16, face_type, face_index: face_index as u16 }
	}
	
	fn room_static_mesh_face(
		room_index: usize, room_static_mesh_index: usize, face_type: FaceType, face_index: usize,
	) -> Self {
		Self::RoomStaticMeshFace {
			room_index: room_index as u16, room_static_mesh_index: room_static_mesh_index as u16, face_type,
			face_index: face_index as u16,
		}
	}
	
	fn room_sprite(room_index: usize, sprite_index: usize) -> Self {
		Self::RoomSprite { room_index: room_index as u16, sprite_index: sprite_index as u16 }
	}
	
	fn entity_mesh_face(
		entity_index: usize, mesh_index: usize, face_type: FaceType, face_index: usize,
	) -> Self {
		Self::EntityMeshFace {
			entity_index: entity_index as u16, mesh_index: mesh_index as u16, face_type,
			face_index: face_index as u16,
		}
	}
	
	fn entity_sprite(entity_index: usize) -> Self {
		Self::EntitySprite { entity_index: entity_index as u16 }
	}
	
	fn flipped(object_data_index: usize) -> Self {
		Self::Flipped { object_data_index: object_data_index as u16 }
	}
}

fn print_object_data(level: &Level, object_data: &[ObjectData], index: usize) {
	println!("object data index: {}", index);
	let data = object_data[index];
	println!("{:?}", data);
	let data = match object_data[index] {
		ObjectData::Flipped { object_data_index } => {
			let data = object_data[object_data_index as usize];
			println!("{:?}", data);
			data
		},
		data => data,
	};
	let mesh_face = match data {
		ObjectData::RoomFace { room_index, face_type, face_index } => {
			let room_geom = level.rooms().get(room_index as usize).get_geom();
			let texture = match face_type {
				RoomFaceType::Quad => match room_geom.quads().get(face_index as usize) {
					TrVersion::Tr1(quad) | TrVersion::Tr2(quad) => quad.object_texture_index,
					TrVersion::Tr3(quad) => quad.texture.0,
				},
				RoomFaceType::Tri => match room_geom.tris().get(face_index as usize) {
					TrVersion::Tr1(tri) | TrVersion::Tr2(tri) => tri.object_texture_index,
					TrVersion::Tr3(tri) => tri.texture.0,
				},
			};
			println!("double sided: {}", texture & 0x8000 != 0);
			let object_texture_index = texture & 0x7FFF;
			let object_texture = &level.object_textures()[object_texture_index as usize];
			println!("blend mode: {}", object_texture.blend_mode);
			None
		},
		ObjectData::RoomStaticMeshFace { room_index, room_static_mesh_index, face_type, face_index } => {
			let room = level.rooms().get(room_index as usize);
			let room_static_mesh = room.room_static_meshes().get(room_static_mesh_index as usize);
			let static_mesh_id = room_static_mesh.static_mesh_id();
			let static_mesh = level
				.static_meshes()
				.iter()
				.find(|static_mesh| static_mesh.id as u16 == static_mesh_id)
				.unwrap();
			let mesh_offset = level.mesh_offsets()[static_mesh.mesh_offset_index as usize];
			Some((mesh_offset, face_type, face_index))
		},
		ObjectData::RoomSprite { .. } => None,
		ObjectData::EntityMeshFace { entity_index, mesh_index, face_type, face_index } => {
			let model_id = level.entities().get(entity_index as usize).model_id();
			let model = level.models().iter().find(|model| model.id as u16 == model_id).unwrap();
			let mesh_offset = level.mesh_offsets()[(model.mesh_offset_index + mesh_index) as usize];
			Some((mesh_offset, face_type, face_index))
		},
		ObjectData::EntitySprite { .. } => None,
		ObjectData::Flipped { .. } => unreachable!("flipped points to flipped"),
	};
	if let Some((mesh_offset, face_type, face_index)) = mesh_face {
		println!("mesh offset: {}", mesh_offset);
		let mesh = level.get_mesh(mesh_offset);
		let face_texture = match face_type {
			FaceType::TexturedQuad => converge!(
				mesh.textured_quads().get(face_index as usize),
				|quad| (Some(quad.object_texture_index), None, None),
			),
			FaceType::TexturedTri => converge!(
				mesh.textured_tris().get(face_index as usize),
				|tri| (Some(tri.object_texture_index), None, None),
			),
			FaceType::SolidQuad => match mesh.solid_quads().get(face_index as usize) {
				TrVersion::Tr1(quad) => (None, Some(quad.color_index), None),
				TrVersion::Tr2(quad) | TrVersion::Tr3(quad) => {
					(None, Some(quad.color_index_24bit as u16), Some(quad.color_index_32bit))
				},
			},
			FaceType::SolidTri => match mesh.solid_tris().get(face_index as usize) {
				TrVersion::Tr1(tri) => (None, Some(tri.color_index), None),
				TrVersion::Tr2(tri) | TrVersion::Tr3(tri) => {
					(None, Some(tri.color_index_24bit as u16), Some(tri.color_index_32bit))
				},
			},
		};
		if let (Some(object_texture_index), ..) = face_texture {
			let object_texture = &level.object_textures()[object_texture_index as usize];
			println!("blend mode: {}", object_texture.blend_mode);
		}
		if let (_, Some(color_index_24bit), _) = face_texture {
			let tr1::Color24Bit { r, g, b } = level.palette_24bit()[color_index_24bit as usize];
			let [r, g, b] = [r, g, b].map(|c| (c << 2) as u32);
			let color = (r << 16) | (g << 8) | b;
			println!("color 24 bit: #{:06X}", color);
		}
		if let (.., Some(color_index_32bit)) = face_texture {
			let palette_32bit = match level {
				TrVersion::Tr1(_) => unreachable!(),
				TrVersion::Tr2(level) => &level.palette_32bit,
				TrVersion::Tr3(level) => &level.palette_32bit,
			} as &[_; 256];
			let tr2::Color32Bit { r, g, b , ..} = palette_32bit[color_index_32bit as usize];
			let [r, g, b] = [r, g, b].map(|c| c as u32);
			let color = (r << 16) | (g << 8) | b;
			println!("color 32 bit: #{:06X}", color);
		}
	}
}

struct ActionMap {
	forward: KeyGroup,
	backward: KeyGroup,
	left: KeyGroup,
	right: KeyGroup,
	up: KeyGroup,
	down: KeyGroup,
	boost: KeyGroup,
}

struct LoadedLevel {
	//constant
	face_vertex_index_buffer: Buffer,
	flipped_indices_buffer: Buffer,
	
	//render
	depth_view: TextureView,
	interact_texture: Texture,
	interact_view: TextureView,
	camera_transform_buffer: Buffer,
	perspective_transform_buffer: Buffer,
	face_instance_buffer: Buffer,
	sprite_instance_buffer: Buffer,
	bind_group: BindGroup,
	
	//camera
	pos: Vec3,
	yaw: f32,
	pitch: f32,
	
	//rooms
	rooms: Vec<RoomData>,
	render_rooms: Vec<RenderRoom>,
	render_room_index: Option<usize>,//if None, render all
	render_alt_rooms: bool,
	
	//object data
	object_data: Vec<ObjectData>,
	click_handle: Option<JoinHandle<u32>>,
	level: Level,
	
	//input state
	mouse_pos: DVec2,
	mouse_control: bool,
	key_states: KeyStates,
	action_map: ActionMap,
	
	frame_update_queue: Vec<Box<dyn FnOnce(&mut Self)>>,
}

struct TrTool {
	//static
	bind_group_layout: BindGroupLayout,
	textured_pipeline: RenderPipeline,
	solid_pipeline: RenderPipeline,
	sprite_pipeline: RenderPipeline,
	
	//state
	window_size: UVec2,
	modifiers: ModifiersState,
	file_dialog: FileDialog,
	error: Option<String>,
	print: bool,
	loaded_level: Option<LoadedLevel>,
}

#[derive(Clone, Copy)]
enum ModelRef<'a> {
	Model(&'a tr1::Model),
	SpriteSequence(&'a tr1::SpriteSequence),
}

fn make_camera_transform(pos: Vec3, yaw: f32, pitch: f32) -> Mat4 {
	Mat4::from_euler(EulerRot::XYZ, pitch, yaw, PI) * Mat4::from_translation(-pos)
}

fn make_perspective_transform(window_size: UVec2) -> Mat4 {
	Mat4::perspective_rh(FRAC_PI_4, window_size.x as f32 / window_size.y as f32, 100.0, 100000.0)
}

impl LoadedLevel {
	fn set_mouse_control(&mut self, window: &Window, mouse_control: bool) {
		match (self.mouse_control, mouse_control) {
			(true, false) => {
				window.set_cursor_visible(true);
				window.set_cursor_grab(CursorGrabMode::None).expect("cursor ungrab");
			},
			(false, true) => {
				window.set_cursor_visible(false);
				window
					.set_cursor_grab(CursorGrabMode::Confined)
					.or_else(|_| window.set_cursor_grab(CursorGrabMode::Locked))
					.expect("cursor grab");
			},
			_ => {},
		}
		self.mouse_control = mouse_control;
	}
	
	fn update_camera_transform(&self, queue: &Queue) {
		let camera_transform = make_camera_transform(self.pos, self.yaw, self.pitch);
		queue.write_buffer(&self.camera_transform_buffer, 0, camera_transform.as_bytes());
	}
	
	fn update_perspective_transform(&self, queue: &Queue, window_size: UVec2) {
		let perspective_transform = make_perspective_transform(window_size);
		queue.write_buffer(&self.perspective_transform_buffer, 0, perspective_transform.as_bytes());
	}
	
	fn frame_update(&mut self, queue: &Queue, delta_time: Duration) {
		if let Some(click_handle) = self.click_handle.take() {
			if click_handle.is_finished() {
				let object_id = click_handle.join().expect("join click handle");
				if object_id != 0 {
					print_object_data(&self.level, &self.object_data, object_id as usize);
				}
			} else {
				self.click_handle = Some(click_handle);
			}
		}
		for update_fn in take(&mut self.frame_update_queue)  {
			update_fn(self);
		}
		let movement = [
			(self.action_map.forward, FORWARD),
			(self.action_map.backward, BACKWARD),
			(self.action_map.left, LEFT),
			(self.action_map.right, RIGHT),
			(self.action_map.up, UP),
			(self.action_map.down, DOWN),
		];
		let movement = movement
			.into_iter()
			.filter_map(|(key_group, vector)| self.key_states.any(key_group).then_some(vector))
			.reduce(|a, b| a + b);
		if let Some(movement) = movement {
			self.pos += 5000.0
				* (self.key_states.any(self.action_map.boost) as u8 * 4 + 1) as f32
				* delta_time.as_secs_f32()
				* Mat4::from_rotation_y(self.yaw).transform_point3(movement);
		}
		self.update_camera_transform(queue);
	}
}

fn yaw_pitch(v: Vec3) -> (f32, f32) {
	((-v.x).atan2(-v.z), v.y.atan2(v.xz().length()))
}

fn direction(yaw: f32, pitch: f32) -> Vec3 {
	let (yaw_sin, yaw_cos) = yaw.sin_cos();
	let (pitch_sin, pitch_cos) = pitch.sin_cos();
	Vec3::new(-pitch_cos * yaw_sin, pitch_sin, -pitch_cos * yaw_cos)
}

fn make_interact_texture(device: &Device, window_size: UVec2) -> Texture {
	make::texture(
		device, window_size, TextureFormat::R32Uint,
		TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
	)
}

fn parse_level(
	device: &Device, queue: &Queue, bind_group_layout: &BindGroupLayout, window_size: UVec2, level: Level,
) -> LoadedLevel {
	//map model and sprite sequence ids to model and sprite sequence refs
	let model_id_map = level
		.models()
		.iter()
		.map(|model| (model.id as u16, ModelRef::Model(model)))
		.chain(level.sprite_sequences().iter().map(|ss| (ss.id as u16, ModelRef::SpriteSequence(ss))))
		.collect::<HashMap<_, _>>();
	
	//group entities by room
	let mut room_entity_indices = vec![vec![]; level.rooms().len()];
	for (entity_index, entity) in level.entities().iter().enumerate() {
		room_entity_indices[entity.room_index() as usize].push(entity_index);
	}
	
	//geom
	let mut geom_buffer = GeomBuffer::new();
	geom_buffer.write_object_textures(level.object_textures());
	geom_buffer.write_sprite_textures(level.sprite_textures());
	
	//write meshes to data, map tr mesh offets to meshes indices
	let mut written_meshes = vec![];
	let mut mesh_offset_map = HashMap::new();
	for &mesh_offset in level.mesh_offsets().iter() {
		mesh_offset_map.entry(mesh_offset).or_insert_with(|| {
			let mesh = level.get_mesh(mesh_offset);
			let vao = geom_buffer.write_vertex_array(mesh.vertices());
			let index = written_meshes.len();
			written_meshes.push(WrittenMesh {
				textured_quads: converge!(mesh.textured_quads(), |f| geom_buffer.write_face_array(f, vao)),
				textured_tris: converge!(mesh.textured_tris(), |f| geom_buffer.write_face_array(f, vao)),
				solid_quads: converge!(mesh.solid_quads(), |f| geom_buffer.write_face_array(f, vao)),
				solid_tris: converge!(mesh.solid_tris(), |f| geom_buffer.write_face_array(f, vao)),
			});
			index
		});
	}
	
	//rooms
	let mut data_writer = DataWriter {
		geom_buffer, face_buffer: FaceBuffer::new(),
		object_data: vec![ObjectData::entity_sprite(usize::MAX)],//dummy value at index 0
	};
	let mut sprite_instances = Vec::<IVec4>::new();
	let mut room_indices = (0..level.rooms().len()).collect::<Vec<_>>();//rooms with alt will be removed
	let mut render_rooms = Vec::with_capacity(level.rooms().len());//rooms.len is upper-bound
	
	let rooms = level.rooms().iter().enumerate().map(|(room_index, room)| {
		let room_pos = room.pos();
		let room_geom = room.get_geom();
		let sprites_start = sprite_instances.len() as u32;
		let mut meshes = vec![];
		
		//room geom
		let (quads, tris) = {
			let vertex_array_offset = converge!(
				room_geom.vertices(), |vertices| data_writer.geom_buffer.write_vertex_array(vertices),
			);
			let transform = Mat4::from_translation(room_pos.as_vec3());
			let transform_index = data_writer.geom_buffer.write_transform(&transform);
			let quad_obj_fn = |face_index| ObjectData::room_face(room_index, RoomFaceType::Quad, face_index);
			let quads = converge!(
				room_geom.quads(), |quads| data_writer.write_room_face_array(
					vertex_array_offset, FaceType::TexturedQuad, quads, transform_index, quad_obj_fn,
				),
			);
			let tri_obj_fn = |face_index| ObjectData::room_face(room_index, RoomFaceType::Tri, face_index);
			let tris = converge!(
				room_geom.tris(), |tris| data_writer.write_room_face_array(
					vertex_array_offset, FaceType::TexturedTri, tris, transform_index, tri_obj_fn,
				),
			);
			(quads, tris)
		};
		
		//static meshes
		for (room_static_mesh_index, room_static_mesh) in room.room_static_meshes().iter().enumerate() {
			let static_mesh_id = room_static_mesh.static_mesh_id();
			let maybe_static_mesh = level
				.static_meshes()
				.iter()
				.find(|static_mesh| static_mesh.id as u16 == static_mesh_id);
			let static_mesh = match maybe_static_mesh {
				Some(static_mesh) => static_mesh,
				None => {
					println!("static mesh id missing: {}", static_mesh_id);
					continue;
				}
			};
			let mesh_offset = level.mesh_offsets()[static_mesh.mesh_offset_index as usize];
			let written_mesh = &written_meshes[mesh_offset_map[&mesh_offset]];
			let translation = Mat4::from_translation(room_static_mesh.pos().as_vec3());
			let rotation = Mat4::from_rotation_y(room_static_mesh.angle() as f32 / 65536.0 * TAU);
			let transform = translation * rotation;
			let transform_index = data_writer.geom_buffer.write_transform(&transform);
			let obj_fn = |face_type, face_index| {
				ObjectData::room_static_mesh_face(room_index, room_static_mesh_index, face_type, face_index)
			};
			meshes.push(data_writer.instantiate_mesh(written_mesh, transform_index, obj_fn));
		}
		
		//room sprites
		for (sprite_index, sprite) in room_geom.sprites().iter().enumerate() {
			let pos = room_pos + room_geom.vertices().get(sprite.vertex_index as usize).pos().as_ivec3();
			sprite_instances.push(pos.extend(sprite.sprite_texture_index as i32));
			data_writer.object_data.push(ObjectData::room_sprite(room_index, sprite_index));
		}
		
		//entities
		for &entity_index in &room_entity_indices[room_index] {
			let entity = level.entities().get(entity_index);
			match model_id_map[&entity.model_id()] {
				ModelRef::Model(model) => {
					let entity_transform = Mat4::from_translation(entity.pos().as_vec3())
						* Mat4::from_rotation_y(entity.angle() as f32 / 65536.0 * TAU);
					let frame = level.get_frame(model);
					let mut rotations = frame.iter_rotations();
					let mut last_transform = Mat4::from_translation(frame.offset().as_vec3())
						* rotations.next().unwrap();
					let transform = entity_transform * last_transform;
					let transform_index = data_writer.geom_buffer.write_transform(&transform);
					let mesh_offset = level.mesh_offsets()[model.mesh_offset_index as usize];
					let mesh = &written_meshes[mesh_offset_map[&mesh_offset]];
					let obj_fn = |face_type, face_index| {
						ObjectData::entity_mesh_face(entity_index, 0, face_type, face_index)
					};
					meshes.push(data_writer.instantiate_mesh(mesh, transform_index, obj_fn));
					let mut parent_stack = vec![];
					let mesh_nodes = level.get_mesh_nodes(model);
					for mesh_node_index in 0..mesh_nodes.len() {
						let mesh_node = &mesh_nodes[mesh_node_index];
						let parent = if mesh_node.flags.pop() {
							parent_stack.pop().expect("mesh transform stack empty")
						} else {
							last_transform
						};
						if mesh_node.flags.push() {
							parent_stack.push(parent);
						}
						let mesh_offset_index = model.mesh_offset_index as usize + mesh_node_index + 1;
						let mesh_offset = level.mesh_offsets()[mesh_offset_index];
						let mesh = &written_meshes[mesh_offset_map[&mesh_offset]];
						last_transform = parent
							* Mat4::from_translation(mesh_node.offset.as_vec3())
							* rotations.next().unwrap();
						let transform = entity_transform * last_transform;
						let transform_index = data_writer.geom_buffer.write_transform(&transform);
						let obj_fn = |face_type, face_index| {
							ObjectData::entity_mesh_face(
								entity_index, mesh_node_index + 1, face_type, face_index,
							)
						};
						meshes.push(data_writer.instantiate_mesh(mesh, transform_index, obj_fn));
					}
				},
				ModelRef::SpriteSequence(sprite_sequence) => {
					sprite_instances.push(entity.pos().extend(sprite_sequence.sprite_texture_index as i32));
					data_writer.object_data.push(ObjectData::entity_sprite(entity_index));
				},
			}
		}
		
		let sprites_end = sprite_instances.len() as u32;
		
		let (center, radius) = room_geom.vertices()
			.iter()
			.map(|v| v.pos())
			.min_max()
			.map(|MinMax { min, max }| {
				let center = (max.as_vec3() + min.as_vec3()) / 2.0;
				let radius = (max - min).max_element() as f32;
				(center, radius)
			})
			.unwrap_or_default();
		let center = center + room_pos.as_vec3();
		
		if room.alt_room_index() != u16::MAX {
			let alt_room_index = room.alt_room_index() as usize;
			room_indices.remove(room_indices.binary_search(&room_index).unwrap());
			room_indices.remove(room_indices.binary_search(&alt_room_index).expect("alt room index"));
			render_rooms.push(RenderRoom([room_index, alt_room_index]));
		}
		
		RoomData { quads, tris, meshes, sprites: sprites_start..sprites_end, center, radius, }
	}).collect::<Vec<_>>();
	
	let DataWriter { geom_buffer, face_buffer, object_data } = data_writer;
	
	//remaining room indices have no alt
	for room_index in room_indices {
		render_rooms.push(RenderRoom([room_index; 2]));
	}
	render_rooms.sort_by_key(|rr| rr.0[0]);
	
	//level bind group
	let data_buffer = make::buffer(device, &geom_buffer.into_buffer(), BufferUsages::STORAGE);
	let (yaw, pitch) = yaw_pitch(Vec3::ONE);
	let pos = rooms
		.get(0)
		.map(|&RoomData { center, radius, .. }| center - direction(yaw, pitch) * radius)
		.unwrap_or_default();
	let camera_transform = make_camera_transform(pos, yaw, pitch);
	let camera_transform_buffer = make::buffer(
		device, camera_transform.as_bytes(), BufferUsages::UNIFORM | BufferUsages::COPY_DST,
	);
	let perspective_transform = make_perspective_transform(window_size);
	let perspective_transform_buffer = make::buffer(
		device, perspective_transform.as_bytes(), BufferUsages::UNIFORM | BufferUsages::COPY_DST,
	);
	let palette_buffer = make::buffer(device, level.palette_24bit().as_bytes(), BufferUsages::UNIFORM);
	let atlases_texture = device.create_texture_with_data(
		queue,
		&TextureDescriptor {
			label: None,
			size: Extent3d {
				width: tr1::ATLAS_SIDE_LEN as u32,
				height: tr1::ATLAS_SIDE_LEN as u32,
				depth_or_array_layers: level.atlases().len() as u32,
			},
			mip_level_count: 1,
			sample_count: 1,
			dimension: TextureDimension::D2,
			format: TextureFormat::R8Uint,
			usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
			view_formats: &[],
		},
		TextureDataOrder::default(),
		level.atlases().as_bytes(),
	);
	let atlases_texture_view = atlases_texture.create_view(&TextureViewDescriptor::default());
	let bind_group = make::bind_group(
		device,
		&bind_group_layout,
		&[
			data_buffer.as_entire_binding(),
			camera_transform_buffer.as_entire_binding(),
			perspective_transform_buffer.as_entire_binding(),
			palette_buffer.as_entire_binding(),
			BindingResource::TextureView(&atlases_texture_view),
		],
	);
	
	let num_sprite_instances = sprite_instances.len() as u32;
	println!("sprite instances: {}", num_sprite_instances);
	
	let action_map = ActionMap {
		forward: KeyGroup::new(&[KeyCode::KeyW, KeyCode::ArrowUp]),
		backward: KeyGroup::new(&[KeyCode::KeyS, KeyCode::ArrowDown]),
		left: KeyGroup::new(&[KeyCode::KeyA, KeyCode::ArrowLeft]),
		right: KeyGroup::new(&[KeyCode::KeyD, KeyCode::ArrowRight]),
		up: KeyGroup::new(&[KeyCode::KeyQ, KeyCode::PageUp]),
		down: KeyGroup::new(&[KeyCode::KeyE, KeyCode::PageDown]),
		boost: KeyGroup::new(&[KeyCode::ShiftLeft, KeyCode::ShiftRight]),
	};
	
	let interact_texture = make_interact_texture(device, window_size);
	let interact_view = interact_texture.create_view(&TextureViewDescriptor::default());
	
	LoadedLevel {
		face_vertex_index_buffer: make::buffer(device, FACE_VERTEX_INDICES.as_bytes(), BufferUsages::VERTEX),
		flipped_indices_buffer: make::buffer(device, FLIPPED_INDICES.as_bytes(), BufferUsages::INDEX),
		depth_view: make::depth_view(device, window_size),
		interact_texture,
		interact_view,
		camera_transform_buffer,
		perspective_transform_buffer,
		face_instance_buffer: make::buffer(device, &face_buffer.into_buffer(), BufferUsages::VERTEX),
		sprite_instance_buffer: make::buffer(device, sprite_instances.as_bytes(), BufferUsages::VERTEX),
		bind_group,
		pos,
		yaw,
		pitch,
		rooms,
		render_rooms,
		render_room_index: None,
		render_alt_rooms: false,
		object_data,
		click_handle: None,
		level,
		mouse_pos: DVec2::ZERO,
		mouse_control: false,
		key_states: KeyStates::new(),
		action_map,
		frame_update_queue: vec![],
	}
}

fn read_level<R: Read, L: Readable>(reader: &mut R) -> Result<L> {
	unsafe {
		let mut level = MaybeUninit::uninit();
		L::read(reader, level.as_mut_ptr())?;
		Ok(level.assume_init())
	}
}

fn load_level(
	device: &Device, queue: &Queue, bind_group_layout: &BindGroupLayout, window_size: UVec2, path: &PathBuf,
) -> Result<LoadedLevel> {
	let mut reader = BufReader::new(File::open(path)?);
	let mut version = [0; 4];
	reader.read_exact(&mut version)?;
	reader.rewind()?;
	let version = u32::from_le_bytes(version);
	let extension = path.extension().map(|e| e.to_string_lossy());
	let level = match (version, extension.as_ref().map(|e| e.as_ref())) {
		(0x00000020, _) => Level::Tr1(read_level::<_, tr1::Level>(&mut reader)?),
		(0x0000002D, _) => Level::Tr2(read_level::<_, tr2::Level>(&mut reader)?),
		(0xFF180038, _) => Level::Tr3(read_level::<_, tr3::Level>(&mut reader)?),
		_ => return Err(Error::other("unknown file type")),
	};
	Ok(parse_level(device, queue, bind_group_layout, window_size, level))
}

fn draw_window<R, F: FnOnce(&mut egui::Ui) -> R>(ctx: &egui::Context, title: &str, contents: F) -> R {
	egui::Window::new(title).collapsible(false).resizable(false).show(ctx, contents).unwrap().inner.unwrap()
}

fn selected_room_text(render_room_index: Option<usize>) -> String {
	match render_room_index {
		Some(render_room_index) => format!("Room {}", render_room_index),
		None => "All".to_string(),
	}
}

fn render_options(loaded_level: &mut LoadedLevel, ui: &mut egui::Ui) {
	ui.checkbox(&mut loaded_level.render_alt_rooms, "Alternate Rooms");
	let old_render_room = loaded_level.render_room_index;
	let combo = egui::ComboBox::from_label("Room");
	combo.selected_text(selected_room_text(loaded_level.render_room_index)).show_ui(ui, |ui| {
		ui.selectable_value(&mut loaded_level.render_room_index, None, selected_room_text(None));
		for render_room_index in 0..loaded_level.render_rooms.len() {
			ui.selectable_value(
				&mut loaded_level.render_room_index,
				Some(render_room_index),
				selected_room_text(Some(render_room_index)),
			);
		}
	});
	if let (true, Some(render_room_index)) = (
		loaded_level.render_room_index != old_render_room, loaded_level.render_room_index,
	) {
		let RoomData { center, radius, .. } = loaded_level.rooms[
			loaded_level.render_rooms[render_room_index].0[loaded_level.render_alt_rooms as usize]
		];
		loaded_level.frame_update_queue.push(Box::new(move |loaded_level| {
			loaded_level.pos = center - (direction(loaded_level.yaw, loaded_level.pitch) * radius);
		}));
	}
}

impl Gui for TrTool {
	fn resize(&mut self, window_size: UVec2, device: &Device, queue: &Queue) {
		self.window_size = window_size;
		if let Some(loaded_level) = &mut self.loaded_level {
			loaded_level.depth_view = make::depth_view(device, window_size);
			loaded_level.interact_texture = make_interact_texture(device, window_size);
			loaded_level.interact_view = loaded_level.interact_texture.create_view(
				&TextureViewDescriptor::default(),
			);
			loaded_level.update_perspective_transform(queue, window_size);
		}
	}
	
	fn modifiers(&mut self, modifers: ModifiersState) {
		self.modifiers = modifers;
	}
	
	fn key(
		&mut self, window: &Window, device: &Device, queue: &Queue, target: &EventLoopWindowTarget<()>,
		key_code: KeyCode, state: ElementState, repeat: bool,
	) {
		_ = (device, queue);
		if let Some(loaded_level) = &mut self.loaded_level {
			loaded_level.key_states.set(key_code, state.is_pressed());
		}
		match (self.modifiers, state, key_code, repeat) {
			(_, ElementState::Pressed, KeyCode::Escape, false) => target.exit(),
			(_, ElementState::Pressed, KeyCode::KeyP, _) => self.print = true,
			(ModifiersState::CONTROL, ElementState::Pressed, KeyCode::KeyO, false) => {
				if let Some(loaded_level) = &mut self.loaded_level {
					loaded_level.set_mouse_control(window, false);
				}
				self.file_dialog.select_file();
			},
			_ => {},
		}
	}
	
	fn mouse_button(
		&mut self, window: &Window, device: Arc<Device>, queue: &Queue, button: MouseButton,
		state: ElementState,
	) {
		if let Some(loaded_level) = &mut self.loaded_level {
			match (state, button) {
				(ElementState::Pressed, MouseButton::Right) => {
					if !matches!(self.file_dialog.state(), DialogState::Open) {
						loaded_level.set_mouse_control(window, !loaded_level.mouse_control);
					}
				},
				(ElementState::Pressed, MouseButton::Left) => {
					let width = ((loaded_level.interact_texture.width() + 63) / 64) * 64;
					let height = loaded_level.interact_texture.height();
					let buffer = device.create_buffer(&BufferDescriptor {
						label: None,
						size: (width * height * 4) as u64,
						usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
						mapped_at_creation: false,
					});
					let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());
					encoder.copy_texture_to_buffer(
						loaded_level.interact_texture.as_image_copy(),
						ImageCopyBuffer {
							buffer: &buffer,
							layout: ImageDataLayout {
								offset: 0,
								bytes_per_row: Some(width * 4),
								rows_per_image: None,
							},
						},
						loaded_level.interact_texture.size(),
					);
					let submission_index = queue.submit([encoder.finish()]);
					buffer.slice(..).map_async(MapMode::Read, |r| r.expect("map interact texture"));
					let pos = loaded_level.mouse_pos.as_uvec2();
					let click_handle = spawn(move || {
						device.poll(Maintain::WaitForSubmissionIndex(submission_index));
						let bytes = &*buffer.slice(..).get_mapped_range();
						let pixel_offset = pos.y * width + pos.x;
						let b = pixel_offset as usize * 4;
						u32::from_le_bytes([bytes[b], bytes[b + 1], bytes[b + 2], bytes[b + 3]])
					});
					loaded_level.click_handle = Some(click_handle);
				},
				_ => {},
			}
		}
	}
	
	fn mouse_motion(&mut self, delta: DVec2) {
		if let Some(loaded_level) = &mut self.loaded_level {
			if loaded_level.mouse_control {
				loaded_level.yaw += delta.x as f32 / 150.0;
				loaded_level.pitch = (loaded_level.pitch + delta.y as f32 / 150.0)
					.clamp(-FRAC_PI_2, FRAC_PI_2);
			}
		}
	}
	
	fn cursor_moved(&mut self, pos: DVec2) {
		if let Some(loaded_level) = &mut self.loaded_level {
			loaded_level.mouse_pos = pos;
		}
	}
	
	fn mouse_wheel(&mut self, _: MouseScrollDelta) {}
	
	fn render(
		&mut self, device: &Device, queue: &Queue, encoder: &mut CommandEncoder, color_view: &TextureView,
		delta_time: Duration, last_render_time: Duration,
	) {
		_ = device;
		if let Some(loaded_level) = &mut self.loaded_level {
			loaded_level.frame_update(queue, delta_time);
			
			let room_version = loaded_level.render_alt_rooms as usize;
			let render_room_range = match loaded_level.render_room_index {
				Some(render_room_index) => render_room_index..render_room_index + 1,
				None => 0..loaded_level.render_rooms.len(),
			};
			
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
					Some(RenderPassColorAttachment {
						ops: Operations {
							load: LoadOp::Clear(Color::BLACK),
							store: StoreOp::Store,
						},
						resolve_target: None,
						view: &loaded_level.interact_view,
					}),
				],
				depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
					depth_ops: Some(Operations {
						load: LoadOp::Clear(1.0),
						store: StoreOp::Store,
					}),
					stencil_ops: None,
					view: &loaded_level.depth_view,
				}),
				timestamp_writes: None,
				occlusion_query_set: None,
			});
			
			rpass.set_bind_group(0, &loaded_level.bind_group, &[]);
			rpass.set_index_buffer(loaded_level.flipped_indices_buffer.slice(..), IndexFormat::Uint16);
			rpass.set_vertex_buffer(0, loaded_level.face_vertex_index_buffer.slice(..));
			
			rpass.set_vertex_buffer(1, loaded_level.face_instance_buffer.slice(..));
			rpass.set_pipeline(&self.textured_pipeline);
			for room in render_room_range.clone().map(|render_room_index| {
				&loaded_level.rooms[loaded_level.render_rooms[render_room_index].0[room_version]]
			}) {
				rpass.draw(0..NUM_QUAD_VERTICES, room.quads.original());
				rpass.draw(0..NUM_TRI_VERTICES, room.tris.original());
				rpass.draw_indexed(0..NUM_QUAD_VERTICES, 0, room.quads.flipped());
				rpass.draw_indexed(0..NUM_TRI_VERTICES, 0, room.tris.flipped());
				for mesh in &room.meshes {
					rpass.draw(0..NUM_QUAD_VERTICES, mesh.textured_quads.clone());
					rpass.draw(0..NUM_TRI_VERTICES, mesh.textured_tris.clone());
				}
			}
			
			rpass.set_pipeline(&self.solid_pipeline);
			for room in render_room_range.clone().map(|render_room_index| {
				&loaded_level.rooms[loaded_level.render_rooms[render_room_index].0[room_version]]
			}) {
				for mesh in &room.meshes {
					rpass.draw(0..NUM_QUAD_VERTICES, mesh.solid_quads.clone());
					rpass.draw(0..NUM_TRI_VERTICES, mesh.solid_tris.clone());
				}
			}
			
			rpass.set_vertex_buffer(1, loaded_level.sprite_instance_buffer.slice(..));
			rpass.set_pipeline(&self.sprite_pipeline);
			for room in render_room_range.clone().map(|render_room_index| {
				&loaded_level.rooms[loaded_level.render_rooms[render_room_index].0[room_version]]
			}) {
				rpass.draw(0..NUM_QUAD_VERTICES, room.sprites.clone());
			}
		}
		if self.print {
			println!("render time: {}us", last_render_time.as_micros());
		}
	}
	
	fn gui(&mut self, window: &Window, device: &Device, queue: &Queue, ctx: &egui::Context) {
		self.file_dialog.update(ctx);
		if let Some(path) = self.file_dialog.take_selected() {
			match load_level(device, queue, &self.bind_group_layout, self.window_size, &path) {
				Ok(loaded_level) => {
					self.loaded_level = Some(loaded_level);
					if let Some(file_name) = path.file_name().map(|f| f.to_string_lossy()) {
						window.set_title(&format!("{} - {}", WINDOW_TITLE, file_name));
					}
				},
				Err(e) => self.error = Some(e.to_string()),
			}
		}
		match &mut self.loaded_level {
			None => {
				egui::panel::CentralPanel::default().show(ctx, |ui| {
					ui.centered_and_justified(|ui| {
						if ui.label("Ctrl+O or click to open file").clicked() {
							self.file_dialog.select_file();
						}
					});
				});
			},
			Some(loaded_level) => draw_window(ctx, "Render Options", |ui| render_options(loaded_level, ui)),
		}
		if let Some(error) = &self.error {
			if draw_window(ctx, "Error", |ui| {
				ui.label(error);
				ui.button("OK").clicked()
			}) {
				self.error = None;
			}
		}
		self.print = false;
	}
}

fn make_pipeline(
	device: &Device, bind_group_layout: &BindGroupLayout, module: &ShaderModule, vs_entry: &str,
	fs_entry: &str, instance: VertexFormat,
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
						(VertexStepMode::Vertex, &[VertexFormat::Uint32]),
						(VertexStepMode::Instance, &[instance]),
					],
				),
			},
			primitive: PrimitiveState {
				topology: PrimitiveTopology::TriangleStrip,
				cull_mode: Some(wgpu::Face::Back),
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
					Some(ColorTargetState {
						format: TextureFormat::R32Uint,
						blend: None,
						write_mask: ColorWrites::ALL,
					}),
				],
			}),
			multiview: None,
		},
	)
}

fn make_gui(device: &Device, window_size: UVec2) -> TrTool {
	let bind_group_layout = make::bind_group_layout(
		device,
		&[
			(make::storage_layout_entry(GEOM_BUFFER_SIZE), ShaderStages::VERTEX),//data
			(make::uniform_layout_entry(size_of::<Mat4>()), ShaderStages::VERTEX),//camera_transform
			(make::uniform_layout_entry(size_of::<Mat4>()), ShaderStages::VERTEX),//perspective_transform
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
	let shader = make::shader(device, include_str!("shader/mesh.wgsl"));
	let [textured_pipeline, solid_pipeline, sprite_pipeline] = [
		("textured_vs_main", "textured_fs_main", VertexFormat::Uint32x2),
		("solid_vs_main", "solid_fs_main", VertexFormat::Uint32x2),
		("sprite_vs_main", "sprite_fs_main", VertexFormat::Sint32x4),
	].map(|(vs_entry, fs_entry, instance)| {
		make_pipeline(device, &bind_group_layout, &shader, vs_entry, fs_entry, instance)
	});
	
	TrTool {
		bind_group_layout,
		textured_pipeline,
		solid_pipeline,
		sprite_pipeline,
		window_size,
		modifiers: ModifiersState::empty(),
		// file_dialog: FileDialog::new().initial_directory(r"C:\Program Files (x86)\Steam\steamapps\common\Tomb Raider (I)\extracted\DATA".into()),
		// file_dialog: FileDialog::new().initial_directory(r"C:\Users\zane\Downloads\silver\trles\problem\SabatusTombRaider1_Revisited\DATA".into()),
		file_dialog: FileDialog::new().initial_directory(r"C:\Program Files (x86)\Steam\steamapps\common\TombRaider (III)\data".into()),
		error: None,
		print: false,
		loaded_level: None,
	}
}

fn main() {
	// fn read_boxed<R: Read, T: tr_model::Readable>(reader: &mut R) -> Result<Box<T>> {
	// 	let mut obj = Box::new(MaybeUninit::uninit());
	// 	unsafe {
	// 		T::read(reader, obj.as_mut_ptr())?;
	// 		Ok(obj.assume_init())
	// 	}
	// }
	// for path in std::fs::read_dir(r"C:\Program Files (x86)\Steam\steamapps\common\TombRaider (III)\data")
	// 	.unwrap().map(|e| e.unwrap().path())
	// 	.filter(|p| p.extension().map(|e| e.eq_ignore_ascii_case("tr2")).unwrap_or(false)) {
	// 	let level = read_boxed::<_, tr3::Level>(&mut File::open(&path).unwrap()).unwrap();
	// 	let level_name = path.file_name().unwrap().to_str().unwrap();
		
	// 	image::save_buffer(format!("{}_pal.png", level_name), &level.palette_24bit.as_bytes().iter().map(|c| c << 2).collect::<Vec<_>>(), 256, 1, image::ColorType::Rgb8).unwrap();
		
	// 	// let image_data = level.atlases.atlases_palette.iter().flatten().map(|&i| level.palette_24bit[i as usize]).map(|c| [c.r, c.g, c.b].map(|c| c << 2)).flatten().collect::<Vec<_>>();
	// 	// image::save_buffer(format!("{}_palette.png", level_name), &image_data, 256, 256 * level.atlases.atlases_palette.len() as u32, image::ColorType::Rgb8).unwrap();
		
	// 	// let image_data = level.atlases.atlases_16bit.iter().flatten().map(|c| ([c.r(), c.g(), c.b()].map(|c| c << 3), c.a() as u8 * 255)).map(|([r, g, b], a)| [r, g, b, a]).flatten().collect::<Vec<_>>();
	// 	// image::save_buffer(format!("{}_16bit.png", level_name), &image_data, 256, 256 * level.atlases.atlases_16bit.len() as u32, image::ColorType::Rgba8).unwrap();
		
	// 	println!("written");
	// }
	// return;
	
	let window_icon_bytes = include_bytes!("res/icon16.data");
	let taskbar_icon_bytes = include_bytes!("res/icon24.data");
	let window_icon = Icon::from_rgba(window_icon_bytes.to_vec(), 16, 16).expect("window icon");
	let taskbar_icon = Icon::from_rgba(taskbar_icon_bytes.to_vec(), 24, 24).expect("taskbar icon");
	gui::run(WINDOW_TITLE, window_icon, taskbar_icon, make_gui);
}
