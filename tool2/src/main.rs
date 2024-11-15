mod as_bytes;
mod gui;
mod make;
mod keys;
mod version_traits;
mod vec_tail;
mod data_writer;
mod face_writer;
mod multi_cursor;

use std::{
	collections::HashMap, f32::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU}, fs::File,
	io::{BufReader, Error, Read, Result, Seek}, mem::{size_of, take, MaybeUninit}, ops::Range,
	path::PathBuf, time::{Duration, Instant},
};
use data_writer::{FaceArrayRef, DataWriter, DATA_SIZE};
use face_writer::FaceWriter;
use image::ColorType;
use keys::{KeyGroup, KeyStates};
use egui_file_dialog::{DialogState, FileDialog};
use as_bytes::AsBytes;
use glam::{DVec2, EulerRot, Mat4, UVec2, Vec3, Vec3Swizzles};
use gui::Gui;
use version_traits::{Entity, Frame, Level, Mesh, Room, RoomGeom, RoomStaticMesh, RoomVertex, SolidFace};
use shared::min_max::{MinMax, VecMinMaxFromIterator};
use tr_model::{tr1, tr2, tr3};
use wgpu::{
	util::{DeviceExt, TextureDataOrder}, BindGroup, BindGroupLayout, BindingResource, BindingType, Buffer,
	BufferDescriptor, BufferUsages, Color, ColorTargetState, ColorWrites, CommandEncoder,
	CommandEncoderDescriptor, CompareFunction, DepthBiasState, DepthStencilState, Device, Extent3d, Face,
	FragmentState, FrontFace, ImageCopyBuffer, ImageDataLayout, LoadOp, MaintainBase, MultisampleState,
	Operations, PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology, Queue,
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
const FACE_VERT_INDICES: [u32; 4] = [1, 2, 0, 3];
const NUM_QUAD_VERTS: u32 = 4;
const NUM_TRI_VERTS: u32 = 3;

const FORWARD: Vec3 = Vec3::NEG_Z;
const BACKWARD: Vec3 = Vec3::Z;
const LEFT: Vec3 = Vec3::X;
const RIGHT: Vec3 = Vec3::NEG_X;
const DOWN: Vec3 = Vec3::Y;
const UP: Vec3 = Vec3::NEG_Y;

struct FaceTypes<TQ, TT = TQ, SQ = TQ, ST = TQ> {
	textured_quads: TQ,
	textured_tris: TT,
	solid_quads: SQ,
	solid_tris: ST,
}

type FaceOffsets = FaceTypes<u32>;
type FaceRanges = FaceTypes<Range<u32>>;
type MeshFaceArrayRefs<SolidQuad, SolidTri> = FaceTypes<
	FaceArrayRef<tr1::MeshTexturedQuad>,
	FaceArrayRef<tr1::MeshTexturedTri>,
	FaceArrayRef<SolidQuad>,
	FaceArrayRef<SolidTri>,
>;

impl FaceOffsets {
	fn make_range(self, end: Self) -> FaceRanges {
		FaceTypes {
			textured_quads: self.textured_quads..end.textured_quads,
			textured_tris: self.textured_tris..end.textured_tris,
			solid_quads: self.solid_quads..end.solid_quads,
			solid_tris: self.solid_tris..end.solid_tris,
		}
	}
}

struct RoomData {
	face_ranges: FaceRanges,
	sprites_range: Range<u32>,
	center: Vec3,
	radius: f32,
}

//[original room index, alt room index]
struct RenderRoom([usize; 2]);

#[derive(Clone, Copy)]
enum RoomFaceType {
	Quad,
	Tri,
}

#[derive(Clone, Copy)]
enum MeshFaceType {
	TexturedQuad,
	TexturedTri,
	SolidQuad,
	SolidTri,
}

enum ObjectData {
	RoomFace {
		room_index: u16,
		face_type: RoomFaceType,
		face_index: u16,
	},
	StaticMeshFace {
		room_index: u16,
		room_static_mesh_index: u16,
		face_type: MeshFaceType,
		face_index: u16,
	},
	EntityMeshFace {
		id: u16,
	},
	Sprite {
		id: u16,
	},
}

struct ObjectDataWriter(Vec<ObjectData>);

impl ObjectDataWriter {
	fn new() -> Self {
		Self(vec![])
	}
	
	fn add_room_faces(&mut self, room_index: u16, face_type: RoomFaceType, num: usize) -> u32 {
		let id = self.0.len() as u32;
		for face_index in 0..num as u16 {
			self.0.push(ObjectData::RoomFace { room_index, face_type, face_index });
		}
		id
	}
	
	fn add_room_static_mesh_faces(
		&mut self, room_index: u16, room_static_mesh_index: u16, face_type: MeshFaceType, num: u32,
	) {
		for face_index in 0..num as u16 {
			self.0.push(
				ObjectData::StaticMeshFace { room_index, room_static_mesh_index, face_type, face_index },
			);
		}
	}
	
	fn add_room_static_mesh<SQ, ST>(
		&mut self, room_index: u16, room_static_mesh_index: u16, mesh: &MeshFaceArrayRefs<SQ, ST>,
	) -> u32 {
		let id = self.0.len() as u32;
		self.add_room_static_mesh_faces(room_index, room_static_mesh_index, MeshFaceType::TexturedQuad, mesh.textured_quads.len);
		self.add_room_static_mesh_faces(room_index, room_static_mesh_index, MeshFaceType::TexturedTri, mesh.textured_tris.len);
		self.add_room_static_mesh_faces(room_index, room_static_mesh_index, MeshFaceType::SolidQuad, mesh.solid_quads.len);
		self.add_room_static_mesh_faces(room_index, room_static_mesh_index, MeshFaceType::SolidTri, mesh.solid_tris.len);
		id
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
	//camera
	pos: Vec3,
	yaw: f32,
	pitch: f32,
	
	//render
	depth_view: TextureView,
	interact_texture: Texture,
	interact_view: TextureView,
	camera_transform_buffer: Buffer,
	perspective_transform_buffer: Buffer,
	face_vertex_index_buffer: Buffer,
	face_instance_buffer: Buffer,
	sprite_instance_buffer: Buffer,
	bind_group: BindGroup,
	
	//rooms
	rooms: Vec<RoomData>,
	render_rooms: Vec<RenderRoom>,
	
	//render options
	render_room_index: Option<usize>,//if None, render all
	render_alt_rooms: bool,
	
	//input state
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

trait LevelInfo {
	fn print_info(&self);
}

impl LevelInfo for tr1::Level {
	fn print_info(&self) {
		let top_bits = self.rooms.iter().map(|room| {
			let geom = room.get_geom();
			geom.quads.iter().map(|quad| quad.object_texture_index).chain(
				geom.tris.iter().map(|tri| tri.object_texture_index),
			)
		}).flatten().filter(|tex_index| tex_index >> 15 != 0).count();
		println!("obj tex top bits: {}", top_bits);
	}
}

impl LevelInfo for tr2::Level {
	fn print_info(&self) {
		let top_bits = self.rooms.iter().map(|room| {
			let geom = room.get_geom();
			geom.quads.iter().map(|quad| quad.object_texture_index).chain(
				geom.tris.iter().map(|tri| tri.object_texture_index),
			)
		}).flatten().filter(|tex_index| tex_index >> 15 != 0).count();
		println!("obj tex top bits: {}", top_bits);
	}
}

impl LevelInfo for tr3::Level {
	fn print_info(&self) {
		let top_bits = self.rooms.iter().map(|room| {
			let geom = room.get_geom();
			geom.quads.iter().map(|quad| quad.object_texture_index).chain(
				geom.tris.iter().map(|tri| tri.object_texture_index),
			)
		}).flatten().filter(|tex_index| tex_index >> 15 != 0).count();
		println!("obj tex top bits: {}", top_bits);
	}
}

fn make_interact_texture(device: &Device, window_size: UVec2) -> Texture {
	make::texture(
		device, window_size, TextureFormat::R32Uint,
		TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
	)
}

fn read_level<L: Level + LevelInfo, R: Read>(
	device: &Device, queue: &Queue, bind_group_layout: &BindGroupLayout, window_size: UVec2, reader: &mut R,
) -> Result<LoadedLevel> {
	//read file and parse level
	let level = unsafe {
		let mut level = Box::new(MaybeUninit::uninit());
		L::read(reader, level.as_mut_ptr())?;
		level.assume_init()
	};
	
	level.print_info();
	
	//map static mesh ids to static mesh refs
	let static_mesh_id_map = level
		.static_meshes()
		.iter()
		.map(|static_mesh| (static_mesh.id as u16, static_mesh))
		.collect::<HashMap<_, _>>();
	
	//map model and sprite sequence ids to model and sprite sequence refs
	let model_id_map = level
		.models()
		.iter()
		.map(|model| (model.id as u16, ModelRef::Model(model)))
		.chain(level.sprite_sequences().iter().map(|ss| (ss.id as u16, ModelRef::SpriteSequence(ss))))
		.collect::<HashMap<_, _>>();
	
	//group entities by room
	let mut room_entities = vec![vec![]; level.rooms().len()];
	for entity in level.entities() {
		room_entities[entity.room_index() as usize].push(entity);
	}
	
	//data
	let mut data = DataWriter::<L>::new();
	data.write_object_textures(&level.object_textures());
	data.write_sprite_textures(&level.sprite_textures());
	let mut faces = FaceWriter::<L>::new();
	let mut sprite_instances = vec![];
	let mut object_data = ObjectDataWriter::new();
	
	//write meshes to data, map tr mesh offets to meshes indices
	// let mut meshes = Vec::<MeshFaceArrayRefs<L::Mesh>>::new();
	let mut mesh_offset_map = HashMap::new();
	for &mesh_offset in level.mesh_offsets().iter() {
		mesh_offset_map.entry(mesh_offset).or_insert_with(|| {
			let mesh = level.get_mesh(mesh_offset);
			let vertex_array_offset = data.write_vertex_array(mesh.vertices());
			let index = meshes.len();
			meshes.push((
				MeshFaceArrayRefs {
					textured_quads: data.write_face_array(mesh.textured_quads(), vertex_array_offset),
					textured_tris: data.write_face_array(mesh.textured_tris(), vertex_array_offset),
					solid_quads: data.write_face_array(mesh.solid_quads(), vertex_array_offset),
					solid_tris: data.write_face_array(mesh.solid_tris(), vertex_array_offset),
				},
				mesh,
			));
			index
		});
	}
	
	let mut room_indices = (0..level.rooms().len()).collect::<Vec<_>>();//rooms with alt will be removed
	let mut render_rooms = Vec::with_capacity(level.rooms().len());//rooms.len is upper-bound
	let mut rooms = Vec::with_capacity(level.rooms().len());
	
	//rooms
	for (room_index, room) in level.rooms().iter().enumerate() {
		let room_index = room_index as u16;
		let face_offsets_start = faces.get_offsets();
		let sprites_offset_start = sprite_instances.len() as u32;
		
		let room_geom = room.get_geom();
		let room_pos = room.pos();
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
		
		let vertex_array_offset = data.write_vertex_array(room_geom.vertices());
		let quads_ref = data.write_face_array(room_geom.quads(), vertex_array_offset);
		let tris_ref = data.write_face_array(room_geom.tris(), vertex_array_offset);
		let transform = Mat4::from_translation(room_pos.as_vec3());
		let transform_index = data.write_transform(&transform);
		
		//object data
		let quads_id = object_data.add_room_faces(room_index, RoomFaceType::Quad, room_geom.quads().len());
		let tris_id = object_data.add_room_faces(room_index, RoomFaceType::Tri, room_geom.tris().len());
		
		faces.write_face_instance_array(quads_ref, transform_index, quads_id);
		faces.write_face_instance_array(tris_ref, transform_index, tris_id);
		for (room_static_mesh_index, room_static_mesh) in room.room_static_meshes().iter().enumerate() {
			let room_static_mesh_index = room_static_mesh_index as u16;
			let static_mesh = match static_mesh_id_map.get(&room_static_mesh.static_mesh_id()) {
				Some(static_mesh) => *static_mesh,
				None => {
					println!("static mesh with id {} missing", room_static_mesh.static_mesh_id());
					continue;
				},
			};
			let transform = Mat4::from_translation(room_static_mesh.pos().as_vec3())
				* Mat4::from_rotation_y(room_static_mesh.angle() as f32 / 65536.0 * TAU);
			let transform_index = data.write_transform(&transform);
			let (face_array_refs, mesh)
				= &meshes[mesh_offset_map[&level.mesh_offsets()[static_mesh.mesh_offset_index as usize]]];
			
			//object data
			let id = object_data.add_room_static_mesh_faces(
				room_index, room_static_mesh_index, MeshFaceType::TexturedQuad, mesh.textured_quads().len(),
			);
			
			// let id_start = face_data.len() as u32;
			// for &object_texture_index in textured_quad_data {
			// 	face_data.push(FaceData {
			// 		room_index: room_index as u16,
			// 		face_type: FaceType::Quad,
			// 		mesh_type: MeshType::Mesh {
			// 			texture: TextureType::Texture { object_texture_index },
			// 			mesh_mesh_type: MeshMeshType::StaticMesh { id: room_static_mesh.static_mesh_id() },
			// 		},
			// 	})
			// }
			// for &object_texture_index in textured_tri_data {
			// 	face_data.push(FaceData {
			// 		room_index: room_index as u16,
			// 		face_type: FaceType::Tri,
			// 		mesh_type: MeshType::Mesh {
			// 			texture: TextureType::Texture { object_texture_index },
			// 			mesh_mesh_type: MeshMeshType::StaticMesh { id: room_static_mesh.static_mesh_id() },
			// 		},
			// 	})
			// }
			// for &(color_index_24bit, color_index_32bit) in solid_quad_data {
			// 	face_data.push(FaceData {
			// 		room_index: room_index as u16,
			// 		face_type: FaceType::Quad,
			// 		mesh_type: MeshType::Mesh {
			// 			texture: TextureType::Solid { color_index_24bit, color_index_32bit },
			// 			mesh_mesh_type: MeshMeshType::StaticMesh { id: room_static_mesh.static_mesh_id() },
			// 		},
			// 	})
			// }
			// for &(color_index_24bit, color_index_32bit) in solid_tri_data {
			// 	face_data.push(FaceData {
			// 		room_index: room_index as u16,
			// 		face_type: FaceType::Tri,
			// 		mesh_type: MeshType::Mesh {
			// 			texture: TextureType::Solid { color_index_24bit, color_index_32bit },
			// 			mesh_mesh_type: MeshMeshType::StaticMesh { id: room_static_mesh.static_mesh_id() },
			// 		},
			// 	})
			// }
			
			faces.write_mesh(face_array_refs, transform_index, 0);
		}
		for sprite in room_geom.sprites() {
			let pos = room_pos + room_geom.vertices()[sprite.vertex_index as usize].pos().as_ivec3();
			sprite_instances.push(pos.extend(sprite.sprite_texture_index as i32));
		}
		for entity in &room_entities[room_index as usize] {
			match model_id_map[&entity.model_id()] {
				ModelRef::Model(model) => {
					let entity_transform = Mat4::from_translation(entity.pos().as_vec3())
						* Mat4::from_rotation_y(entity.angle() as f32 / 65536.0 * TAU);
					let frame = level.get_frame(model);
					let mut rotations = frame.iter_rotations();
					let mut last_transform = Mat4::from_translation(frame.offset().as_vec3())
						* rotations.next().unwrap();
					let transform_index = data.write_transform(&(entity_transform * last_transform));
					let (mesh, ..)
						= &meshes[mesh_offset_map[&level.mesh_offsets()[model.mesh_offset_index as usize]]];
					
					
					
					faces.write_mesh(mesh, transform_index, 0);
					let mut parent_stack = vec![];
					let mesh_nodes = level.get_mesh_nodes(model);
					for mesh_node_index in 0..mesh_nodes.len() {
						let mesh_node = &mesh_nodes[mesh_node_index];
						let parent = if mesh_node.flags.pop() {
							parent_stack.pop().expect("mesh stack empty")
						} else {
							last_transform
						};
						if mesh_node.flags.push() {
							parent_stack.push(parent);
						}
						let (mesh, ..)
							= &meshes[mesh_offset_map[&level.mesh_offsets()[
								model.mesh_offset_index as usize + mesh_node_index + 1
							]]];
						last_transform = parent
							* Mat4::from_translation(mesh_node.offset.as_vec3())
							* rotations.next().unwrap();
						let transform_index = data.write_transform(&(entity_transform * last_transform));
						faces.write_mesh(mesh, transform_index, 0);
					}
				},
				ModelRef::SpriteSequence(sprite_sequence) => {
					sprite_instances.push(entity.pos().extend(sprite_sequence.sprite_texture_index as i32));
				},
			}
		}
		
		let face_ranges = face_offsets_start.make_range(faces.get_offsets());
		let sprites_range = sprites_offset_start..sprite_instances.len() as u32;
		
		rooms.push(RoomData { face_ranges, sprites_range, center, radius });
		
		if room.alt_room_index() != u16::MAX {
			let alt_room_index = room.alt_room_index() as usize;
			room_indices.remove(room_indices.binary_search(&room_index).unwrap());
			room_indices.remove(room_indices.binary_search(&alt_room_index).expect("alt room index"));
			render_rooms.push(RenderRoom([room_index, alt_room_index]));
		}
	}
	
	for static_room_index in room_indices {
		render_rooms.push(RenderRoom([static_room_index; 2]));
	}
	render_rooms.sort_by_key(|rr| rr.0[0]);
	
	//level bind group
	let data_buffer = make::buffer(device, &data.into_buffer(), BufferUsages::STORAGE);
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
	let palette_buffer = make::buffer(device, level.palette().as_bytes(), BufferUsages::UNIFORM);
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
	
	let face_instances = faces.into_buffer();
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
	
	Ok(LoadedLevel {
		pos,
		yaw,
		pitch,
		
		depth_view: make::depth_view(device, window_size),
		interact_texture,
		interact_view,
		camera_transform_buffer,
		perspective_transform_buffer,
		face_vertex_index_buffer: make::buffer(device, FACE_VERT_INDICES.as_bytes(), BufferUsages::VERTEX),
		face_instance_buffer: make::buffer(device, &face_instances, BufferUsages::VERTEX),
		sprite_instance_buffer: make::buffer(device, sprite_instances.as_bytes(), BufferUsages::VERTEX),
		bind_group,
		
		rooms,
		render_rooms,
		render_room_index: None,
		render_alt_rooms: false,
		
		mouse_control: false,
		key_states: KeyStates::new(),
		action_map,
		
		frame_update_queue: vec![],
	})
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
	match (version, extension.as_ref().map(|e| e.as_ref())) {
		(32, _) => read_level::<tr1::Level, _>(device, queue, bind_group_layout, window_size, &mut reader),
		(45, _) => read_level::<tr2::Level, _>(device, queue, bind_group_layout, window_size, &mut reader),
		(0xFF180038, _) => read_level::<tr3::Level, _>(device, queue, bind_group_layout, window_size, &mut reader),
		_ => return Err(Error::other("unknown file type")),
	}
}

fn draw_window<R, F: FnOnce(&mut egui::Ui) -> R>(ctx: &egui::Context, title: &str, contents: F) -> R {
	egui::Window::new(title).collapsible(false).resizable(false).show(ctx, contents).unwrap().inner.unwrap()
}

fn selected_room_text(render_room_index: Option<usize>) -> String {
	match render_room_index {
		Some(render_room_index) => format!("Room {}", render_room_index),
		None => "All".into(),
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
		if let Some(loaded_level) = &mut self.loaded_level {
			loaded_level.key_states.set(key_code, state.is_pressed());
		}
		match (self.modifiers, state, key_code, repeat, &mut self.loaded_level) {
			(_, ElementState::Pressed, KeyCode::Escape, false, _) => target.exit(),
			(_, ElementState::Pressed, KeyCode::KeyP, _, _) => self.print = true,
			(ModifiersState::CONTROL, ElementState::Pressed, KeyCode::KeyO, false, loaded_level) => {
				if let Some(loaded_level) = loaded_level {
					loaded_level.set_mouse_control(window, false);
				}
				self.file_dialog.select_file();
			},
			(_, ElementState::Pressed, KeyCode::KeyZ, false, Some(loaded_level)) => {
				let width = ((loaded_level.interact_texture.width() + 63) / 64) * 64;
				let height = loaded_level.interact_texture.height();
				let buf = device.create_buffer(&BufferDescriptor {
					label: None,
					size: (width * height * 4) as u64,
					usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
					mapped_at_creation: false,
				});
				let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());
				encoder.copy_texture_to_buffer(
					loaded_level.interact_texture.as_image_copy(),
					ImageCopyBuffer {
						buffer: &buf,
						layout: ImageDataLayout {
							offset: 0,
							bytes_per_row: Some(width * 4),
							rows_per_image: None,
						},
					},
					loaded_level.interact_texture.size(),
				);
				queue.submit([encoder.finish()]);
				let slice = buf.slice(..);
				slice.map_async(wgpu::MapMode::Read, |_| {});
				device.poll(MaintainBase::Wait);
				let bytes = &*slice.get_mapped_range();
			},
			_ => {},
		}
	}
	
	fn mouse_button(&mut self, window: &Window, button: MouseButton, state: ElementState) {
		if let Some(loaded_level) = &mut self.loaded_level {
			match (state, button) {
				(ElementState::Pressed, MouseButton::Right) => {
					if !matches!(self.file_dialog.state(), DialogState::Open) {
						loaded_level.set_mouse_control(window, !loaded_level.mouse_control);
					}
				},
				_ => {},
			}
		}
	}
	
	fn mouse_moved(&mut self, delta: DVec2) {
		if let Some(loaded_level) = &mut self.loaded_level {
			if loaded_level.mouse_control {
				loaded_level.yaw += delta.x as f32 / 150.0;
				loaded_level.pitch = (loaded_level.pitch + delta.y as f32 / 150.0)
					.clamp(-FRAC_PI_2, FRAC_PI_2);
			}
		}
	}
	
	fn mouse_wheel(&mut self, _: MouseScrollDelta) {}
	
	fn render(
		&mut self, queue: &Queue, encoder: &mut CommandEncoder, color_view: &TextureView,
		delta_time: Duration, last_render_time: Duration,
	) {
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
			rpass.set_vertex_buffer(0, loaded_level.face_vertex_index_buffer.slice(..));
			rpass.set_vertex_buffer(1, loaded_level.face_instance_buffer.slice(..));
			rpass.set_pipeline(&self.textured_pipeline);
			for render_room_index in render_room_range.clone() {
				let room = &loaded_level.rooms[loaded_level.render_rooms[render_room_index].0[room_version]];
				rpass.draw(0..NUM_QUAD_VERTS, room.face_ranges.textured_quads.clone());
				rpass.draw(0..NUM_TRI_VERTS, room.face_ranges.textured_tris.clone());
			}
			rpass.set_pipeline(&self.solid_pipeline);
			for render_room_index in render_room_range.clone() {
				let room = &loaded_level.rooms[loaded_level.render_rooms[render_room_index].0[room_version]];
				rpass.draw(0..NUM_QUAD_VERTS, room.face_ranges.solid_quads.clone());
				rpass.draw(0..NUM_TRI_VERTS, room.face_ranges.solid_tris.clone());
			}
			rpass.set_pipeline(&self.sprite_pipeline);
			rpass.set_vertex_buffer(1, loaded_level.sprite_instance_buffer.slice(..));
			for render_room_index in render_room_range {
				let room = &loaded_level.rooms[loaded_level.render_rooms[render_room_index].0[room_version]];
				rpass.draw(0..NUM_QUAD_VERTS, room.sprites_range.clone());
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
			(make::storage_layout_entry(DATA_SIZE), ShaderStages::VERTEX),//data
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
		file_dialog: FileDialog::new().initial_directory(r"C:\Program Files (x86)\Steam\steamapps\common\TombRaider (III)\data".into()),
		// file_dialog: FileDialog::new().initial_directory(r"C:\Program Files (x86)\Steam\steamapps\common\Tomb Raider (I)\extracted\DATA".into()),
		// file_dialog: FileDialog::new().initial_directory(r"C:\Users\zane\Downloads\silver\trles\problem\SabatusTombRaider1_Revisited\DATA".into()),
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
