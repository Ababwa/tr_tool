mod as_bytes;
mod gui;
mod make;
mod keys;
mod tr_traits;
mod vec_tail;
mod geom_buffer;
mod data_writer;
mod file_dialog;
mod object_data;

use std::{
	collections::HashMap, env, f32::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU}, fs::File,
	io::{BufReader, Error, Read, Result, Seek}, mem::{self, size_of, MaybeUninit}, ops::Range,
	path::PathBuf, slice, sync::Arc, thread::{self, JoinHandle}, time::Duration,
};
use data_writer::{DataWriter, MeshFaceOffsets, Output, RoomFaceOffsets};
use file_dialog::FileDialogWrapper;
use geom_buffer::{GeomBuffer, GEOM_BUFFER_SIZE};
use keys::{KeyGroup, KeyStates};
use as_bytes::{AsBytes, ReinterpretAsBytes};
use glam::{DVec2, EulerRot, Mat4, Vec3, Vec3Swizzles};
use gui::Gui;
use object_data::{print_object_data, ObjectData, PolyType};
use shared::min_max::{MinMax, VecMinMaxFromIterator};
use tr_model::{tr1, tr2, tr3, tr4, tr5};
use tr_traits::{
	Entity, Face, Frame, Level, LevelStore, Mesh, Model, Room, RoomGeom, RoomStaticMesh, RoomVertex,
};
use wgpu::{
	BindGroup, BindGroupLayout, BindingResource, BlendComponent, BlendFactor, BlendOperation, BlendState,
	Buffer, BufferDescriptor, BufferUsages, Color, ColorTargetState, ColorWrites, CommandEncoder,
	CommandEncoderDescriptor, Device, Extent3d, FragmentState, FrontFace, ImageCopyBuffer, ImageDataLayout,
	IndexFormat, LoadOp, Maintain, MapMode, MultisampleState, Operations, PipelineLayoutDescriptor,
	PrimitiveState, PrimitiveTopology, Queue, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
	RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, ShaderModule, ShaderStages, StoreOp,
	Texture, TextureDimension, TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
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

#[repr(C)]
struct Viewport {
	clip: [i32; 4],
	view: [i32; 4],
}

impl ReinterpretAsBytes for Viewport {}

const DATA_ENTRY: u32 = 0;
const STATICS_ENTRY: u32 = 1;
const CAMERA_ENTRY: u32 = 2;
const PERSPECTIVE_ENTRY: u32 = 3;
const PALETTE_ENTRY: u32 = 4;
const ATLASES_ENTRY: u32 = 5;
const VIEWPORT_ENTRY: u32 = 6;
const SCROLL_OFFSET_ENTRY: u32 = 7;

type InteractPixel = u32;
const INTERACT_TEXTURE_FORMAT: TextureFormat = TextureFormat::R32Uint;
const INTERACT_PIXEL_SIZE: u32 = size_of::<InteractPixel>() as u32;

const FORWARD: Vec3 = Vec3::NEG_Z;
const BACKWARD: Vec3 = Vec3::Z;
const LEFT: Vec3 = Vec3::X;
const RIGHT: Vec3 = Vec3::NEG_X;
const DOWN: Vec3 = Vec3::Y;
const UP: Vec3 = Vec3::NEG_Y;

struct ActionMap {
	forward: KeyGroup,
	backward: KeyGroup,
	left: KeyGroup,
	right: KeyGroup,
	up: KeyGroup,
	down: KeyGroup,
	fast: KeyGroup,
	slow: KeyGroup,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TextureMode {
	Palette,
	Bit16,
	Bit32,
}

impl TextureMode {
	fn label(&self) -> &'static str {
		match self {
			TextureMode::Palette => "Palette",
			TextureMode::Bit16 => "16 Bit",
			TextureMode::Bit32 => "32 Bit",
		}
	}
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SolidMode {
	Bit24,
	Bit32,
}

impl SolidMode {
	fn label(&self) -> &'static str {
		match self {
			SolidMode::Bit24 => "24 Bit",
			SolidMode::Bit32 => "32 Bit",
		}
	}
}

struct RoomMesh {
	quads: RoomFaceOffsets,
	tris: RoomFaceOffsets,
}

struct RenderRoom {
	geom: Vec<RoomMesh>,
	static_meshes: Vec<MeshFaceOffsets>,
	entity_meshes: Vec<Vec<MeshFaceOffsets>>,
	room_sprites: Range<u32>,
	entity_sprites: Range<u32>,
	center: Vec3,
	radius: f32,
}

struct FlipRoomIndices {
	original: usize,
	flipped: usize,
}

impl FlipRoomIndices {
	fn get(&self, flipped: bool) -> usize {
		if flipped {
			self.flipped
		} else {
			self.original
		}
	}
}

struct FlipGroup {
	number: u8,
	rooms: Vec<FlipRoomIndices>,
	show_flipped: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TexturesTab {
	Textures(TextureMode),
	Misc,
}

impl TexturesTab {
	fn label(&self) -> &'static str {
		match self {
			TexturesTab::Textures(texture_mode) => texture_mode.label(),
			TexturesTab::Misc => "Misc",
		}
	}
}

struct LoadedLevelShared {
	viewport_buffer: Buffer,
	palette_24bit_bg: Option<BindGroup>,
	texture_16bit_bg: Option<BindGroup>,
	texture_32bit_bg: Option<BindGroup>,
	misc_images_bg: Option<BindGroup>,
}

struct LoadedLevel {
	//render
	depth_view: TextureView,
	interact_texture: Texture,
	interact_view: TextureView,
	face_instance_buffer: Buffer,
	sprite_instance_buffer: Buffer,
	camera_transform_buffer: Buffer,
	perspective_transform_buffer: Buffer,
	scroll_offset_buffer: Buffer,
	solid_32bit_bg: Option<BindGroup>,
	shared: Arc<LoadedLevelShared>,
	solid_mode: Option<SolidMode>,
	texture_mode: TextureMode,
	//camera
	pos: Vec3,
	yaw: f32,
	pitch: f32,
	//rooms
	render_rooms: Vec<RenderRoom>,
	static_room_indices: Vec<usize>,
	flip_groups: Vec<FlipGroup>,
	render_room_index: Option<usize>,//if None, render all
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
	frame_update_queue: Vec<Box<dyn FnOnce(&mut Self) + Sync + Send>>,
	//render options
	show_room_mesh: bool,
	show_static_meshes: bool,
	show_entity_meshes: bool,
	show_room_sprites: bool,
	show_entity_sprites: bool,
	//textures
	textures_tab: TexturesTab,
	num_atlases: u32,
	num_misc_images: Option<u32>,
}

struct TexturePipelines {
	opaque: RenderPipeline,
	additive: RenderPipeline,
	sprite: RenderPipeline,
	flat: RenderPipeline,
}

type FileDialog = FileDialogWrapper<TexturesTab>;

struct TrToolShared {
	palette_pls: TexturePipelines,
	bit16_pls: TexturePipelines,
	bit32_pls: TexturePipelines,
	face_vertex_index_buffer: Buffer,
}

struct TrTool {
	//gui resources
	window: Arc<Window>,
	device: Arc<Device>,
	queue: Arc<Queue>,
	//static
	bind_group_layout: BindGroupLayout,
	solid_24bit_pl: RenderPipeline,
	solid_32bit_pl: RenderPipeline,
	shared: Arc<TrToolShared>,
	reverse_indices_buffer: Buffer,
	//state
	window_size: PhysicalSize<u32>,
	modifiers: ModifiersState,
	file_dialog: FileDialog,
	error: Option<String>,
	print: bool,
	loaded_level: Option<LoadedLevel>,
	//windows
	show_render_options_window: bool,
	show_textures_window: bool,
}

#[derive(Clone, Copy)]
enum ModelRef<'a, M> {
	Model(&'a M),
	SpriteSequence(&'a tr1::SpriteSequence),
}

#[repr(C)]
struct Statics {
	transforms_offset: u32,
	face_array_offsets_offset: u32,
	object_textures_offset: u32,
	object_texture_size: u32,
	sprite_textures_offset: u32,
	num_atlases: u32,
}

impl ReinterpretAsBytes for Statics {}

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
					LevelStore::Tr4(level) => print_object_data(level.as_ref(), &self.object_data, o_idx),
					LevelStore::Tr5(level) => print_object_data(level.as_ref(), &self.object_data, o_idx),
				}
			} else {
				self.click_handle = Some(click_handle);
			}
		}
		for update_fn in mem::take(&mut self.frame_update_queue) {
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
				* if self.key_states.any(self.action_map.fast) { 5.0 } else { 1.0 }
				* if self.key_states.any(self.action_map.slow) { 0.2 } else { 1.0 }
				* delta_time.as_secs_f32()
				* Mat4::from_rotation_y(self.yaw).transform_point3(movement);
		}
		self.update_camera_transform(queue);
	}
	
	fn render_options(&mut self, ui: &mut egui::Ui) {
		if !self.flip_groups.is_empty() {
			ui.horizontal(|ui| {
				ui.label("Flip groups");
				for flip_group in &mut self.flip_groups {
					ui.toggle_value(&mut flip_group.show_flipped, flip_group.number.to_string());
				}
			});
		}
		let old_render_room = self.render_room_index;
		egui::ComboBox::from_label("Room")
			.selected_text(selected_room_text(self.render_room_index))
			.show_ui(ui, |ui| {
				ui.selectable_value(&mut self.render_room_index, None, selected_room_text(None));
				for render_room_index in 0..self.render_rooms.len() {
					ui.selectable_value(
						&mut self.render_room_index,
						Some(render_room_index),
						selected_room_text(Some(render_room_index)),
					);
				}
			});
		if let (true, Some(render_room_index)) = {
			(self.render_room_index != old_render_room, self.render_room_index)
		} {
			let RenderRoom { center, radius, .. } = self.render_rooms[render_room_index];
			let move_camera = move |loaded_level: &mut Self| {
				loaded_level.pos = center - direction(loaded_level.yaw, loaded_level.pitch) * radius;
			};
			self.frame_update_queue.push(Box::new(move_camera));
		}
		if [
			&self.shared.palette_24bit_bg,
			&self.shared.texture_16bit_bg,
			&self.shared.texture_32bit_bg,
		].into_iter().filter(|bg| bg.is_some()).count() > 1 {
			egui::ComboBox::from_label("Texture mode")
				.selected_text(self.texture_mode.label())
				.show_ui(ui, |ui| {
					for (bg, mode) in [
						(&self.shared.palette_24bit_bg, TextureMode::Palette),
						(&self.shared.texture_16bit_bg, TextureMode::Bit16),
						(&self.shared.texture_32bit_bg, TextureMode::Bit32),
					] {
						if bg.is_some() {
							ui.selectable_value(&mut self.texture_mode, mode, mode.label());
						}
					}
				});
		}
		if let (Some(solid_mode), Some(_), Some(_)) = {
			(&mut self.solid_mode, &self.shared.palette_24bit_bg, &self.solid_32bit_bg)
		} {
			egui::ComboBox::from_label("Solid color mode")
				.selected_text(solid_mode.label())
				.show_ui(ui, |ui| {
					for mode in [SolidMode::Bit24, SolidMode::Bit32] {
						ui.selectable_value(solid_mode, mode, mode.label());
					}
				});
		}
		ui.collapsing("Object type toggles", |ui| {
			for (val, label) in [
				(&mut self.show_room_mesh, "Room mesh"),
				(&mut self.show_static_meshes, "Static meshes"),
				(&mut self.show_entity_meshes, "Entity meshes"),
				(&mut self.show_room_sprites, "Room sprites"),
				(&mut self.show_entity_sprites, "Entity sprites"),
			] {
				ui.checkbox(val, label);
			}
		});
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
		Extent3d {
			width,
			height,
			depth_or_array_layers: 1,
		},
		TextureDimension::D2,
		INTERACT_TEXTURE_FORMAT,
		TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
	)
}

