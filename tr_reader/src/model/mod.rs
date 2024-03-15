pub mod tr2;
pub mod tr3;
pub mod tr4;

use std::{collections::HashMap, io::{Cursor, Read, Result}};
use bitfield::bitfield;
use byteorder::{ReadBytesExt, LE};
use glam::{i16vec3, I16Vec2, I16Vec3, IVec3, U16Vec2, U16Vec3};
use glam_traits::ext::U8Vec2;
use nonmax::{NonMaxU16, NonMaxU8};
use shared::{geom::MinMax, reinterpret};
use crate::{read_boxed_slice, read_boxed_slice_raw, read_list, Readable};

// 1 sector unit = 1024 world coord units

pub const PALETTE_SIZE: usize = 256;
pub const IMAGE_SIZE: usize = 256;
pub const NUM_PIXELS: usize = IMAGE_SIZE * IMAGE_SIZE;
pub const LIGHT_MAP_SIZE: usize = 32;
pub const SOUND_MAP_SIZE: usize = 370;

const FRAME_SINGLE_ROT_MASK_TR123: u16 = 1023;
const FRAME_SINGLE_ROT_MASK_TR45: u16 = 4095;

pub trait TrVersion {
	const FRAME_SINGLE_ROT_MASK: u16;
}

#[repr(C)]
#[derive(Readable, Clone, Copy)]
pub struct Color3 {
	pub r: u8,
	pub g: u8,
	pub b: u8,
}

#[repr(C)]
#[derive(Readable, Clone, Copy)]
pub struct Color4 {
	pub color: Color3,
	pub unused: u8,
}

pub struct Images {
	pub pallete_images: Box<[[u8; NUM_PIXELS]]>,
	pub images16: Box<[[u16; NUM_PIXELS]]>,
}

impl Readable for Images {
	fn read<R: Read>(reader: &mut R) -> Result<Self> {
		let num_images = reader.read_u32::<LE>()? as usize;
		let pallete_images = unsafe { read_boxed_slice_raw(reader, num_images)? };//safe: arrays of primitives
		let images16 = unsafe { read_boxed_slice_raw(reader, num_images)? };
		Ok(Images { pallete_images, images16 })
	}
}

#[derive(Readable, Clone, Copy)]
pub struct RoomVertexComponentTr2 {
	#[skip(2)]
	pub flags: u16,
	pub brightness: u16,
}

#[derive(Readable, Clone, Copy)]
pub struct RoomVertexComponentTr34 {
	#[skip(2)]
	pub flags: u16,
	pub color: u16,
}

#[derive(Readable, Clone, Copy)]
#[impl_where(Component: Readable)]
pub struct RoomVertex<Component> {
	/// Relative to Room
	pub vertex: I16Vec3,
	pub component: Component,
}

bitfield! {
	#[derive(Readable, Clone, Copy)]
	pub struct TexturedFaceDetails(u16);
	/// Index into object_textures
	pub texture_index, _: 14, 0;
	pub double_sided, _: 15;
}

#[derive(Readable, Clone, Copy)]
pub struct SolidFaceDetails {
	/// Index into palette3
	pub palette3_index: u8,
	/// Index into palette4
	pub palette4_index: u8,
}

#[derive(Readable, Clone, Copy)]
#[impl_where(D: Readable)]
pub struct Face<const N: usize, D> {
	pub vertex_indices: [u16; N],
	pub texture_details: D,
}

#[derive(Readable)]
#[impl_where(
	VertexComponent: Readable,
	AmbientLight: Readable,
	Light: Readable,
	Extra: Readable,
)]
pub struct Room<VertexComponent, AmbientLight, Light, Extra> {
	/// World coord
	pub x: i32,
	/// World coord
	pub z: i32,
	pub y_bottom: i32,
	pub y_top: i32,
	#[skip(4)]
	#[list(u16)]
	pub vertices: Box<[RoomVertex<VertexComponent>]>,
	/// `vertex_indices` index into Room.vertices
	#[list(u16)]
	pub quads: Box<[Face<4, TexturedFaceDetails>]>,
	/// `vertex_indices` index into Room.vertices
	#[list(u16)]
	pub tris: Box<[Face<3, TexturedFaceDetails>]>,
	#[list(u16)]
	pub sprites: Box<[Sprite]>,
	#[list(u16)]
	pub portals: Box<[Portal]>,
	pub sectors: Sectors,
	pub ambient_light: AmbientLight,
	#[list(u16)]
	pub lights: Box<[Light]>,
	#[list(u16)]
	pub room_static_meshes: Box<[RoomStaticMesh]>,
	/// Index into LevelData.rooms
	pub flip_room_index: Option<NonMaxU16>,
	pub flags: RoomFlags,
	pub extra: Extra,
}

pub enum MeshLighting {
	Normals(Box<[I16Vec3]>),
	Lights(Box<[u16]>),
}

