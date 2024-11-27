mod as_bytes;
mod gui;
mod make;
mod keys;
mod tr_traits;
mod vec_tail;
mod fixed_vec;
mod geom_buffer;
mod multi_cursor;
mod data_writer;

use std::{
	collections::HashMap, f32::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU}, fs::File,
	io::{BufReader, Error, Read, Result, Seek}, mem::{size_of, take, MaybeUninit}, ops::Range,
	path::PathBuf, sync::Arc, thread::{spawn, JoinHandle}, time::Duration,
};
use data_writer::{DataWriter, MeshFaceOffsets, Results, RoomFaceOffsets};
use geom_buffer::{GeomBuffer, GEOM_BUFFER_SIZE};
use keys::{KeyGroup, KeyStates};
use egui_file_dialog::{DialogState, FileDialog};
use as_bytes::{AsBytes, ReinterpretAsBytes};
use glam::{DVec2, EulerRot, Mat4, Vec3, Vec3Swizzles};
use gui::Gui;
// use versions::{converge, Level, Slice, TrVersion};
use shared::min_max::{MinMax, VecMinMaxFromIterator};
use tr_model::{tr1, tr2, tr3};
use tr_traits::{
	Entity, Face, Frame, Level, LevelStore, Mesh, MeshFaceType, Room, RoomFace, RoomFaceType, RoomGeom,
	RoomStaticMesh, RoomVertex, SolidFace, TexturedFace,
};
use wgpu::{
	BindGroup, BindGroupLayout, BindingResource, BlendComponent, BlendFactor, BlendOperation, BlendState,
	Buffer, BufferDescriptor, BufferUsages, Color, ColorTargetState, ColorWrites, CommandEncoder,
	CommandEncoderDescriptor, CompareFunction, DepthBiasState, DepthStencilState, Device, Extent3d,
	FragmentState, FrontFace, ImageCopyBuffer, ImageDataLayout, IndexFormat, LoadOp, Maintain, MapMode,
	MultisampleState, Operations, PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology, Queue,
	RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor, RenderPipeline,
	RenderPipelineDescriptor, ShaderModule, ShaderStages, StencilState, StoreOp, Texture,
	TextureDimension, TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
	TextureViewDimension, VertexFormat, VertexState, VertexStepMode,
};
use winit::{
	dpi::{PhysicalPosition, PhysicalSize}, event::{ElementState, MouseButton, MouseScrollDelta},
	event_loop::EventLoopWindowTarget, keyboard::{KeyCode, ModifiersState},
	window::{CursorGrabMode, Icon, Window},
};

const WINDOW_TITLE: &str = "TR Tool";

/*
This ordering creates a "Z" so triangle strip mode may be used for quads, and the first three indices used
for tris.
*/
const FACE_VERTEX_INDICES: [u32; 4] = [1, 2, 0, 3];
const REVERSE_INDICES: [u16; 4] = [0, 2, 1, 3];//yields face vertex indices [1, 0, 2, 3]
const NUM_QUAD_VERTICES: u32 = 4;
const NUM_TRI_VERTICES: u32 = 3;

const FORWARD: Vec3 = Vec3::NEG_Z;
const BACKWARD: Vec3 = Vec3::Z;
const LEFT: Vec3 = Vec3::X;
const RIGHT: Vec3 = Vec3::NEG_X;
const DOWN: Vec3 = Vec3::Y;
const UP: Vec3 = Vec3::NEG_Y;

const INTERACT_TEXTURE_FORMAT: TextureFormat = TextureFormat::R16Uint;
const INTERACT_PIXEL_SIZE: u32 = 2;
type InteractPixel = u16;

struct WrittenFaceArray<'a, F> {
	index: usize,
	faces: &'a [F],
}

struct WrittenMesh<'a, L: Level + 'a> {
	textured_quads: WrittenFaceArray<'a, <L::Mesh<'a> as Mesh<'a>>::TexturedQuad>,
	textured_tris: WrittenFaceArray<'a, <L::Mesh<'a> as Mesh<'a>>::TexturedTri>,
	solid_quads: WrittenFaceArray<'a, <L::Mesh<'a> as Mesh<'a>>::SolidQuad>,
	solid_tris: WrittenFaceArray<'a, <L::Mesh<'a> as Mesh<'a>>::SolidTri>,
}

struct RoomData {
	quads: RoomFaceOffsets,
	tris: RoomFaceOffsets,
	meshes: Vec<MeshFaceOffsets>,
	sprites: Range<u32>,
	center: Vec3,
	radius: f32,
}