struct WrittenFaceArray<'a, F> {
	index: u16,
	faces: &'a [F],
}

struct WrittenMesh<'a, L: Level + 'a> {
	textured_quads: WrittenFaceArray<'a, <L::Mesh<'a> as Mesh<'a>>::TexturedQuad>,
	textured_tris: WrittenFaceArray<'a, <L::Mesh<'a> as Mesh<'a>>::TexturedTri>,
	solid_quads: WrittenFaceArray<'a, <L::Mesh<'a> as Mesh<'a>>::SolidQuad>,
	solid_tris: WrittenFaceArray<'a, <L::Mesh<'a> as Mesh<'a>>::SolidTri>,
}

fn write_face_array<'a, F: Face>(
	geom_buffer: &mut GeomBuffer,
	vertex_array_offset: u32,
	faces: &'a [F],
) -> WrittenFaceArray<'a, F> {
	WrittenFaceArray { index: geom_buffer.write_face_array(faces, vertex_array_offset), faces }
}

fn make_atlases_view_gen<T: ReinterpretAsBytes>(
	device: &Device, queue: &Queue, atlases: &[T], format: TextureFormat, size: u32,
) -> TextureView {
	make::texture_view_with_data(
		device,
		queue,
		Extent3d {
			width: size,
			height: size,
			depth_or_array_layers: atlases.len() as u32,
		},
		TextureDimension::D2,
		format,
		TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
		atlases.as_bytes(),
	)
}

fn make_atlases_view<T>(device: &Device, queue: &Queue, atlases: &[T], format: TextureFormat) -> TextureView
where T: ReinterpretAsBytes {
	make_atlases_view_gen(device, queue, atlases, format, tr1::ATLAS_SIDE_LEN as u32)
}

fn make_palette_view<T>(device: &Device, queue: &Queue, palette: &T) -> TextureView
where T: ReinterpretAsBytes {
	make::texture_view_with_data(
		device,
		queue,
		Extent3d {
			width: size_of::<T>() as u32,
			height: 1,
			depth_or_array_layers: 1,
		},
		TextureDimension::D1,
		TextureFormat::R8Uint,
		TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
		palette.as_bytes(),
	)
}

