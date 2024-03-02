use std::{collections::HashMap, io::{Cursor, Read, Result}, slice};
use bitfield::bitfield;
use byteorder::{ReadBytesExt, LE};
use glam::{i16vec3, ivec3, I16Vec3, IVec3, U16Vec2, U16Vec3, Vec3};
use nonmax::{NonMaxU16, NonMaxU8};
use shared::geom::MinMax;
use crate::{get_zlib, read_boxed_slice, read_list, Readable};

// 1 sector unit = 1024 world coord units

pub const IMAGE_SIZE: usize = 256;
pub const NUM_PIXELS: usize = IMAGE_SIZE * IMAGE_SIZE;

pub struct Images {
	pub num_room_images: u16,
	pub num_obj_images: u16,
	pub num_bump_maps: u16,
	pub images32: Box<[[u8; NUM_PIXELS * 4]]>,
	pub images16: Box<[[u8; NUM_PIXELS * 2]]>,
	pub misc_images: Box<[[u8; NUM_PIXELS * 4]; 2]>,
}

fn read_big_zlib<R: Read, const N: usize>(reader: &mut R, len: usize) -> Result<Box<[[u8; N]]>> {
	let mut reader = get_zlib(reader)?;
	let mut vec = Vec::with_capacity(len);
	unsafe {//no safe way to do this currently
		vec.set_len(len);
		let buf = slice::from_raw_parts_mut(vec.as_mut_ptr() as *mut u8, len * N);
		reader.read_exact(buf)?;
	}
	Ok(vec.into_boxed_slice())
}

impl Readable for Images {
	fn read<R: Read>(reader: &mut R) -> Result<Self> {
		let num_room_images = reader.read_u16::<LE>()?;
		let num_obj_images = reader.read_u16::<LE>()?;
		let num_bump_maps = reader.read_u16::<LE>()?;
		let num_images = (num_room_images + num_obj_images + num_bump_maps) as usize;
		let images32 = read_big_zlib(reader, num_images)?;
		let images16 = read_big_zlib(reader, num_images)?;
		let misc_images = read_big_zlib(reader, 2)?.try_into().ok().unwrap();//exactly 2
		Ok(Images {
			num_room_images,
			num_obj_images,
			num_bump_maps,
			images32,
			images16,
			misc_images,
		})
	}
}

#[derive(Readable, Clone, Copy)]
pub struct RoomVertex {
	/// Relative to Room
	pub vertex: I16Vec3,
	#[skip_2]
	pub flags: u16,
	pub color: u16,
}

bitfield! {
	#[derive(Readable, Clone, Copy)]
	pub struct TextureDetails(u16);
	/// Index into LevelData.object_textures
	pub texture_index, _: 14, 0;
	pub double_sided, _: 15;
}

#[derive(Readable, Clone, Copy)]
pub struct Face<const N: usize> {
	pub vertex_indices: [u16; N],
	pub texture_details: TextureDetails,
}

#[derive(Readable, Clone, Copy)]
pub struct Sprite {
	/// Id? into Room.vertices
	pub vertex_id: u16,
	/// Id? into LevelData.sprite_textures
	pub texture_id: u16,
}

#[derive(Readable, Clone, Copy)]
pub struct Portal {
	/// Index into LevelData.rooms
	pub adjoining_room_index: u16,
	pub normal: I16Vec3,
	/// Relative to Room
	pub vertices: [I16Vec3; 4],
}

bitfield! {
	#[derive(Readable, Clone, Copy)]
	pub struct SectorMaterialAndBox(u16);
	/// Footstep sound
	pub material, _: 3, 0;
	/// Id? into BoxData.boxes
	pub box_id, _: 14, 4;
}

#[derive(Readable, Clone, Copy)]
pub struct Sector {
	/// Id? into LevelData.floor_data
	pub floor_data_id: u16,
	pub material_and_box: SectorMaterialAndBox,
	/// Id? into LevelData.Rooms
	pub room_below_id: Option<NonMaxU8>,
	pub floor: i8,
	/// Id? into LevelData.Rooms
	pub room_above_id: Option<NonMaxU8>,
	pub ceiling: i8,
}

#[derive(Readable, Clone, Copy)]
pub struct Light {
	/// World coords
	pub pos: IVec3,
	// Color
	pub r: u8,
	pub g: u8,
	pub b: u8,
	pub light_type: u8,
	#[skip_1]
	pub intensity: u8,
	pub hotspot: f32,
	pub falloff: f32,
	pub length: f32,
	pub cutoff: f32,
	pub direction: Vec3,
}