/// `[original_room_index, alt_room_index]`
struct RenderRoom([usize; 2]);

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
		face_type: MeshFaceType,
		face_index: u16,
	},
	RoomSprite {
		room_index: u16,
		sprite_index: u16,
	},
	EntityMeshFace {
		entity_index: u16,
		mesh_index: u16,
		face_type: MeshFaceType,
		face_index: u16,
	},
	EntitySprite {
		entity_index: u16,
	},
	Reverse {
		object_data_index: u16,
	},
}

impl ObjectData {
	fn room_face(room_index: usize, face_type: RoomFaceType, face_index: usize) -> Self {
		Self::RoomFace { room_index: room_index as u16, face_type, face_index: face_index as u16 }
	}
	
	fn room_static_mesh_face(
		room_index: usize, room_static_mesh_index: usize, face_type: MeshFaceType, face_index: usize,
	) -> Self {
		Self::RoomStaticMeshFace {
			room_index: room_index as u16,
			room_static_mesh_index: room_static_mesh_index as u16,
			face_type,
			face_index: face_index as u16,
		}
	}
	
	fn room_sprite(room_index: usize, sprite_index: u16) -> Self {
		Self::RoomSprite { room_index: room_index as u16, sprite_index }
	}
	
	fn entity_mesh_face(
		entity_index: usize, mesh_index: usize, face_type: MeshFaceType, face_index: usize,
	) -> Self {
		Self::EntityMeshFace {
			entity_index: entity_index as u16,
			mesh_index: mesh_index as u16,
			face_type,
			face_index: face_index as u16,
		}
	}
	
	fn entity_sprite(entity_index: usize) -> Self {
		Self::EntitySprite { entity_index: entity_index as u16 }
	}
}