fn parse_level<L: Level>(
	device: &Device,
	queue: &Queue,
	bind_group_layout: &BindGroupLayout,
	window_size: PhysicalSize<u32>,
	reader: &mut BufReader<File>,
) -> Result<LoadedLevel> {
	let level = unsafe {
		let mut level = Box::new(MaybeUninit::uninit());
		L::read(reader, level.as_mut_ptr())?;
		level.assume_init()
	};
	assert!(level.entities().len() <= 65536);
	//map model and sprite sequence ids to model and sprite sequence refs
	let model_id_map = level
		.models()
		.iter()
		.map(|model| (model.id() as u16, ModelRef::Model(model)))
		.chain(level.sprite_sequences().iter().map(|ss| (ss.id as u16, ModelRef::SpriteSequence(ss))))
		.collect::<HashMap<_, _>>();
	//group entities by room
	let mut room_entity_indices = vec![vec![]; level.rooms().len()];
	for (entity_index, entity) in level.entities().iter().enumerate() {
		room_entity_indices[entity.room_index() as usize].push(entity_index);
	}
	//write meshes, map tr mesh offets to meshes indices
	let mut geom_buffer = GeomBuffer::new();
	let mut written_meshes = vec![];
	let mut mesh_offset_map = HashMap::new();
	for &mesh_offset in level.mesh_offsets() {
		mesh_offset_map.entry(mesh_offset).or_insert_with(|| {
			let mesh = level.get_mesh(mesh_offset);
			let vao = geom_buffer.write_vertex_array(mesh.vertices());
			let written_mesh = WrittenMesh::<L> {
				textured_quads: write_face_array(&mut geom_buffer, vao, mesh.textured_quads()),
				textured_tris: write_face_array(&mut geom_buffer, vao, mesh.textured_tris()),
				solid_quads: write_face_array(&mut geom_buffer, vao, mesh.solid_quads()),
				solid_tris: write_face_array(&mut geom_buffer, vao, mesh.solid_tris()),
			};
			let index = written_meshes.len();
			written_meshes.push(written_mesh);
			index
		});
	}
	//write sprites (do first to ensure obj ids fit in u16)
	let mut data_writer = DataWriter::new(geom_buffer);
	let room_sprite_ranges = level.rooms().iter().enumerate().map(|(room_index, room)| {
		let room_index = room_index as u16;
		let room_sprites = data_writer.write_room_sprites(
			room.pos(),
			room.vertices(),
			room.sprites(),
			|sprite_index| ObjectData::RoomSprite { room_index, sprite_index },
		);
		let entity_sprites_start = data_writer.sprite_offset();
		for &entity_index in &room_entity_indices[room_index as usize] {
			let entity = &level.entities()[entity_index];
			if let ModelRef::SpriteSequence(ss) = model_id_map[&entity.model_id()] {
				data_writer.write_entity_sprite(entity_index as u16, entity.pos(), ss.sprite_texture_index);
			}
		}
		let entity_sprites_end = data_writer.sprite_offset();
		(room_sprites, entity_sprites_start..entity_sprites_end)
	}).collect::<Vec<_>>();
	//geom
	let mut static_room_indices = (0..level.rooms().len()).collect::<Vec<_>>();//flip rooms will be removed
	let mut flip_groups = HashMap::<u8, Vec<FlipRoomIndices>>::new();
	let render_rooms = {
		level.rooms().iter().enumerate().zip(room_entity_indices).zip(room_sprite_ranges)
	}.map(|(((room_index, room), entity_indices), (room_sprites, entity_sprites))| {
		let room_index = room_index as u16;
		let room_pos = room.pos();
		//room geom
		let geom = {
			room.geom().into_iter().enumerate()
		}.map(|(geom_index, RoomGeom { vertices, quads, tris })| {
			let geom_index = geom_index as u16;
			let vertex_array_offset = data_writer.geom_buffer.write_vertex_array(vertices);
			let transform = Mat4::from_translation(room_pos.as_vec3());
			let transform_index = data_writer.geom_buffer.write_transform(&transform);
			let quads = data_writer.write_room_face_array(
				level.as_ref(),
				vertex_array_offset,
				quads,
				transform_index,
				|face_index| {
					ObjectData::RoomFace {
						room_index,
						geom_index,
						face_type: PolyType::Quad,
						face_index,
					}
				},
			);
			let tris = data_writer.write_room_face_array(
				level.as_ref(),
				vertex_array_offset,
				tris,
				transform_index,
				|face_index| {
					ObjectData::RoomFace {
						room_index,
						geom_index,
						face_type: PolyType::Tri,
						face_index,
					}
				},
			);
			RoomMesh { quads, tris }
		}).collect::<Vec<_>>();
		//static meshes
		let room_static_meshes = {
			room.room_static_meshes().iter().enumerate()
		}.filter_map(|(room_static_mesh_index, room_static_mesh)| {
			let room_static_mesh_index = room_static_mesh_index as u16;
			let static_mesh_id = room_static_mesh.static_mesh_id();
			let maybe_static_mesh = level
				.static_meshes()
				.iter()
				.find(|static_mesh| static_mesh.id as u16 == static_mesh_id);
			let static_mesh = match maybe_static_mesh {
				Some(static_mesh) => static_mesh,
				None => {
					println!("static mesh id missing: {}", static_mesh_id);
					return None;
				},
			};
			let mesh_offset = level.mesh_offsets()[static_mesh.mesh_offset_index as usize];
			let written_mesh = &written_meshes[mesh_offset_map[&mesh_offset]];
			let translation = Mat4::from_translation(room_static_mesh.pos().as_vec3());
			let rotation = Mat4::from_rotation_y(room_static_mesh.angle() as f32 / 65536.0 * TAU);
			let transform = translation * rotation;
			let transform_index = data_writer.geom_buffer.write_transform(&transform);
			Some(data_writer.place_mesh(
				level.as_ref(),
				written_mesh,
				transform_index,
				|face_type, face_index| {
					ObjectData::RoomStaticMeshFace {
						room_index,
						room_static_mesh_index,
						face_type,
						face_index,
					}
				},
			))
		}).collect::<Vec<_>>();
		//entities
		let entity_meshes = entity_indices.into_iter().filter_map(|entity_index| {
			let entity = &level.entities()[entity_index];
			let ModelRef::Model(model) = model_id_map[&entity.model_id()] else {
				return None;
			};
			let entity_index = entity_index as u16;
			let entity_translation = Mat4::from_translation(entity.pos().as_vec3());
			let entity_rotation = Mat4::from_rotation_y(entity.angle() as f32 / 65536.0 * TAU);
			let entity_transform = entity_translation * entity_rotation;
			let frame = level.get_frame(model);
			let mut rotations = frame.iter_rotations();
			let first_translation = Mat4::from_translation(frame.offset().as_vec3());
			let first_rotation = rotations.next().expect("model has no rotations");
			let mut last_transform = first_translation * first_rotation;
			let transform = entity_transform * last_transform;
			let transform_index = data_writer.geom_buffer.write_transform(&transform);
			let mesh_offset = level.mesh_offsets()[model.mesh_offset_index() as usize];
			let mesh = &written_meshes[mesh_offset_map[&mesh_offset]];
			let mut meshes = Vec::with_capacity(model.num_meshes() as usize);
			meshes.push(
				data_writer.place_mesh(
					level.as_ref(),
					mesh,
					transform_index,
					|face_type, face_index| {
						ObjectData::EntityMeshFace {
							entity_index,
							mesh_index: 0,
							face_type,
							face_index,
						}
					},
				),
			);
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
				let mesh_offset_index = model.mesh_offset_index() as usize + mesh_node_index + 1;
				let mesh_offset = level.mesh_offsets()[mesh_offset_index];
				let mesh = &written_meshes[mesh_offset_map[&mesh_offset]];
				let translation = Mat4::from_translation(mesh_node.offset.as_vec3());
				let rotation = rotations.next().expect("model has insufficient rotations");
				last_transform = parent * translation * rotation;
				let transform = entity_transform * last_transform;
				let transform_index = data_writer.geom_buffer.write_transform(&transform);
				meshes.push(
					data_writer.place_mesh(
						level.as_ref(),
						mesh,
						transform_index,
						|face_type, face_index| {
							ObjectData::EntityMeshFace {
								entity_index,
								mesh_index: mesh_node_index as u16 + 1,
								face_type,
								face_index,
							}
						},
					),
				);
			}
			Some(meshes)
		}).collect::<Vec<_>>();
		let room_index = room_index as usize;
		if room.flip_room_index() != u16::MAX {
			let flip_room_index = room.flip_room_index() as usize;
			//unwrap: static_room_indices contains room_index until removed
			static_room_indices.remove(static_room_indices.binary_search(&room_index).unwrap());
			static_room_indices.remove(
				static_room_indices
					.binary_search(&flip_room_index)
					.expect("flip room index missing"),
			);
			flip_groups
				.entry(room.flip_group())
				.or_default()
				.push(FlipRoomIndices { original: room_index, flipped: flip_room_index });
		}
		let (center, radius) = room
			.vertices()
			.iter()
			.map(|v| v.pos())
			.min_max()
			.map(|MinMax { min, max }| {
				let center = (max + min) / 2.0;
				let radius = (max - min).max_element();
				(center, radius)
			})
			.unwrap_or_default();
		let center = center + room_pos.as_vec3();
		RenderRoom {
			geom,
			static_meshes: room_static_meshes,
			entity_meshes,
			room_sprites,
			entity_sprites,
			center,
			radius,
		}
	}).collect::<Vec<_>>();
	//data prep
	let mut flip_groups = flip_groups
		.into_iter()
		.map(|(number, rooms)| FlipGroup { number, rooms, show_flipped: false })
		.collect::<Vec<_>>();
	flip_groups.sort_by_key(|f| f.number);
	let Output {
		geom_output: geom_buffer::Output {
			data_buffer,
			transforms_offset,
			face_array_offsets_offset,
			object_textures_offset,
			sprite_textures_offset,
		},
		face_buffer,
		sprite_buffer,
		object_data,
	} = data_writer.done(level.object_textures(), level.sprite_textures());
	let num_atlases = level.num_atlases() as u32;
	let statics = Statics {
		transforms_offset,
		face_array_offsets_offset,
		object_textures_offset,
		object_texture_size: size_of::<L::ObjectTexture>() as u32 / 2,
		sprite_textures_offset,
		num_atlases,
	};
	let (yaw, pitch) = yaw_pitch(Vec3::ONE);
	let pos = render_rooms
		.first()
		.map(|&RenderRoom { center, radius, .. }| center - direction(yaw, pitch) * radius)
		.unwrap_or_default();
	let camera_transform = make_camera_transform(pos, yaw, pitch);
	let perspective_transform = make_perspective_transform(window_size);
	//buffers
	let data_buffer = make::buffer(device, &*data_buffer, BufferUsages::STORAGE);
	let statics_buffer = make::buffer(device, statics.as_bytes(), BufferUsages::UNIFORM);
	let camera_transform_buffer = make::writable_uniform(device, camera_transform.as_bytes());
	let perspective_transform_buffer = make::writable_uniform(device, perspective_transform.as_bytes());
	let viewport_buffer = make::writable_uniform(device, &[0; size_of::<Viewport>()]);
	let scroll_offset_buffer = make::writable_uniform(device, &[0; size_of::<egui::Vec2>()]);
	//entries
	let common_entries = &[
		make::entry(DATA_ENTRY, data_buffer.as_entire_binding()),
		make::entry(STATICS_ENTRY, statics_buffer.as_entire_binding()),
		make::entry(CAMERA_ENTRY, camera_transform_buffer.as_entire_binding()),
		make::entry(PERSPECTIVE_ENTRY, perspective_transform_buffer.as_entire_binding()),
		make::entry(VIEWPORT_ENTRY, viewport_buffer.as_entire_binding()),
		make::entry(SCROLL_OFFSET_ENTRY, scroll_offset_buffer.as_entire_binding()),
	][..];
	//bind groups
	let mut solid_32bit_bg = None;
	let mut palette_24bit_bg = None;
	let mut texture_16bit_bg = None;
	let mut texture_32bit_bg = None;
	let mut solid_mode = None;
	let mut texture_mode = None;
	let dummy_palette_view = make_palette_view(device, queue, &0u8);
	let dummy_palette_entry = make::entry(PALETTE_ENTRY, BindingResource::TextureView(&dummy_palette_view));
	let dummy_atlases_view = make_atlases_view_gen(device, queue, &[0u8; 2], TextureFormat::R8Uint, 1);
	let dummy_atlases_entry = make::entry(ATLASES_ENTRY, BindingResource::TextureView(&dummy_atlases_view));
	if let (Some(atlases), Some(palette)) = (level.atlases_palette(), level.palette_24bit()) {
		let palette_view = make_palette_view(device, queue, palette);
		let palette_entry = make::entry(PALETTE_ENTRY, BindingResource::TextureView(&palette_view));
		let atlases_view = make_atlases_view(device, queue, atlases, TextureFormat::R8Uint);
		let atlases_entry = make::entry(ATLASES_ENTRY, BindingResource::TextureView(&atlases_view));
		let entries = [common_entries, &[palette_entry, atlases_entry]].concat();
		let bind_group = make::bind_group(device, bind_group_layout, &entries);
		palette_24bit_bg = Some(bind_group);
		solid_mode = Some(SolidMode::Bit24);
		texture_mode = Some(TextureMode::Palette);
	}
	if let Some(palette) = level.palette_32bit() {
		let palette_view = make_palette_view(device, queue, palette);
		let palette_entry = make::entry(PALETTE_ENTRY, BindingResource::TextureView(&palette_view));
		let entries = [common_entries, &[palette_entry, dummy_atlases_entry]].concat();
		let bind_group = make::bind_group(device, bind_group_layout, &entries);
		solid_32bit_bg = Some(bind_group);
		solid_mode = Some(SolidMode::Bit32);
	}
	if let Some(atlases) = level.atlases_16bit() {
		let atlases_view = make_atlases_view(device, queue, atlases, TextureFormat::R16Uint);
		let atlases_entry = make::entry(ATLASES_ENTRY, BindingResource::TextureView(&atlases_view));
		let entries = [common_entries, &[dummy_palette_entry.clone(), atlases_entry]].concat();
		let bind_group = make::bind_group(device, bind_group_layout, &entries);
		texture_16bit_bg = Some(bind_group);
		texture_mode = Some(TextureMode::Bit16);
	}
	if let Some(atlases) = level.atlases_32bit() {
		let atlases_view = make_atlases_view(device, queue, atlases, TextureFormat::R32Uint);
		let atlases_entry = make::entry(ATLASES_ENTRY, BindingResource::TextureView(&atlases_view));
		let entries = [common_entries, &[dummy_palette_entry.clone(), atlases_entry]].concat();
		let bind_group = make::bind_group(device, bind_group_layout, &entries);
		texture_32bit_bg = Some(bind_group);
		texture_mode = Some(TextureMode::Bit32);
	}
	let texture_mode = texture_mode.unwrap();//all formats have at least one texture
	let (misc_images_bg, num_misc_images) = level.misc_images().map(|misc_images| {
		let atlases_view = make_atlases_view(device, queue, misc_images, TextureFormat::R32Uint);
		let atlases_entry = make::entry(ATLASES_ENTRY, BindingResource::TextureView(&atlases_view));
		let entries = [common_entries, &[dummy_palette_entry.clone(), atlases_entry]].concat();
		let bind_group = make::bind_group(device, bind_group_layout, &entries);
		(Some(bind_group), Some(misc_images.len() as u32))
	}).unwrap_or_default();
	let shared = Arc::new(LoadedLevelShared {
		viewport_buffer,
		palette_24bit_bg,
		texture_16bit_bg,
		texture_32bit_bg,
		misc_images_bg,
	});
	let action_map = ActionMap {
		forward: KeyGroup::new(&[KeyCode::KeyW, KeyCode::ArrowUp]),
		backward: KeyGroup::new(&[KeyCode::KeyS, KeyCode::ArrowDown]),
		left: KeyGroup::new(&[KeyCode::KeyA, KeyCode::ArrowLeft]),
		right: KeyGroup::new(&[KeyCode::KeyD, KeyCode::ArrowRight]),
		up: KeyGroup::new(&[KeyCode::KeyQ, KeyCode::PageUp]),
		down: KeyGroup::new(&[KeyCode::KeyE, KeyCode::PageDown]),
		fast: KeyGroup::new(&[KeyCode::ShiftLeft, KeyCode::ShiftRight]),
		slow: KeyGroup::new(&[KeyCode::ControlLeft, KeyCode::ControlRight]),
	};
	let interact_texture = make_interact_texture(device, window_size);
	let interact_view = interact_texture.create_view(&TextureViewDescriptor::default());
	Ok(LoadedLevel {
		depth_view: make::depth_view(device, window_size),
		interact_texture,
		interact_view,
		face_instance_buffer: make::buffer(device, face_buffer.as_bytes(), BufferUsages::VERTEX),
		sprite_instance_buffer: make::buffer(device, sprite_buffer.as_bytes(), BufferUsages::VERTEX),
		camera_transform_buffer,
		perspective_transform_buffer,
		scroll_offset_buffer,
		solid_32bit_bg,
		shared,
		solid_mode,
		texture_mode,
		pos,
		yaw,
		pitch,
		render_rooms,
		static_room_indices,
		flip_groups,
		render_room_index: None,
		object_data,
		level: level.store(),
		click_handle: None,
		mouse_pos: PhysicalPosition::default(),
		locked_mouse_pos: PhysicalPosition::default(),
		mouse_control: false,
		key_states: KeyStates::new(),
		action_map,
		frame_update_queue: vec![],
		show_room_mesh: true,
		show_static_meshes: true,
		show_entity_meshes: true,
		show_room_sprites: true,
		show_entity_sprites: true,
		textures_tab: TexturesTab::Textures(texture_mode),
		num_atlases,
		num_misc_images,
	})
}