#[derive(Readable, Clone, Copy)]
pub struct RoomStaticMesh {
	/// World coords
	pub pos: IVec3,
	/// Units are 1/65536th of a rotation
	pub rotation: u16,
	pub color: u16,
	/// Id into LevelData.static_meshes
	#[skip_2]
	pub static_mesh_id: u16,
}

pub struct Sectors {
	pub sectors_dim: U16Vec2,
	pub sectors: Box<[Sector]>,
}

impl Readable for Sectors {
	fn read<R: Read>(reader: &mut R) -> Result<Self> {
		let sectors_dim = U16Vec2::read(reader)?;
		let sectors = read_boxed_slice(reader, sectors_dim.element_product() as usize)?;
		Ok(Sectors { sectors_dim, sectors })
	}
}

bitfield! {
	#[derive(Readable, Clone, Copy)]
	pub struct RoomFlags(u16);
	pub water, _: 0;
	pub skybox, _: 3;
}

#[derive(Readable)]
pub struct Room {
	/// World coord
	pub x: i32,
	/// World coord
	pub z: i32,
	pub y_bottom: i32,
	pub y_top: i32,
	#[skip_4]
	#[list_u16]
	pub vertices: Box<[RoomVertex]>,
	/// `vertex_indices` index into Room.vertices
	#[list_u16]
	pub quads: Box<[Face<4>]>,
	/// `vertex_indices` index into Room.vertices
	#[list_u16]
	pub triangles: Box<[Face<3>]>,
	#[list_u16]
	pub sprites: Box<[Sprite]>,
	#[list_u16]
	pub portals: Box<[Portal]>,
	pub sectors: Sectors,
	/// ARGB
	pub color: u32,
	#[list_u16]
	pub lights: Box<[Light]>,
	#[list_u16]
	pub room_static_meshes: Box<[RoomStaticMesh]>,
	/// Index into LevelData.rooms
	pub flip_room_index: Option<NonMaxU16>,
	pub flags: RoomFlags,
	pub water_effect: u8,
	pub reverb: u8,
	pub flip_group: u8,
}

pub enum MeshComponent {
	Normals(Box<[I16Vec3]>),
	Lights(Box<[u16]>),
}

impl Readable for MeshComponent {
	fn read<R: Read>(reader: &mut R) -> Result<Self> {
		Ok(match reader.read_i16::<LE>()? {
			num if num > 0 => MeshComponent::Normals(read_boxed_slice(reader, num as usize)?),
			num => MeshComponent::Lights(read_boxed_slice(reader, (-num) as usize)?),
		})
	}
}

bitfield! {
	#[derive(Readable, Clone, Copy)]
	pub struct MeshEffects(u16);
	pub additive, _: 0;
	pub shiny, _: 1;
	pub shine_strength, _: 7, 2;
}

#[derive(Readable, Clone, Copy)]
pub struct MeshFace<const N: usize> {
	/// Vertex_ids id into Mesh.vertices
	pub face: Face<N>,
	pub effects: MeshEffects,
}

#[derive(Readable)]
pub struct Mesh {
	pub center: I16Vec3,
	pub radius: i32,
	/// Relative to RoomStaticMesh.pos if static mesh
	#[list_u16]
	pub vertices: Box<[I16Vec3]>,
	pub component: MeshComponent,
	#[list_u16]
	pub quads: Box<[MeshFace<4>]>,
	#[list_u16]
	pub triangles: Box<[MeshFace<3>]>,
}

pub struct Meshes {
	pub meshes: Box<[Mesh]>,
	pub index_map: Box<[usize]>,
}

impl Meshes {
	pub fn get_mesh(&self, mesh_id: u16) -> &Mesh {
		&self.meshes[self.index_map[mesh_id as usize]]
	}
}