fn print_object_data<L: Level>(level: &L, object_data: &[ObjectData], index: u16) {
	println!("object data index: {}", index);
	let data = match object_data.get(index as usize) {
		Some(&data) => data,
		None => {
			println!("out of bounds");
			return;
		},
	};
	println!("{:?}", data);
	let data = match data {
		ObjectData::Reverse { object_data_index } => {
			let data = object_data[object_data_index as usize];
			println!("{:?}", data);
			data
		},
		data => data,
	};
	let mesh_face = match data {
		ObjectData::RoomFace { room_index, face_type, face_index } => {
			let room_geom = level.rooms()[room_index as usize].get_geom();
			let (double_sided, object_texture_index) = match face_type {
				RoomFaceType::Quad => {
					let quad = &room_geom.quads()[face_index as usize];
					(quad.double_sided(), quad.object_texture_index())
				},
				RoomFaceType::Tri => {
					let tri = &room_geom.tris()[face_index as usize];
					(tri.double_sided(), tri.object_texture_index())
				},
			};
			println!("double sided: {}", double_sided);
			let object_texture = &level.object_textures()[object_texture_index as usize];
			println!("blend mode: {}", object_texture.blend_mode);
			None
		},
		ObjectData::RoomStaticMeshFace { room_index, room_static_mesh_index, face_type, face_index } => {
			let room = &level.rooms()[room_index as usize];
			let room_static_mesh = &room.room_static_meshes()[room_static_mesh_index as usize];
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
			let model_id = level.entities()[entity_index as usize].model_id();
			let model = level.models().iter().find(|model| model.id as u16 == model_id).unwrap();
			let mesh_offset = level.mesh_offsets()[(model.mesh_offset_index + mesh_index) as usize];
			Some((mesh_offset, face_type, face_index))
		},
		ObjectData::EntitySprite { .. } => None,
		ObjectData::Reverse { .. } => unreachable!("reverse points to reverse"),
	};
	if let Some((mesh_offset, face_type, face_index)) = mesh_face {
		println!("mesh offset: {}", mesh_offset);
		let mesh = level.get_mesh(mesh_offset);
		let (object_texture_index, color_index_24bit, color_index_32bit) = match face_type {
			MeshFaceType::TexturedQuad => {
				(Some(mesh.textured_quads()[face_index as usize].object_texture_index()), None, None)
			},
			MeshFaceType::TexturedTri => {
				(Some(mesh.textured_tris()[face_index as usize].object_texture_index()), None, None)
			},
			MeshFaceType::SolidQuad => {
				let quad = &mesh.solid_quads()[face_index as usize];
				(None, Some(quad.color_index_24bit()), quad.color_index_32bit())
			},
			MeshFaceType::SolidTri =>  {
				let tri = &mesh.solid_tris()[face_index as usize];
				(None, Some(tri.color_index_24bit()), tri.color_index_32bit())
			},
		};
		if let Some(object_texture_index) = object_texture_index {
			let object_texture = &level.object_textures()[object_texture_index as usize];
			println!("blend mode: {}", object_texture.blend_mode);
		}
		if let (Some(color_index), Some(palette)) = (color_index_24bit, level.palette_24bit()) {
			let tr1::Color24Bit { r, g, b } = palette[color_index as usize];
			let [r, g, b] = [r, g, b].map(|c| (c << 2) as u32);
			let color = (r << 16) | (g << 8) | b;
			println!("color 24 bit: #{:06X}", color);
		}
		if let (Some(color_index), Some(palette)) = (color_index_32bit, level.palette_32bit()) {
			let color = &palette[color_index as usize];
			let [r, g, b] = [color.r(), color.g(), color.b()].map(|c| c as u32);
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

enum TextureType {
	Palette,
	Direct16Bit,
}

impl TextureType {
	fn label(&self) -> &'static str {
		match self {
			TextureType::Palette => "Palette",
			TextureType::Direct16Bit => "16 Bit",
		}
	}
}

enum SolidType {
	Color24Bit,
	Color32Bit,
}

impl SolidType {
	fn label(&self) -> &'static str {
		match self {
			SolidType::Color24Bit => "24 Bit",
			SolidType::Color32Bit => "32 Bit",
		}
	}
}

struct TextureBindGroup {
	bind_group: BindGroup,
	texture_type: TextureType,
}

struct SolidBindGroup {
	bind_group: BindGroup,
	solid_type: SolidType,
}

struct LoadedLevel {
	//constant
	face_vertex_index_buffer: Buffer,
	reverse_indices_buffer: Buffer,
	
	//render
	depth_view: TextureView,
	interact_texture: Texture,
	interact_view: TextureView,
	camera_transform_buffer: Buffer,
	perspective_transform_buffer: Buffer,
	face_instance_buffer: Buffer,
	sprite_instance_buffer: Buffer,
	texture_bind_groups: Vec<TextureBindGroup>,
	solid_bind_groups: Vec<SolidBindGroup>,
	texture_bind_group_index: usize,
	solid_bind_group_index: Option<usize>,
	
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
	level: LevelStore,
	object_data: Vec<ObjectData>,
	click_handle: Option<JoinHandle<InteractPixel>>,
	
	//input state
	mouse_pos: PhysicalPosition<f64>,
	locked_mouse_pos: PhysicalPosition<f64>,
	mouse_control: bool,
	key_states: KeyStates,
	action_map: ActionMap,
	
	frame_update_queue: Vec<Box<dyn FnOnce(&mut Self)>>,
}

struct BindGroupLayouts {
	solid: BindGroupLayout,
	texture_palette: BindGroupLayout,
	texture_direct: BindGroupLayout,
}

struct TexturePipelines {
	opaque: RenderPipeline,
	additive: RenderPipeline,
	sprite: RenderPipeline,
}

struct TrTool {
	//static
	bind_group_layouts: BindGroupLayouts,
	solid_24bit_pl: RenderPipeline,
	solid_32bit_pl: RenderPipeline,
	texture_palette_pls: TexturePipelines,
	texture_16bit_pls: TexturePipelines,
	
	//state
	window_size: PhysicalSize<u32>,
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

fn make_perspective_transform(window_size: PhysicalSize<u32>) -> Mat4 {
	Mat4::perspective_rh(FRAC_PI_4, window_size.width as f32 / window_size.height as f32, 100.0, 100000.0)
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
	
	fn update_perspective_transform(&self, queue: &Queue, window_size: PhysicalSize<u32>) {
		let perspective_transform = make_perspective_transform(window_size);
		queue.write_buffer(&self.perspective_transform_buffer, 0, perspective_transform.as_bytes());
	}
	
	fn frame_update(&mut self, queue: &Queue, delta_time: Duration) {
		if let Some(click_handle) = self.click_handle.take() {
			if click_handle.is_finished() {
				let o_idx = click_handle.join().expect("join click handle");
				match &self.level {
					LevelStore::Tr1(level) => print_object_data(level.as_ref(), &self.object_data, o_idx),
					LevelStore::Tr2(level) => print_object_data(level.as_ref(), &self.object_data, o_idx),
					LevelStore::Tr3(level) => print_object_data(level.as_ref(), &self.object_data, o_idx),
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

fn make_interact_texture(device: &Device, PhysicalSize { width, height }: PhysicalSize<u32>) -> Texture {
	make::texture(
		device,
		Extent3d { width, height, depth_or_array_layers: 1 },
		TextureDimension::D2, INTERACT_TEXTURE_FORMAT,
		TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
	)
}

fn write_face_array<'a, F: Face>(
	geom_buffer: &mut GeomBuffer, vertex_array_offset: usize, faces: &'a [F],
) -> WrittenFaceArray<'a, F> {
	WrittenFaceArray { index: geom_buffer.write_face_array(faces, vertex_array_offset), faces }
}

fn make_atlases_view<T: ReinterpretAsBytes>(
	device: &Device, queue: &Queue, atlases: &[T], format: TextureFormat,
) -> TextureView {
	make::texture_view_with_data(
		device, queue,
		Extent3d {
			width: tr1::ATLAS_SIDE_LEN as u32,
			height: tr1::ATLAS_SIDE_LEN as u32,
			depth_or_array_layers: atlases.len() as u32,
		},
		TextureDimension::D2, format, TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
		atlases.as_bytes(),
	)
}

fn make_palette_view<T: ReinterpretAsBytes>(
	device: &Device, queue: &Queue, palette: &[T; tr1::PALETTE_LEN],
) -> TextureView {
	make::texture_view_with_data(
		device, queue,
		Extent3d {
			width: size_of::<[T; tr1::PALETTE_LEN]>() as u32,
			height: 1,
			depth_or_array_layers: 1,
		},
		TextureDimension::D1, TextureFormat::R8Uint,
		TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING, palette.as_bytes(),
	)
}

fn parse_level<R: Read, L: Level>(
	device: &Device, queue: &Queue, bind_group_layouts: &BindGroupLayouts, window_size: PhysicalSize<u32>,
	reader: &mut R,
) -> Result<LoadedLevel> {
	let level = unsafe {
		let mut level = Box::new(MaybeUninit::uninit());
		L::read(reader, level.as_mut_ptr())?;
		level.assume_init()
	};
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
			let written_mesh = WrittenMesh {
				textured_quads: write_face_array(&mut geom_buffer, vao, mesh.textured_quads()),
				textured_tris: write_face_array(&mut geom_buffer, vao, mesh.textured_tris()),
				solid_quads: write_face_array(&mut geom_buffer, vao, mesh.solid_quads()),
				solid_tris: write_face_array(&mut geom_buffer, vao, mesh.solid_tris()),
			};
			written_meshes.push(written_mesh);
			index
		});
	}
	
	//rooms
	let mut data_writer = DataWriter::new(geom_buffer);
	let mut room_indices = (0..level.rooms().len()).collect::<Vec<_>>();//rooms with alt will be removed
	let mut render_rooms = Vec::with_capacity(level.rooms().len());//rooms.len is upper-bound
	
	let rooms = level.rooms().iter().enumerate().map(|(room_index, room)| {
		let room_pos = room.pos();
		let room_geom = room.get_geom();
		let room_vertices = room_geom.vertices();
		let sprites_start = data_writer.sprite_offset();
		let mut meshes = vec![];
		
		//room geom
		let (quads, tris) = {
			let vertex_array_offset = data_writer.geom_buffer.write_vertex_array(room_vertices);
			let transform = Mat4::from_translation(room_pos.as_vec3());
			let transform_index = data_writer.geom_buffer.write_transform(&transform);
			let obj_fn = |face_type, face_index| ObjectData::room_face(room_index, face_type, face_index);
			let quads = data_writer.write_room_face_array(
				level.as_ref(), vertex_array_offset, room_geom.quads(), transform_index, obj_fn,
			);
			let tris = data_writer.write_room_face_array(
				level.as_ref(), vertex_array_offset, room_geom.tris(), transform_index, obj_fn,
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
			let obj_fn = |face_type, f_index| {
				ObjectData::room_static_mesh_face(room_index, room_static_mesh_index, face_type, f_index)
			};
			
			meshes.push(data_writer.place_mesh(level.as_ref(), written_mesh, transform_index, obj_fn));
		}
		
		//room sprites
		data_writer.write_room_sprites(room_pos, room_vertices, room_geom.sprites(), |sprite_index| {
			ObjectData::room_sprite(room_index, sprite_index)
		});
		
		//entities
		for &entity_index in &room_entity_indices[room_index] {
			let entity = &level.entities()[entity_index];
			match model_id_map[&entity.model_id()] {
				ModelRef::Model(model) => {
					let entity_translation = Mat4::from_translation(entity.pos().as_vec3());
					let entity_rotation = Mat4::from_rotation_y(entity.angle() as f32 / 65536.0 * TAU);
					let entity_transform = entity_translation * entity_rotation;
					let frame = level.get_frame(model);
					let mut rotations = frame.iter_rotations();
					let first_translation = Mat4::from_translation(frame.offset().as_vec3());
					let first_rotation = rotations.next().unwrap();
					let mut last_transform = first_translation * first_rotation;
					let transform = entity_transform * last_transform;
					let transform_index = data_writer.geom_buffer.write_transform(&transform);
					let mesh_offset = level.mesh_offsets()[model.mesh_offset_index as usize];
					let mesh = &written_meshes[mesh_offset_map[&mesh_offset]];
					let obj_fn = |face_type, face_index| {
						ObjectData::entity_mesh_face(entity_index, 0, face_type, face_index)
					};
					meshes.push(data_writer.place_mesh(level.as_ref(), mesh, transform_index, obj_fn));
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
						let translation = Mat4::from_translation(mesh_node.offset.as_vec3());
						let rotation = rotations.next().unwrap();
						last_transform = parent * translation * rotation;
						let transform = entity_transform * last_transform;
						let transform_index = data_writer.geom_buffer.write_transform(&transform);
						let obj_fn = |face_type, face_index| ObjectData::entity_mesh_face(
							entity_index, mesh_node_index + 1, face_type, face_index,
						);
						meshes.push(
							data_writer.place_mesh(level.as_ref(), mesh, transform_index, obj_fn),
						);
					}
				},
				ModelRef::SpriteSequence(ss) => {
					data_writer.write_entity_sprite(entity_index, entity.pos(), ss.sprite_texture_index);
				},
			}
		}
		
		if room.alt_room_index() != u16::MAX {
			let alt_room_index = room.alt_room_index() as usize;
			room_indices.remove(room_indices.binary_search(&room_index).unwrap());
			room_indices.remove(room_indices.binary_search(&alt_room_index).expect("alt room index"));
			render_rooms.push(RenderRoom([room_index, alt_room_index]));
		}
		
		let sprites_end = data_writer.sprite_offset();
		let (center, radius) = room_geom
			.vertices()
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
		RoomData { quads, tris, meshes, sprites: sprites_start..sprites_end, center, radius, }
	}).collect::<Vec<_>>();
	
	//remaining room indices have no alt
	for room_index in room_indices {
		render_rooms.push(RenderRoom([room_index; 2]));
	}
	render_rooms.sort_by_key(|rr| rr.0[0]);
	
	let Results { geom_buffer, face_buffer, sprite_buffer, object_data } = data_writer.done();
	
	//bind groups
	let geom_buffer = make::buffer(device, &geom_buffer, BufferUsages::STORAGE);
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
	let geom_entry = (0, geom_buffer.as_entire_binding());
	let camera_entry = (1, camera_transform_buffer.as_entire_binding());
	let perspective_entry = (2, perspective_transform_buffer.as_entire_binding());
	let mut texture_bind_groups = vec![];
	let mut solid_bind_groups = vec![];
	if let (Some(atlases), Some(palette)) = (level.atlases_palette(), level.palette_24bit()) {
		let atlases_view = make_atlases_view(device, queue, atlases, TextureFormat::R8Uint);
		let palette_view = make_palette_view(device, queue, palette);
		let atlases_entry = (3, BindingResource::TextureView(&atlases_view));
		let palette_entry = (4, BindingResource::TextureView(&palette_view));
		let bind_group = make::bind_group(device, &bind_group_layouts.texture_palette, &[
			geom_entry.clone(),
			camera_entry.clone(),
			perspective_entry.clone(),
			atlases_entry.clone(),
			palette_entry.clone(),
		]);
		texture_bind_groups.push(TextureBindGroup { bind_group, texture_type: TextureType::Palette });
		let bind_group = make::bind_group(device, &bind_group_layouts.solid, &[
			geom_entry.clone(),
			camera_entry.clone(),
			perspective_entry.clone(),
			palette_entry.clone(),
		]);
		solid_bind_groups.push(SolidBindGroup { bind_group, solid_type: SolidType::Color24Bit });
	}
	if let Some(palette) = level.palette_32bit() {
		let palette_view = make_palette_view(device, queue, palette);
		let palette_entry = (4, BindingResource::TextureView(&palette_view));
		let bind_group = make::bind_group(device, &bind_group_layouts.solid, &[
			geom_entry.clone(),
			camera_entry.clone(),
			perspective_entry.clone(),
			palette_entry.clone(),
		]);
		solid_bind_groups.push(SolidBindGroup { bind_group, solid_type: SolidType::Color32Bit });
	}
	if let Some(atlases) = level.atlases_16bit() {
		let atlases_view = make_atlases_view(device, queue, atlases, TextureFormat::R16Uint);
		let atlases_entry = (3, BindingResource::TextureView(&atlases_view));
		let bind_group = make::bind_group(device, &bind_group_layouts.texture_direct, &[
			geom_entry.clone(),
			camera_entry.clone(),
			perspective_entry.clone(),
			atlases_entry.clone(),
		]);
		texture_bind_groups.push(TextureBindGroup { bind_group, texture_type: TextureType::Direct16Bit });
	}
	
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
	
	let face_vertex_index_buffer = make::buffer(
		device, FACE_VERTEX_INDICES.as_bytes(), BufferUsages::VERTEX,
	);
	let reverse_indices_buffer = make::buffer(device, REVERSE_INDICES.as_bytes(), BufferUsages::INDEX);
	Ok(LoadedLevel {
		face_vertex_index_buffer,
		reverse_indices_buffer,
		depth_view: make::depth_view(device, window_size),
		interact_texture,
		interact_view,
		camera_transform_buffer,
		perspective_transform_buffer,
		face_instance_buffer: make::buffer(device, &face_buffer, BufferUsages::VERTEX),
		sprite_instance_buffer: make::buffer(device, &sprite_buffer, BufferUsages::VERTEX),
		solid_bind_group_index: (!solid_bind_groups.is_empty()).then_some(0),
		texture_bind_group_index: 0,
		solid_bind_groups,
		texture_bind_groups,
		pos,
		yaw,
		pitch,
		rooms,
		render_rooms,
		render_room_index: None,
		render_alt_rooms: false,
		object_data,
		level: level.store(),
		click_handle: None,
		mouse_pos: PhysicalPosition::default(),
		locked_mouse_pos: PhysicalPosition::default(),
		mouse_control: false,
		key_states: KeyStates::new(),
		action_map,
		frame_update_queue: vec![],
	})
}

fn load_level(
	device: &Device, queue: &Queue, bind_group_layouts: &BindGroupLayouts, window_size: PhysicalSize<u32>,
	path: &PathBuf,
) -> Result<LoadedLevel> {
	let mut reader = BufReader::new(File::open(path)?);
	let mut version = [0; 4];
	reader.read_exact(&mut version)?;
	reader.rewind()?;
	let version = u32::from_le_bytes(version);
	let extension = path.extension().map(|e| e.to_string_lossy());
	match (version, extension.as_ref().map(|e| e.as_ref())) {
		(0x00000020, _) => parse_level::<_, tr1::Level>(
			device, queue, bind_group_layouts, window_size, &mut reader,
		),
		(0x0000002D, _) => parse_level::<_, tr2::Level>(
			device, queue, bind_group_layouts, window_size, &mut reader,
		),
		(0xFF180038, _) => parse_level::<_, tr3::Level>(
			device, queue, bind_group_layouts, window_size, &mut reader,
		),
		_ => return Err(Error::other("unknown file type")),
	}
}

fn draw_window<R, F: FnOnce(&mut egui::Ui) -> R>(ctx: &egui::Context, title: &str, contents: F) -> R {
	egui::Window::new(title)
		.collapsible(false)
		.resizable(false)
		.show(ctx, contents)
		.unwrap()
		.inner
		.unwrap()
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
	let room_combo = egui::ComboBox::from_label("Room");
	room_combo.selected_text(selected_room_text(loaded_level.render_room_index)).show_ui(ui, |ui| {
		ui.selectable_value(&mut loaded_level.render_room_index, None, selected_room_text(None));
		for render_room_index in 0..loaded_level.render_rooms.len() {
			ui.selectable_value(
				&mut loaded_level.render_room_index,
				Some(render_room_index),
				selected_room_text(Some(render_room_index)),
			);
		}
	});
	if loaded_level.texture_bind_groups.len() > 1 {
		let texture_combo = egui::ComboBox::from_label("Textures").selected_text(
			loaded_level
				.texture_bind_groups[loaded_level.texture_bind_group_index]
				.texture_type
				.label(),
		);
		texture_combo.show_index(
			ui, &mut loaded_level.texture_bind_group_index, loaded_level.texture_bind_groups.len(),
			|index| {
				loaded_level.texture_bind_groups[index].texture_type.label()
			},
		);
	}
	if let (Some(solid_bind_group_index), true) = (
		&mut loaded_level.solid_bind_group_index, loaded_level.solid_bind_groups.len() > 1
	) {
		let solid_combo = egui::ComboBox::from_label("Solid Color Palette").selected_text(
			loaded_level
				.solid_bind_groups[*solid_bind_group_index]
				.solid_type
				.label(),
		);
		solid_combo.show_index(
			ui, solid_bind_group_index, loaded_level.solid_bind_groups.len(), |index| {
				loaded_level.solid_bind_groups[index].solid_type.label()
			},
		);
	}
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
	fn resize(&mut self, window_size: PhysicalSize<u32>, device: &Device, queue: &Queue) {
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
		&mut self, window: &Window, target: &EventLoopWindowTarget<()>, key_code: KeyCode,
		state: ElementState, repeat: bool,
	) {
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
						loaded_level.locked_mouse_pos = loaded_level.mouse_pos;
						loaded_level.set_mouse_control(window, !loaded_level.mouse_control);
					}
				},
				(ElementState::Pressed, MouseButton::Left) => {
					const WIDTH_ALIGN: u32 = 256 / INTERACT_PIXEL_SIZE;
					let chunks = (loaded_level.interact_texture.width() + WIDTH_ALIGN - 1) / WIDTH_ALIGN;
					let width = chunks * WIDTH_ALIGN;
					let height = loaded_level.interact_texture.height();
					let buffer = device.create_buffer(&BufferDescriptor {
						label: None,
						size: (width * height * INTERACT_PIXEL_SIZE) as u64,
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
								bytes_per_row: Some(width * INTERACT_PIXEL_SIZE),
								rows_per_image: None,
							},
						},
						loaded_level.interact_texture.size(),
					);
					let submission_index = queue.submit([encoder.finish()]);
					buffer.slice(..).map_async(MapMode::Read, |r| r.expect("map interact texture"));
					let pos = loaded_level.mouse_pos.cast::<u32>();
					let click_handle = spawn(move || {
						device.poll(Maintain::WaitForSubmissionIndex(submission_index));
						let bytes = &*buffer.slice(..).get_mapped_range();
						let pixel_offset = pos.y * width + pos.x;
						let byte_offset = (pixel_offset * INTERACT_PIXEL_SIZE) as usize;
						InteractPixel::from_le_bytes([bytes[byte_offset], bytes[byte_offset + 1]])
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
				let pitch = (loaded_level.pitch + delta.y as f32 / 150.0).clamp(-FRAC_PI_2, FRAC_PI_2);
				loaded_level.pitch = pitch;
			}
		}
	}
	
	fn cursor_moved(&mut self, window: &Window, pos: PhysicalPosition<f64>) {
		if let Some(loaded_level) = &mut self.loaded_level {
			loaded_level.mouse_pos = pos;
			if loaded_level.mouse_control {
				window.set_cursor_position(loaded_level.locked_mouse_pos).expect("set cursor pos");
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
							load: LoadOp::Clear(Color { r: f64::MAX, g: 0.0, b: 0.0, a: 0.0 }),
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
			let render_room_range = match loaded_level.render_room_index {
				Some(render_room_index) => render_room_index..render_room_index + 1,
				None => 0..loaded_level.render_rooms.len(),
			};
			let rooms = render_room_range.map(|render_room_index| &loaded_level.rooms[
				loaded_level.render_rooms[render_room_index].0[loaded_level.render_alt_rooms as usize]
			]).collect::<Vec<_>>();
			let solid_bind_group = loaded_level.solid_bind_group_index.map(
				|solid_bind_group_index| &loaded_level.solid_bind_groups[solid_bind_group_index]
			);
			let texture_bind_group = &loaded_level.texture_bind_groups[
				loaded_level.texture_bind_group_index
			];
			let texture_pls = match texture_bind_group.texture_type {
				TextureType::Palette => &self.texture_palette_pls,
				TextureType::Direct16Bit => &self.texture_16bit_pls,
			};
			rpass.set_index_buffer(loaded_level.reverse_indices_buffer.slice(..), IndexFormat::Uint16);
			rpass.set_vertex_buffer(0, loaded_level.face_vertex_index_buffer.slice(..));
			rpass.set_vertex_buffer(1, loaded_level.face_instance_buffer.slice(..));
			if let Some(solid_bind_group) = solid_bind_group {
				let solid_pl = match solid_bind_group.solid_type {
					SolidType::Color24Bit => &self.solid_24bit_pl,
					SolidType::Color32Bit => &self.solid_32bit_pl,
				};
				rpass.set_bind_group(0, &solid_bind_group.bind_group, &[]);
				rpass.set_pipeline(solid_pl);
				for &room in &rooms {
					for mesh in &room.meshes {
						rpass.draw(0..NUM_QUAD_VERTICES, mesh.solid_quads.clone());
						rpass.draw(0..NUM_TRI_VERTICES, mesh.solid_tris.clone());
					}
				}
			}
			rpass.set_bind_group(0, &texture_bind_group.bind_group, &[]);
			rpass.set_pipeline(&texture_pls.opaque);
			for &room in &rooms {
				rpass.draw(0..NUM_QUAD_VERTICES, room.quads.original());
				rpass.draw(0..NUM_TRI_VERTICES, room.tris.original());
				rpass.draw_indexed(0..NUM_QUAD_VERTICES, 0, room.quads.reverse());
				rpass.draw_indexed(0..NUM_TRI_VERTICES, 0, room.tris.reverse());
				for mesh in &room.meshes {
					rpass.draw(0..NUM_QUAD_VERTICES, mesh.textured_quads.opaque());
					rpass.draw(0..NUM_TRI_VERTICES, mesh.textured_tris.opaque());
				}
			}
			rpass.set_pipeline(&texture_pls.additive);
			for &room in &rooms {
				rpass.draw(0..NUM_QUAD_VERTICES, room.quads.additive());
				rpass.draw(0..NUM_TRI_VERTICES, room.tris.additive());
				rpass.draw_indexed(0..NUM_QUAD_VERTICES, 0, room.quads.reverse_additive());
				rpass.draw_indexed(0..NUM_TRI_VERTICES, 0, room.tris.reverse_additive());
				for mesh in &room.meshes {
					rpass.draw(0..NUM_QUAD_VERTICES, mesh.textured_quads.additive());
					rpass.draw(0..NUM_TRI_VERTICES, mesh.textured_tris.additive());
				}
			}
			rpass.set_vertex_buffer(1, loaded_level.sprite_instance_buffer.slice(..));
			rpass.set_pipeline(&texture_pls.sprite);
			for &room in &rooms {
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
			match load_level(device, queue, &self.bind_group_layouts, self.window_size, &path) {
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
			Some(loaded_level) => {
				draw_window(ctx, "Render Options", |ui| render_options(loaded_level, ui));
			}
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
	fs_entry: &str, instance: VertexFormat, blend: Option<BlendState>,
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
				depth_write_enabled: blend.is_none(),
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
						blend,
						write_mask: ColorWrites::ALL,
					}),
					Some(ColorTargetState {
						format: INTERACT_TEXTURE_FORMAT,
						blend: None,
						write_mask: ColorWrites::ALL,
					}),
				],
			}),
			multiview: None,
		},
	)
}