fn load_level(
	window: &Window,
	device: &Device,
	queue: &Queue,
	win_size: PhysicalSize<u32>,
	bind_group_layout: &BindGroupLayout,
	path: &PathBuf,
) -> Result<LoadedLevel> {
	let mut reader = BufReader::new(File::open(path)?);
	let mut version = [0; 4];
	reader.read_exact(&mut version)?;
	reader.rewind()?;
	let version = u32::from_le_bytes(version);
	let extension = path
		.extension()
		.and_then(|e| e.to_str())
		.ok_or(Error::other("Failed to get file extension"))?;
	let loaded_level = match (version, extension.to_ascii_lowercase().as_str()) {
		(0x00000020, "phd") => parse_level::<tr1::Level>(device, queue, bind_group_layout, win_size, &mut reader),
		(0x0000002D, "tr2") => parse_level::<tr2::Level>(device, queue, bind_group_layout, win_size, &mut reader),
		(0xFF180038, "tr2") => parse_level::<tr3::Level>(device, queue, bind_group_layout, win_size, &mut reader),
		(0x00345254, "tr4") => parse_level::<tr4::Level>(device, queue, bind_group_layout, win_size, &mut reader),
		(0x00345254, "trc") => parse_level::<tr5::Level>(device, queue, bind_group_layout, win_size, &mut reader),
		_ => return Err(Error::other(format!("Unknown file type\nVersion: 0x{:X}", version))),
	}?;
	if let Some(file_name) = path.file_name().map(|f| f.to_string_lossy()) {
		window.set_title(&format!("{} - {}", WINDOW_TITLE, file_name));
	}
	Ok(loaded_level)
}