impl Readable for Meshes {
	fn read<R: Read>(reader: &mut R) -> Result<Self> {
		let num_mesh_bytes = 2 * reader.read_u32::<LE>()? as usize;
		let mesh_bytes = read_boxed_slice::<_, u8>(reader, num_mesh_bytes)?;
		let mut offset_map = HashMap::new();
		let mut index = 0..;
		let index_map = read_list::<_, u32, u32>(reader)?
			.into_vec()
			.into_iter()
			.map(|offset| *offset_map.entry(offset).or_insert_with(|| index.next().unwrap()))
			.collect::<Vec<_>>()
			.into_boxed_slice();
		let mut offset_map = offset_map.into_iter().collect::<Vec<_>>();
		offset_map.sort_by_key(|&(_, index)| index);
		let meshes = offset_map
			.into_iter()
			.map(|(offset, _)| Mesh::read(&mut Cursor::new(&mesh_bytes[offset as usize..])))
			.collect::<Result<Vec<_>>>()?
			.into_boxed_slice();
		Ok(Meshes { meshes, index_map })
	}
}

#[derive(Readable, Clone, Copy)]
pub struct Anim {
	/// Byte offset into LevelData.frame_data
	pub frame_byte_offset: u32,
	/// 30ths of a second
	pub frame_duration: u8,
	pub num_frames: u8,
	pub state: u16,
	/// Fixed-point
	pub speed: u32,
	/// Fixed-point
	pub accel: u32,
	/// Fixed-point
	pub lateral_speed: u32,
	/// Fixed-point
	pub lateral_accel: u32,
	pub frame_start: u16,
	pub frame_end: u16,
	pub next_anim: u16,
	pub next_frame: u16,
	pub num_state_changes: u16,
	/// Id? into LevelData.state_changes
	pub state_change_id: u16,
	pub num_anim_commands: u16,
	/// Id? into LevelData.anim_commands
	pub anim_command_id: u16,
}

#[derive(Readable, Clone, Copy)]
pub struct StateChange {
	pub state: u16,
	pub num_anim_dispatches: u16,
	/// Id? into LevelData.anim_dispatches
	pub anim_dispatch_id: u16,
}

#[derive(Readable, Clone, Copy)]
pub struct AnimDispatch {
	pub low_frame: u16,
	pub high_frame: u16,
	/// Id? into LevelData.animations
	pub next_anim_id: u16,
	/// Id? into LevelData.frames
	pub next_frame_id: u16,
}