impl Readable for MeshLighting {
	fn read<R: Read>(reader: &mut R) -> Result<Self> {
		Ok(match reader.read_i16::<LE>()? {
			num if num > 0 => MeshLighting::Normals(read_boxed_slice(reader, num as usize)?),
			num => MeshLighting::Lights(read_boxed_slice(reader, (-num) as usize)?),
		})
	}
}

#[derive(Readable)]
pub struct MeshComponentTr123 {
	#[list(u16)]
	pub textured_quads: Box<[Face<4, TexturedFaceDetails>]>,
	#[list(u16)]
	pub textured_tris: Box<[Face<3, TexturedFaceDetails>]>,
	#[list(u16)]
	pub solid_quads: Box<[Face<4, SolidFaceDetails>]>,
	#[list(u16)]
	pub solid_tris: Box<[Face<3, SolidFaceDetails>]>,
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
	pub face: Face<N, TexturedFaceDetails>,
	pub effects: MeshEffects,
}

#[derive(Readable)]
pub struct MeshComponentTr45 {
	#[list(u16)]
	pub quads: Box<[MeshFace<4>]>,
	#[list(u16)]
	pub tris: Box<[MeshFace<3>]>,
}

#[derive(Readable)]
#[impl_where(C: Readable)]
pub struct Mesh<C> {
	pub center: I16Vec3,
	pub radius: i32,
	/// Relative to RoomStaticMesh.pos if static mesh
	#[list(u16)]
	pub vertices: Box<[I16Vec3]>,
	pub lighting: MeshLighting,
	pub component: C,
}

#[derive(Readable, Clone, Copy)]
#[impl_where(Lateral: Readable)]
pub struct Animation<Lateral> {
	/// Byte offset into frame_data
	pub frame_byte_offset: u32,
	/// 30ths of a second
	pub frame_duration: u8,
	pub num_frames: u8,
	pub state: u16,
	/// Fixed-point
	pub speed: u32,
	/// Fixed-point
	pub accel: u32,
	pub lateral: Lateral,
	pub frame_start: u16,
	pub frame_end: u16,
	pub next_anim: u16,
	pub next_frame: u16,
	pub num_state_changes: u16,
	/// Id? into state_changes
	pub state_change_id: u16,
	pub num_anim_commands: u16,
	/// Id? into anim_commands
	pub anim_command_id: u16,
}

#[derive(Readable, Clone, Copy)]
pub struct Sprite {
	/// Index into Room.vertices
	pub vertex_index: u16,
	/// Index into sprite_textures
	pub texture_index: u16,
}

#[derive(Readable, Clone, Copy)]
pub struct Portal {
	/// Index into rooms
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
	/// Index into BoxData.boxes
	pub box_index, _: 14, 4;
}

#[derive(Readable, Clone, Copy)]
pub struct Sector {
	/// Index into floor_data
	pub floor_data_index: u16,
	pub material_and_box: SectorMaterialAndBox,
	/// Index into rooms
	pub room_below_id: Option<NonMaxU8>,
	pub floor: i8,
	/// Index into rooms
	pub room_above_index: Option<NonMaxU8>,
	pub ceiling: i8,
}

pub struct Sectors {
	pub num_sectors: U16Vec2,
	pub sectors: Box<[Sector]>,
}

impl Readable for Sectors {
	fn read<R: Read>(reader: &mut R) -> Result<Self> {
		let num_sectors = U16Vec2::read(reader)?;
		let sectors = read_boxed_slice(reader, num_sectors.element_product() as usize)?;
		Ok(Sectors { num_sectors, sectors })
	}
}

#[derive(Readable, Clone, Copy)]
pub struct RoomStaticMesh {
	/// World coords
	pub pos: IVec3,
	/// Units are 1/65536th of a rotation
	pub rotation: u16,
	pub color: u16,
	/// Id into LevelData.static_meshes
	#[skip(2)]
	pub static_mesh_id: u16,
}

bitfield! {
	#[derive(Readable, Clone, Copy)]
	pub struct RoomFlags(u16);
	pub water, _: 0;
}

pub struct Meshes<M> {
	pub meshes: Box<[M]>,
	pub index_map: Box<[usize]>,
}

impl<M> Meshes<M> {
	pub fn get_mesh(&self, mesh_id: u16) -> &M {
		&self.meshes[self.index_map[mesh_id as usize]]
	}
}

impl<M: Readable> Readable for Meshes<M> {
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
			.map(|(offset, _)| M::read(&mut Cursor::new(&mesh_bytes[offset as usize..])))
			.collect::<Result<Vec<_>>>()?
			.into_boxed_slice();
		Ok(Meshes { meshes, index_map })
	}
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