fn draw_window<R, F>(
	ctx: &egui::Context, title: &str, resizable: bool, open: &mut bool, contents: F,
) -> Option<R> where F: FnOnce(&mut egui::Ui) -> R {
	egui::Window::new(title).resizable(resizable).open(open).show(ctx, contents)?.inner
}

fn selected_room_text(render_room_index: Option<usize>) -> String {
	match render_room_index {
		Some(render_room_index) => format!("Room {}", render_room_index),
		None => "All".to_string(),
	}
}

struct TexturesCallback {
	queue: Arc<Queue>,
	tr_tool_shared: Arc<TrToolShared>,
	loaded_level_shared: Arc<LoadedLevelShared>,
	textures_tab: TexturesTab,
}

impl egui_wgpu::CallbackTrait for TexturesCallback {
	fn paint<'a>(
		&'a self, info: egui::PaintCallbackInfo, rpass: &mut wgpu::RenderPass<'a>,
		_: &'a egui_wgpu::CallbackResources,
	) {
		let cp = info.clip_rect_in_pixels();
		let vp = info.viewport_in_pixels();
		let viewport = Viewport {
			clip: [cp.left_px, cp.top_px, cp.width_px, cp.height_px],
			view: [vp.left_px, vp.top_px, vp.width_px, vp.height_px],
		};
		self.queue.write_buffer(&self.loaded_level_shared.viewport_buffer, 0, viewport.as_bytes());
		rpass.set_vertex_buffer(0, self.tr_tool_shared.face_vertex_index_buffer.slice(..));
		let tt = &self.tr_tool_shared;
		let ll = &self.loaded_level_shared;
		let (texture_pls, bind_group) = match self.textures_tab {
			TexturesTab::Textures(TextureMode::Palette) => (&tt.palette_pls, &ll.palette_24bit_bg),
			TexturesTab::Textures(TextureMode::Bit16) => (&tt.bit16_pls, &ll.texture_16bit_bg),
			TexturesTab::Textures(TextureMode::Bit32) => (&tt.bit32_pls, &ll.texture_32bit_bg),
			TexturesTab::Misc => (&tt.bit32_pls, &ll.misc_images_bg),
		};
		let bind_group = bind_group.as_ref().unwrap();//texture can't be selected unless it exists
		rpass.set_pipeline(&texture_pls.flat);
		rpass.set_bind_group(0, bind_group, &[]);
		rpass.draw(0..NUM_QUAD_VERTICES, 0..1);
	}
}

fn palette_images_to_rgba(palette: &[tr1::Color24Bit; tr1::PALETTE_LEN], atlases: &[[u8; tr1::ATLAS_PIXELS]]) -> Vec<u8> {
	atlases
		.iter()
		.flatten()
		.map(|&color_index| {
			let tr1::Color24Bit { r, g, b } = palette[color_index as usize];
			let [r, g, b] = [r, g, b].map(|c| c << 2);
			[r, g, b, (color_index != 0) as u8 * 255]
		})
		.flatten()
		.collect::<Vec<_>>()
}