const ADDITIVE_BLEND: BlendState = BlendState {
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
};

fn make_gui(device: &Device, window_size: PhysicalSize<u32>) -> TrTool {
	let shader = make::shader(device, include_str!("shader/mesh.wgsl"));
	
	let geom = (0, make::storage_layout_entry(GEOM_BUFFER_SIZE), ShaderStages::VERTEX);
	let camera = (1, make::uniform_layout_entry(size_of::<Mat4>()), ShaderStages::VERTEX);
	let perspective = (2, make::uniform_layout_entry(size_of::<Mat4>()), ShaderStages::VERTEX);
	let atlases = (3, make::texture_layout_entry(TextureViewDimension::D2Array), ShaderStages::FRAGMENT);
	let palette = (4, make::texture_layout_entry(TextureViewDimension::D1), ShaderStages::FRAGMENT);
	
	let solid = make::bind_group_layout(device, &[geom, camera, perspective, palette]);
	let texture_palette = make::bind_group_layout(device, &[geom, camera, perspective, atlases, palette]);
	let texture_direct = make::bind_group_layout(device, &[geom, camera, perspective, atlases]);
	
	let [solid_24bit_pl, solid_32bit_pl] = [
		("solid_24bit_vs_main", "solid_24bit_fs_main"), ("solid_32bit_vs_main", "solid_32bit_fs_main"),
	].map(|(vs_entry, fs_entry)| make_pipeline(
		device, &solid, &shader, vs_entry, fs_entry, VertexFormat::Uint32x2, None,
	));
	let [texture_palette_tp, texture_16bit_tp] = [
		(&texture_palette, "texture_palette_fs_main"), (&texture_direct, "texture_16bit_fs_main"),
	].map(|(bind_group_layout, fs_entry)| TexturePipelines {
		opaque: make_pipeline(
			device, bind_group_layout, &shader, "texture_vs_main", fs_entry, VertexFormat::Uint32x2,
			None,
		),
		additive: make_pipeline(
			device, bind_group_layout, &shader, "texture_vs_main", fs_entry, VertexFormat::Uint32x2,
			Some(ADDITIVE_BLEND),
		),
		sprite: make_pipeline(
			device, bind_group_layout, &shader, "sprite_vs_main", fs_entry, VertexFormat::Sint32x4,
			None,
		),
	});
	let path =
		r"C:\Program Files (x86)\Steam\steamapps\common\Tomb Raider (I)\extracted\DATA"
		// r"C:\Program Files (x86)\Steam\steamapps\common\Tomb Raider (II)\data"
		// r"C:\Program Files (x86)\Steam\steamapps\common\TombRaider (III)\data"
		// r"C:\Users\zane\Downloads\silver\trles\problem\SabatusTombRaider1_Revisited\DATA"
	;
	TrTool {
		bind_group_layouts: BindGroupLayouts { solid, texture_palette, texture_direct },
		solid_24bit_pl,
		solid_32bit_pl,
		texture_palette_pls: texture_palette_tp,
		texture_16bit_pls: texture_16bit_tp,
		window_size,
		modifiers: ModifiersState::empty(),
		file_dialog: FileDialog::new().initial_directory(path.into()),
		error: None,
		print: false,
		loaded_level: None,
	}
}

fn main() {
	let window_icon_bytes = include_bytes!("res/icon16.data");
	let taskbar_icon_bytes = include_bytes!("res/icon24.data");
	let window_icon = Icon::from_rgba(window_icon_bytes.to_vec(), 16, 16).expect("window icon");
	let taskbar_icon = Icon::from_rgba(taskbar_icon_bytes.to_vec(), 24, 24).expect("taskbar icon");
	gui::run(WINDOW_TITLE, window_icon, taskbar_icon, make_gui);
}