bitfield! {
	#[derive(Readable, Clone, Copy)]
	pub struct MeshNodeDetails(u32);
	pub pop, _: 0;
	pub push, _: 1;
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MeshNode {
	pub details: MeshNodeDetails,
	/// Relative to parent
	pub offset: IVec3,
}

#[derive(Readable)]
pub struct MeshNodeData(#[list(u32)] pub Box<[u32]>);

impl MeshNodeData {
	/// Should be called with Model.num_meshes - 1
	pub fn get_mesh_nodes(&self, mesh_node_offset: u32, num_meshes: u16) -> &[MeshNode] {
		let lo = mesh_node_offset as usize;
		let hi = lo + num_meshes as usize * 4;
		unsafe { reinterpret::slice(&self.0[lo..hi]) }//contiguous 4-aligned values
	}
}

#[derive(Readable)]
pub struct FrameData(#[list(u32)] pub Box<[u16]>);

#[derive(Clone, Copy)]
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
	pub fn get_frame<T: TrVersion>(&self, frame_byte_offset: u32, num_meshes: u16) -> Frame {
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
					let rot = word & T::FRAME_SINGLE_ROT_MASK;
					match axis {
						1 => FrameRotation::X(rot),
						2 => FrameRotation::Y(rot),
						3 => FrameRotation::Z(rot),
						_ => unreachable!(),//2 bits must be 0-3
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
	/// Id into meshes
	pub mesh_id: u16,
	/// Offset into mesh_node_data
	pub mesh_node_offset: u32,
	/// Byte offset into frames
	pub frame_byte_offset: u32,
	/// Index into animations
	pub anim_index: Option<NonMaxU16>,
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
	pub flags: u16,
}

#[derive(Readable, Clone, Copy)]
pub struct SpriteTexture {
	/// Index into images
	pub atlas_index: u16,
	pub pos: U8Vec2,
	pub size: U16Vec2,
	pub world_bounds: [I16Vec2; 2],
}

#[derive(Readable, Clone, Copy)]
pub struct SpriteSequence {
	pub id: u32,
	pub neg_length: i16,
	/// Index into sprite_textures
	pub sprite_index: u16,
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
pub struct SoundSource {
	/// World coords
	pub pos: IVec3,
	pub sound_id: u16,
	pub flags: u16,
}

#[derive(Readable, Clone, Copy)]
#[impl_where(T: Readable)]
pub struct TrBox<T> {
	/// Sectors
	pub z: MinMax<T>,
	pub x: MinMax<T>,
	pub y: i16,
	pub overlap: u16,
}

pub struct BoxData<T> {
	pub boxes: Box<[TrBox<T>]>,
	pub overlaps: Box<[u16]>,
	pub zones: Box<[u16]>,
}

impl<T: Readable> Readable for BoxData<T> {
	fn read<R: Read>(reader: &mut R) -> Result<Self> {
		let boxes = read_list::<_, _, u32>(reader)?;
		let overlaps = read_list::<_, _,u32>(reader)?;
		let zones = read_boxed_slice(reader, boxes.len() * 10)?;
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
	/// Index into images
	pub atlas_index, _: 14, 0;
	pub triangle, _: 15;
}

#[derive(Readable, Clone, Copy)]
#[impl_where(Details: Readable, Component: Readable)]
pub struct ObjectTexture<Details, Component> {
	pub blend_mode: BlendMode,
	pub atlas_and_triangle: ObjectTextureAtlasAndTriangle,
	pub details: Details,
	/// Units are 1/256th of a pixel
	pub vertices: [U16Vec2; 4],
	pub component: Component,
}

#[derive(Readable, Clone, Copy)]
#[skip_after(2)]
pub struct EntityComponentSkip;

#[derive(Readable, Clone, Copy)]
pub struct EntityComponentOcb(pub u16);

#[derive(Readable, Clone, Copy)]
#[impl_where(Component: Readable)]
pub struct Entity<Component> {
	/// Id into models or sprite_textures
	pub model_id: u16,
	/// Index into rooms
	pub room_index: u16,
	/// World coords
	pub pos: IVec3,
	/// Units are 1/65536th of a rotation
	pub rotation: u16,
	/// If None, use mesh light
	pub brightness: Option<NonMaxU16>,
	pub component: Component,
	pub flags: u16,
}

pub struct LightMap(pub Box<[[u8; PALETTE_SIZE]; LIGHT_MAP_SIZE]>);

impl Readable for LightMap {
	fn read<R: Read>(reader: &mut R) -> Result<Self> {
		unsafe {
			Ok(Self(read_boxed_slice_raw(reader, LIGHT_MAP_SIZE)?.try_into().ok().unwrap()))//exactly 32
		}//array of bytes
	}
}

#[derive(Readable, Clone, Copy)]
pub struct CinematicFrame {
	pub target: I16Vec3,
	pub pos: I16Vec3,
	pub fov: i16,
	pub roll: i16,
}

#[derive(Readable, Clone, Copy)]
pub struct SoundDetailsComponentTr12 {
	pub volume: u16,
	pub chance: u16,
}

#[derive(Readable, Clone, Copy)]
pub struct SoundDetailsComponentTr345 {
	pub volume: u8,
	/// Sectors
	pub range: u8,
	pub chance: u8,
	pub pitch: u8,
}

#[derive(Readable, Clone, Copy)]
#[impl_where(Component: Readable)]
pub struct SoundDetails<Component> {
	/// Index into sample_indices
	pub sample_index: u16,
	pub component: Component,
	pub flags: u16,
}