fn bit16_images_to_rgba(atlases: &[[tr2::Color16BitArgb; tr1::ATLAS_PIXELS]]) -> Vec<u8> {
	atlases
		.iter()
		.flatten()
		.map(|color| {
			let [r, g, b] = [color.r(), color.g(), color.b()].map(|c| c << 3);
			[r, g, b, color.a() as u8 * 255]
		})
		.flatten()
		.collect::<Vec<_>>()
}

fn bit32_images_to_rgba(atlases: &[[tr4::Color32BitBgra; tr1::ATLAS_PIXELS]]) -> Vec<u8> {
	atlases
		.iter()
		.flatten()
		.map(|&tr4::Color32BitBgra { b, g, r, a }| [r, g, b, a])
		.flatten()
		.collect::<Vec<_>>()
}

impl Gui for TrTool {
	fn resize(&mut self, window_size: PhysicalSize<u32>) {
		self.window_size = window_size;
		if let Some(loaded_level) = &mut self.loaded_level {
			loaded_level.depth_view = make::depth_view(&self.device, window_size);
			loaded_level.interact_texture = make_interact_texture(&self.device, window_size);
			loaded_level.interact_view = loaded_level
				.interact_texture
				.create_view(&TextureViewDescriptor::default());
			loaded_level.update_perspective_transform(&self.queue, window_size);
		}
	}
	
	fn modifiers(&mut self, modifers: ModifiersState) {
		self.modifiers = modifers;
	}
	
	fn key(
		&mut self, target: &EventLoopWindowTarget<()>, key_code: KeyCode, state: ElementState, repeat: bool,
	) {
		if let Some(loaded_level) = &mut self.loaded_level {
			loaded_level.key_states.set(key_code, state.is_pressed());
		}
		match (self.modifiers, state, key_code, repeat, &mut self.loaded_level) {
			(_, ElementState::Pressed, KeyCode::Escape, false, _) => target.exit(),
			(_, ElementState::Pressed, KeyCode::KeyP, _, _) => self.print = true,
			(ModifiersState::CONTROL, ElementState::Pressed, KeyCode::KeyO, false, _) => {
				if let Some(loaded_level) = &mut self.loaded_level {
					loaded_level.set_mouse_control(&self.window, false);
				}
				self.file_dialog.select_level();
			},
			(_, ElementState::Pressed, KeyCode::KeyR, false, Some(_)) => {
				self.show_render_options_window ^= true;
			},
			(_, ElementState::Pressed, KeyCode::KeyT, false, Some(_)) => self.show_textures_window ^= true,
			_ => {},
		}
	}
	