#[derive(Readable)]
pub struct MeshNodeData(#[list_u32] pub Box<[u32]>);

bitfield! {
	#[derive(Readable, Clone, Copy)]
	pub struct MeshNodeDetails(u32);
	pub pop, _: 0;
	pub push, _: 1;
}

#[derive(Clone, Copy)]
pub struct MeshNode {
	pub details: MeshNodeDetails,
	/// Relative to parent
	pub offset: IVec3,
}

impl MeshNodeData {
	/// Should be called with Model.num_meshes - 1
	pub fn get_mesh_nodes(&self, mesh_node_offset: u32, num_meshes: u16) -> Vec<MeshNode> {
		let mesh_node_offset = mesh_node_offset as usize;
		(0..num_meshes as usize).map(|mesh_num| MeshNode {
			details: MeshNodeDetails(self.0[mesh_node_offset + mesh_num * 4]),
			offset: ivec3(
				self.0[mesh_node_offset + mesh_num * 4 + 1] as i32,
				self.0[mesh_node_offset + mesh_num * 4 + 2] as i32,
				self.0[mesh_node_offset + mesh_num * 4 + 3] as i32,
			),
		}).collect()
	}
}

#[derive(Readable)]
pub struct FrameData(#[list_u32] pub Box<[u16]>);

#[derive(Debug, Clone, Copy)]
pub enum FrameRotation {
	X(u16),
	Y(u16),
	Z(u16),
	All(U16Vec3),
}

pub struct Frame {
	pub bound_box: MinMax<I16Vec3>,
	pub offset: I16Vec3,
	pub rotations: Vec<FrameRotation>,
}

impl FrameData {
	pub fn get_frame(&self, frame_byte_offset: u32, num_meshes: u16) -> Frame {
		let frame_offset = frame_byte_offset as usize / 2;
		let bound_box = MinMax {
			min: i16vec3(
				self.0[frame_offset] as i16,
				self.0[frame_offset + 1] as i16,
				self.0[frame_offset + 2] as i16,
			),
			max: i16vec3(
				self.0[frame_offset + 3] as i16,
				self.0[frame_offset + 4] as i16,
				self.0[frame_offset + 5] as i16,
			),
		};
		let offset = i16vec3(
			self.0[frame_offset + 6] as i16,
			self.0[frame_offset + 7] as i16,
			self.0[frame_offset + 8] as i16,
		);
		let mut rotations = Vec::with_capacity(num_meshes as usize);
		let mut frame_offset = frame_offset + 9;
		for _ in 0..num_meshes {
			let word = self.0[frame_offset];
			let rot = match word >> 14 {
				0 => {
					let word2 = self.0[frame_offset + 1];
					frame_offset += 2;
					FrameRotation::All(U16Vec3 {
						x: (word >> 4) & 1023,
						y: ((word & 15) << 6) | (word2 >> 10),
						z: word2 & 1023,
					})
				},
				axis => {
					frame_offset += 1;
					let rot = word & 4095;
					match axis {
						1 => FrameRotation::X(rot),
						2 => FrameRotation::Y(rot),
						3 => FrameRotation::Z(rot),
						_ => unreachable!(),
					}
				},
			};
			rotations.push(rot);
		}
		Frame { bound_box, offset, rotations }
	}
}

#[derive(Readable, Clone, Copy)]
pub struct Model {
	pub id: u32,
	pub num_meshes: u16,
	/// Id into LevelData.meshes
	pub mesh_id: u16,
	/// Offset into LevelData.mesh_node_data
	pub mesh_node_offset: u32,
	/// Byte offset into LevelData.frames
	pub frame_byte_offset: u32,
	/// Id? into LevelData.animations
	pub anim_id: Option<NonMaxU16>,
}

#[derive(Readable, Clone, Copy)]
pub struct BoundBox {
	pub x: MinMax<i16>,
	pub y: MinMax<i16>,
	pub z: MinMax<i16>,
}

#[derive(Readable, Clone, Copy)]
pub struct StaticMesh {
	pub id: u32,
	/// Id into LevelData.meshes
	pub mesh_id: u16,
	pub visibility: BoundBox,
	pub collision: BoundBox,
	/// Unused, necessary to read when parsing
	pub flags: u16,
}

#[derive(Readable, Clone, Copy)]
pub struct SpriteTexture {
	pub atlas: u16,
	#[skip_2]
	pub width: u16,
	pub height: u16,
	pub left: i16,
	pub top: i16,
	pub right: i16,
	pub bottom: i16,
}

#[derive(Readable, Clone, Copy)]
pub struct SpriteSequence {
	pub sprite_id: u32,
	pub neg_length: i16,
	pub offset: u16,
}

#[derive(Readable, Clone, Copy)]
pub struct Camera {
	/// World coords
	pub pos: IVec3,
	/// Index into LevelData.rooms
	pub room_index: u16,
	pub flags: u16,
}

#[derive(Readable, Clone, Copy)]
pub struct FlybyCamera {
	/// World coords
	pub pos: IVec3,
	pub direction: IVec3,
	pub chain: u8,
	pub index: u8,
	pub fov: u16,
	pub roll: i16,
	pub timer: u16,
	pub speed: u16,
	pub flags: u16,
	/// Index into LevelData.rooms
	pub room_index: u32,
}

#[derive(Readable, Clone, Copy)]
pub struct SoundSource {
	/// World coords
	pub pos: IVec3,
	pub sound_id: u16,
	pub flags: u16,
}

#[derive(Readable, Clone, Copy)]
pub struct TrBox {
	/// Sectors
	pub z_min: u8,
	pub z_max: u8,
	pub x_min: u8,
	pub x_max: u8,
	pub y: i16,
	pub overlap: u16,
}

pub struct BoxData {
	pub boxes: Box<[TrBox]>,
	pub overlaps: Box<[u16]>,
	pub zones: Box<[u16]>,
}

impl Readable for BoxData {
	fn read<R: Read>(reader: &mut R) -> Result<Self> {
		let num_boxes = reader.read_u32::<LE>()? as usize;
		let boxes = read_boxed_slice(reader, num_boxes)?;
		let num_overlaps = reader.read_u32::<LE>()? as usize;
		let overlaps = read_boxed_slice(reader, num_overlaps)?;
		let zones = read_boxed_slice(reader, num_boxes * 10)?;
		Ok(BoxData { boxes, overlaps, zones })
	}
}

#[derive(Clone, Copy)]
pub enum BlendMode {
	Opaque,
	Test,
	Add,
}

impl Readable for BlendMode {
	fn read<R: Read>(reader: &mut R) -> Result<Self> {
		Ok(match reader.read_u16::<LE>()? {
			0 => BlendMode::Opaque,
			1 => BlendMode::Test,
			2 => BlendMode::Add,
			m => panic!("unknown blend mode: {}", m),
		})
	}
}

bitfield! {
	#[derive(Readable, Clone, Copy)]
	pub struct ObjectTextureAtlasAndTriangle(u16);
	/// Index into Images.images (16 or 32)
	pub atlas_index, _: 14, 0;
	pub triangle, _: 15;
}

bitfield! {
	#[derive(Readable, Clone, Copy)]
	pub struct ObjectTextureDetails(u16);
	pub mapping_correction, _: 2, 0;
	pub bump_mapping, _: 10, 9;
	/// True if room texture, false if object texture
	pub room_texture, _: 15;
}

#[derive(Readable, Clone, Copy)]
pub struct ObjectTexture {
	pub blend_mode: BlendMode,
	pub atlas_and_triangle: ObjectTextureAtlasAndTriangle,
	pub details: ObjectTextureDetails,
	/// Units are 1/256th of a pixel
	pub vertices: [U16Vec2; 4],
	#[skip_8]
	pub width: u32,
	pub height: u32,
}

#[derive(Readable, Clone, Copy)]
pub struct Entity {
	/// Id into Level.models
	pub model_id: u16,
	/// Index into LevelData.rooms
	pub room_index: u16,
	/// World coords
	pub pos: IVec3,
	/// Units are 1/65536th of a rotation
	pub rotation: u16,
	/// 65535 = use mesh light
	pub light_intensity: u16,
	pub ocb: u16,
	pub flags: u16,
}

#[derive(Readable, Clone, Copy)]
pub struct Ai {
	/// Id into Level.models
	pub model_id: u16,
	/// Index into LevelData.rooms
	pub room_index: u16,
	/// World coords
	pub pos: IVec3,
	pub ocb: u16,
	pub flags: u16,
	pub angle: i32,
}

#[derive(Readable, Clone, Copy)]
pub struct SoundDetail {
	#[skip_2]
	pub volume: u8,
	/// Sectors
	pub range: u8,
	pub chance: u8,
	pub pitch: u8,
	pub flags: u16,
}

#[derive(Readable)]
pub struct LevelData {
	#[skip_4]
	#[list_u16]
	pub rooms: Box<[Room]>,
	#[list_u32]
	pub floor_data: Box<[u16]>,
	pub meshes: Meshes,
	#[list_u32]
	pub animations: Box<[Anim]>,
	#[list_u32]
	pub state_changes: Box<[StateChange]>,
	#[list_u32]
	pub anim_dispatches: Box<[AnimDispatch]>,
	#[list_u32]
	pub anim_commands: Box<[u16]>,
	pub mesh_node_data: MeshNodeData,
	pub frame_data: FrameData,
	#[list_u32]
	pub models: Box<[Model]>,
	#[list_u32]
	pub static_meshes: Box<[StaticMesh]>,
	pub spr: [u8; 3],
	#[list_u32]
	pub sprite_textures: Box<[SpriteTexture]>,
	#[list_u32]
	pub sprite_sequences: Box<[SpriteSequence]>,
	#[list_u32]
	pub cameras: Box<[Camera]>,
	#[list_u32]
	pub flyby_cameras: Box<[FlybyCamera]>,
	#[list_u32]
	pub sound_sources: Box<[SoundSource]>,
	pub box_data: BoxData,
	#[list_u32]
	pub animated_textures: Box<[u16]>,
	pub animated_textures_uv_count: u8,
	pub tex: [u8; 3],
	#[list_u32]
	pub object_textures: Box<[ObjectTexture]>,
	#[list_u32]
	pub entities: Box<[Entity]>,
	#[list_u32]
	pub ais: Box<[Ai]>,
	#[list_u16]
	pub demo_data: Box<[u8]>,
	pub sound_map: Box<[u16; 370]>,
	#[list_u32]
	pub sound_details: Box<[SoundDetail]>,
	#[list_u32]
	pub sample_indices: Box<[u32]>,
	pub zero: [u8; 6],
}

#[derive(Readable)]
pub struct Sample {
	pub uncompressed: u32,
	#[list_u32]
	pub data: Box<[u8]>,
}

#[derive(Readable)]
pub struct Level {
	pub version: u32,
	pub images: Images,
	#[zlib]
	pub level_data: LevelData,
	#[list_u32]
	pub samples: Box<[Sample]>,
}

pub fn read_level<R: Read>(reader: &mut R) -> Result<Level> {
	Level::read(reader)
}