	fn mouse_button(&mut self, button: MouseButton, state: ElementState) {
		if let Some(loaded_level) = &mut self.loaded_level {
			match (state, button) {
				(ElementState::Pressed, MouseButton::Right) => {
					if self.file_dialog.is_closed() {
						loaded_level.locked_mouse_pos = loaded_level.mouse_pos;
						loaded_level.set_mouse_control(&self.window, !loaded_level.mouse_control);
					}
				},
				(ElementState::Pressed, MouseButton::Left) => {
					const WIDTH_ALIGN: u32 = 256 / INTERACT_PIXEL_SIZE;
					let chunks = (loaded_level.interact_texture.width() + WIDTH_ALIGN - 1) / WIDTH_ALIGN;
					let width = chunks * WIDTH_ALIGN;
					let height = loaded_level.interact_texture.height();
					let buffer = self.device.create_buffer(&BufferDescriptor {
						label: None,
						size: (width * height * INTERACT_PIXEL_SIZE) as u64,
						usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
						mapped_at_creation: false,
					});
					let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor::default());
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
					let submission_index = self.queue.submit([encoder.finish()]);
					buffer.slice(..).map_async(MapMode::Read, |r| r.expect("map interact texture"));
					let pos = loaded_level.mouse_pos.cast::<u32>();
					let device = self.device.clone();
					let click_handle = thread::spawn(move || {
						device.poll(Maintain::WaitForSubmissionIndex(submission_index));
						let bytes = &*buffer.slice(..).get_mapped_range();
						let pixel_offset = pos.y * width + pos.x;
						let byte_offset = (pixel_offset * INTERACT_PIXEL_SIZE) as usize;
						InteractPixel::from_le_bytes([
							bytes[byte_offset],
							bytes[byte_offset + 1],
							bytes[byte_offset + 2],
							bytes[byte_offset + 3],
						])
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
	
	fn cursor_moved(&mut self, pos: PhysicalPosition<f64>) {
		if let Some(loaded_level) = &mut self.loaded_level {
			loaded_level.mouse_pos = pos;
			if loaded_level.mouse_control {
				self.window.set_cursor_position(loaded_level.locked_mouse_pos).expect("set cursor pos");
			}
		}
	}
	
	fn mouse_wheel(&mut self, _: MouseScrollDelta) {}
	
	fn render(
		&mut self, encoder: &mut CommandEncoder, color_view: &TextureView, delta_time: Duration,
		last_render_time: Duration,
	) {
		if let Some(loaded_level) = &mut self.loaded_level {
			loaded_level.frame_update(&self.queue, delta_time);
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
			let room_indices = match loaded_level.render_room_index {
				Some(render_room_index) => vec![render_room_index],
				None => loaded_level
					.flip_groups
					.iter()
					.map(|f| f.rooms.iter().map(|r| r.get(f.show_flipped)))
					.flatten()
					.chain(loaded_level.static_room_indices.iter().copied())
					.collect(),
			};
			let rooms = room_indices
				.into_iter()
				.map(|room_index| &loaded_level.render_rooms[room_index])
				.collect::<Vec<_>>();
			let solid = loaded_level.solid_mode.as_ref().map(|solid_mode| {
				let (solid_pl, solid_bg) = match solid_mode {
					SolidMode::Bit24 => (&self.solid_24bit_pl, &loaded_level.shared.palette_24bit_bg),
					SolidMode::Bit32 => (&self.solid_32bit_pl, &loaded_level.solid_32bit_bg),
				};
				(solid_pl, solid_bg.as_ref().unwrap())
			});
			let (texture_pls, texture_bg) = match loaded_level.texture_mode {
				TextureMode::Palette => (&self.shared.palette_pls, &loaded_level.shared.palette_24bit_bg),
				TextureMode::Bit16 => (&self.shared.bit16_pls, &loaded_level.shared.texture_16bit_bg),
				TextureMode::Bit32 => (&self.shared.bit32_pls, &loaded_level.shared.texture_32bit_bg),
			};
			let texture_bg = texture_bg.as_ref().unwrap();
			
			rpass.set_index_buffer(self.reverse_indices_buffer.slice(..), IndexFormat::Uint16);
			rpass.set_vertex_buffer(0, self.shared.face_vertex_index_buffer.slice(..));
			rpass.set_vertex_buffer(1, loaded_level.face_instance_buffer.slice(..));
			if let Some((solid_pl, solid_bg)) = solid {
				rpass.set_bind_group(0, solid_bg, &[]);
				rpass.set_pipeline(solid_pl);
				if loaded_level.show_static_meshes {
					for &room in &rooms {
						for mesh in &room.static_meshes {
							rpass.draw(0..NUM_QUAD_VERTICES, mesh.solid_quads.clone());
							rpass.draw(0..NUM_TRI_VERTICES, mesh.solid_tris.clone());
						}
					}
				}
				if loaded_level.show_entity_meshes {
					for &room in &rooms {
						for mesh in room.entity_meshes.iter().flatten() {
							rpass.draw(0..NUM_QUAD_VERTICES, mesh.solid_quads.clone());
							rpass.draw(0..NUM_TRI_VERTICES, mesh.solid_tris.clone());
						}
					}
				}
			}
			rpass.set_bind_group(0, texture_bg, &[]);
			rpass.set_pipeline(&texture_pls.opaque);
			for &room in &rooms {
				if loaded_level.show_room_mesh {
					for RoomMesh { quads, tris } in &room.geom {
						rpass.draw(0..NUM_QUAD_VERTICES, quads.opaque_obverse());
						rpass.draw(0..NUM_TRI_VERTICES, tris.opaque_obverse());
						rpass.draw_indexed(0..NUM_QUAD_VERTICES, 0, quads.opaque_reverse());
						rpass.draw_indexed(0..NUM_TRI_VERTICES, 0, tris.opaque_reverse());
					}
				}
				if loaded_level.show_static_meshes {
					for mesh in &room.static_meshes {
						rpass.draw(0..NUM_QUAD_VERTICES, mesh.textured_quads.opaque());
						rpass.draw(0..NUM_TRI_VERTICES, mesh.textured_tris.opaque());
					}
				}
				if loaded_level.show_entity_meshes {
					for mesh in room.entity_meshes.iter().flatten() {
						rpass.draw(0..NUM_QUAD_VERTICES, mesh.textured_quads.opaque());
						rpass.draw(0..NUM_TRI_VERTICES, mesh.textured_tris.opaque());
					}
				}
			}
			rpass.set_pipeline(&texture_pls.additive);
			for &room in &rooms {
				if loaded_level.show_room_mesh {
					for RoomMesh { quads, tris } in &room.geom {
						rpass.draw(0..NUM_QUAD_VERTICES, quads.additive_obverse());
						rpass.draw(0..NUM_TRI_VERTICES, tris.additive_obverse());
						rpass.draw_indexed(0..NUM_QUAD_VERTICES, 0, quads.additive_reverse());
						rpass.draw_indexed(0..NUM_TRI_VERTICES, 0, tris.additive_reverse());
					}
				}
				if loaded_level.show_static_meshes {
					for mesh in &room.static_meshes {
						rpass.draw(0..NUM_QUAD_VERTICES, mesh.textured_quads.additive());
						rpass.draw(0..NUM_TRI_VERTICES, mesh.textured_tris.additive());
					}
				}
				if loaded_level.show_entity_meshes {
					for mesh in room.entity_meshes.iter().flatten() {
						rpass.draw(0..NUM_QUAD_VERTICES, mesh.textured_quads.additive());
						rpass.draw(0..NUM_TRI_VERTICES, mesh.textured_tris.additive());
					}
				}
			}
			rpass.set_vertex_buffer(1, loaded_level.sprite_instance_buffer.slice(..));
			rpass.set_pipeline(&texture_pls.sprite);
			if loaded_level.show_room_sprites {
				for &room in &rooms {
					rpass.draw(0..NUM_QUAD_VERTICES, room.room_sprites.clone());
				}
			}
			if loaded_level.show_entity_sprites {
				for &room in &rooms {
					rpass.draw(0..NUM_QUAD_VERTICES, room.entity_sprites.clone());
				}
			}
		}
		if self.print {
			println!("render time: {}us", last_render_time.as_micros());
		}
	}
	
	fn gui(&mut self, ctx: &egui::Context) {
		self.file_dialog.update(ctx);
		if let Some(path) = self.file_dialog.get_level_path() {
			match load_level(&self.window, &self.device, &self.queue, self.window_size, &self.bind_group_layout, &path) {
				Ok(loaded_level) => self.loaded_level = Some(loaded_level),
				Err(e) => self.error = Some(e.to_string()),
			}
		}
		match &mut self.loaded_level {
			None => {
				egui::panel::CentralPanel::default().show(ctx, |ui| {
					ui.centered_and_justified(|ui| {
						if ui.label("Ctrl+O or click to open file").clicked() {
							self.file_dialog.select_level();
						}
					});
				});
			},
			Some(loaded_level) => {
				draw_window(ctx, "Render Options", false, &mut self.show_render_options_window, |ui| {
					loaded_level.render_options(ui)
				});
				draw_window(ctx, "Textures", true, &mut self.show_textures_window, |ui| {
					let ll = &loaded_level.shared;
					let bind_groups = [
						&ll.palette_24bit_bg,
						&ll.texture_16bit_bg,
						&ll.texture_32bit_bg,
						&ll.misc_images_bg,
					];
					if bind_groups.into_iter().filter(|bg| bg.is_some()).count() > 1 {
						ui.horizontal(|ui| {
							for (bg, tab) in [
								(&ll.palette_24bit_bg, TexturesTab::Textures(TextureMode::Palette)),
								(&ll.texture_16bit_bg, TexturesTab::Textures(TextureMode::Bit16)),
								(&ll.texture_32bit_bg, TexturesTab::Textures(TextureMode::Bit32)),
								(&ll.misc_images_bg, TexturesTab::Misc),
							] {
								if bg.is_some() {
									ui.selectable_value(&mut loaded_level.textures_tab, tab, tab.label());
								}
							}
						});
					}
					if ui.button("Save").clicked() {
						self.file_dialog.save_texture(loaded_level.textures_tab);
					}
					ui.add_space(2.0);
					let (num_images, id): (_, u8) = match loaded_level.textures_tab {
						TexturesTab::Textures(_) => (loaded_level.num_atlases, 0),
						TexturesTab::Misc => (loaded_level.num_misc_images.unwrap(), 1),
					};
					let scroll_output = egui::ScrollArea::vertical().id_source(id).show(ui, |ui| {
						const WIDTH: f32 = tr1::ATLAS_SIDE_LEN as f32;
						let height = (num_images * 256) as f32;
						let (_, rect) = ui.allocate_space(egui::vec2(WIDTH, height));
						let textures_cb = TexturesCallback {
							queue: self.queue.clone(),
							tr_tool_shared: self.shared.clone(),
							loaded_level_shared: loaded_level.shared.clone(),
							textures_tab: loaded_level.textures_tab,
						};
						ui.painter().add(egui_wgpu::Callback::new_paint_callback(rect, textures_cb));
					});
					let scroll_offset_bytes = scroll_output.state.offset.as_bytes();
					self.queue.write_buffer(&loaded_level.scroll_offset_buffer, 0, scroll_offset_bytes);
				});
				if let Some((path, texture)) = self.file_dialog.get_texture_path() {
					let level = loaded_level.level.as_dyn();
					let rgba = match texture {
						TexturesTab::Textures(TextureMode::Palette) => {
							let palette = level.palette_24bit().unwrap();
							let atlases = level.atlases_palette().unwrap();
							palette_images_to_rgba(palette, atlases)
						},
						TexturesTab::Textures(TextureMode::Bit16) => {
							let atlases = level.atlases_16bit().unwrap();
							bit16_images_to_rgba(atlases)
						},
						TexturesTab::Textures(TextureMode::Bit32) => {
							let atlases = level.atlases_32bit().unwrap();
							bit32_images_to_rgba(atlases)
						},
						TexturesTab::Misc => {
							let images = level.misc_images().unwrap();
							bit32_images_to_rgba(images)
						},
					};
					let result = image::save_buffer(
						path,
						&rgba,
						tr1::ATLAS_SIDE_LEN as u32,
						(rgba.len() / (tr1::ATLAS_SIDE_LEN * 4)) as u32,
						image::ColorType::Rgba8,
					);
					if let Err(e) = result {
						self.error = Some(e.to_string());
					}
				}
			}
		}
		if let Some(error) = &self.error {
			let mut show = true;
			draw_window(ctx, "Error", false, &mut show, |ui| ui.label(error));
			if !show {
				self.error = None;
			}
		}
		self.print = false;
	}
}

const FACE_INSTANCE_FORMAT: VertexFormat = VertexFormat::Uint32x3;

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

const INTERACT_TARGET: ColorTargetState = ColorTargetState {
	format: INTERACT_TEXTURE_FORMAT,
	blend: None,
	write_mask: ColorWrites::ALL,
};

fn make_pipeline(
	device: &Device,
	bind_group_layout: &BindGroupLayout,
	module: &ShaderModule,
	vs_entry: &str,
	fs_entry: &str,
	instance: Option<VertexFormat>,
	cull_mode: Option<wgpu::Face>,
	blend: Option<BlendState>,
	interact: Option<ColorTargetState>,
	depth: bool,
) -> RenderPipeline {
	let vertex_step = (VertexStepMode::Vertex, &[VertexFormat::Uint32][..]);
	let vertex_steps = match instance.as_ref() {
		Some(instance) => &[vertex_step, (VertexStepMode::Instance, slice::from_ref(instance))][..],
		None => &[vertex_step],
	};
	let color_target = Some(ColorTargetState {
		format: TextureFormat::Bgra8Unorm,
		blend,
		write_mask: ColorWrites::ALL,
	});
	let targets = if interact.is_some() {
		&[color_target, interact][..]
	} else {
		&[color_target]
	};
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
					vertex_steps,
				),
			},
			primitive: PrimitiveState {
				topology: PrimitiveTopology::TriangleStrip,
				cull_mode,
				front_face: FrontFace::Cw,
				strip_index_format: None,
				..PrimitiveState::default()//other fields require features
			},
			depth_stencil: depth.then(|| make::depth_stencil_state(blend.is_none())),
			multisample: MultisampleState::default(),
			fragment: Some(FragmentState {
				entry_point: fs_entry,
				module,
				targets,
			}),
			multiview: None,
		},
	)
}

fn make_gui(
	window: Arc<Window>, device: Arc<Device>, queue: Arc<Queue>, window_size: PhysicalSize<u32>,
) -> TrTool {
	let shader = make::shader(&device, include_str!("shader/mesh.wgsl"));
	let entries = [
		(DATA_ENTRY, make::storage_layout_entry(GEOM_BUFFER_SIZE), ShaderStages::VERTEX),
		(STATICS_ENTRY, make::uniform_layout_entry(size_of::<Statics>()), ShaderStages::VERTEX),
		(CAMERA_ENTRY, make::uniform_layout_entry(size_of::<Mat4>()), ShaderStages::VERTEX),
		(PERSPECTIVE_ENTRY, make::uniform_layout_entry(size_of::<Mat4>()), ShaderStages::VERTEX),
		(PALETTE_ENTRY, make::texture_layout_entry(TextureViewDimension::D1), ShaderStages::FRAGMENT),
		(ATLASES_ENTRY, make::texture_layout_entry(TextureViewDimension::D2Array), ShaderStages::FRAGMENT),
		(VIEWPORT_ENTRY, make::uniform_layout_entry(size_of::<Viewport>()), ShaderStages::VERTEX),
		(SCROLL_OFFSET_ENTRY, make::uniform_layout_entry(size_of::<egui::Vec2>()), ShaderStages::VERTEX),
	];
	let bind_group_layout = make::bind_group_layout(&device, &entries);
	//pipelines
	let [solid_24bit_pl, solid_32bit_pl] = [
		("solid_24bit_vs_main", "solid_24bit_fs_main"), ("solid_32bit_vs_main", "solid_32bit_fs_main"),
	].map(|(vs_entry, fs_entry)| {
		make_pipeline(
			&device,
			&bind_group_layout,
			&shader,
			vs_entry,
			fs_entry,
			Some(FACE_INSTANCE_FORMAT),
			Some(wgpu::Face::Back),
			None,
			Some(INTERACT_TARGET),
			true,
		)
	});
	let texture_modes = [
		("texture_palette_fs_main", "flat_palette_fs_main"),
		("texture_16bit_fs_main", "flat_16bit_fs_main"),
		("texture_32bit_fs_main", "flat_32bit_fs_main"),
	];
	let render_modes = [
		("texture_vs_main", FACE_INSTANCE_FORMAT, None),
		("texture_vs_main", FACE_INSTANCE_FORMAT, Some(ADDITIVE_BLEND)),
		("sprite_vs_main", VertexFormat::Sint32x4, None),
	];
	let [palette_pls, bit16_pls, bit32_pls] = texture_modes.map(|(tex_fs_entry, flat_fs_entry)| {
		let [opaque, additive, sprite] = render_modes.map(|(vs_entry, instance, blend)| {
			make_pipeline(
				&device,
				&bind_group_layout,
				&shader,
				vs_entry,
				tex_fs_entry,
				Some(instance),
				Some(wgpu::Face::Back),
				blend,
				Some(INTERACT_TARGET),
				true,
			)
		});
		let flat = make_pipeline(
			&device,
			&bind_group_layout,
			&shader,
			"flat_vs_main",
			flat_fs_entry,
			None,
			None,
			None,
			None,
			false,
		);
		TexturePipelines { opaque, additive, sprite, flat }
	});
	let face_vertex_index_buffer = make::buffer(&device, FACE_VERTEX_INDICES.as_bytes(), BufferUsages::VERTEX);
	let reverse_indices_buffer = make::buffer(&device, REVERSE_INDICES.as_bytes(), BufferUsages::INDEX);
	let mut loaded_level = None;
	if let Some(arg) = env::args().skip(1).next() {
		match load_level(&window, &device, &queue, window_size, &bind_group_layout, &arg.into()) {
			Ok(level) => loaded_level = Some(level),
			Err(e) => eprintln!("{}", e),
		}
	}
	let shared = Arc::new(TrToolShared { palette_pls, bit16_pls, bit32_pls, face_vertex_index_buffer });
	TrTool {
		window,
		device,
		queue,
		bind_group_layout,
		solid_24bit_pl,
		solid_32bit_pl,
		shared,
		reverse_indices_buffer,
		window_size,
		modifiers: ModifiersState::empty(),
		file_dialog: FileDialog::new(),
		error: None,
		print: false,
		loaded_level,
		show_render_options_window: true,
		show_textures_window: false,
	}
}

fn main() {
	let window_icon_bytes = include_bytes!("res/icon16.data");
	let taskbar_icon_bytes = include_bytes!("res/icon24.data");
	let window_icon = Icon::from_rgba(window_icon_bytes.to_vec(), 16, 16).expect("window icon");
	let taskbar_icon = Icon::from_rgba(taskbar_icon_bytes.to_vec(), 24, 24).expect("taskbar icon");
	gui::run(WINDOW_TITLE, window_icon, taskbar_icon, make_gui);
}
