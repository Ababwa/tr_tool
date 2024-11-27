use std::{io::{Read, Result}, ptr::addr_of_mut};
use tr_readable::{read_flat, read_val_flat, Readable};
use crate::{tr1::ATLAS_PIXELS, tr2::Color16Bit};

pub struct Atlases {
	pub num_room_atlases: u16,
	pub num_obj_atlases: u16,
	pub num_bump_atlases: u16,
	pub atlases_32bit: Box<[[u32; ATLAS_PIXELS]]>,
	pub atlases_16bit: Box<[[Color16Bit; ATLAS_PIXELS]]>,
	pub misc_images: Box<[[u32; ATLAS_PIXELS]; 2]>,
}

impl Readable for Atlases {
	unsafe fn read<R: Read>(reader: &mut R, this: *mut Self) -> Result<()> {
		read_flat(reader, addr_of_mut!((*this).num_room_atlases))?;
		read_flat(reader, addr_of_mut!((*this).num_obj_atlases))?;
		read_flat(reader, addr_of_mut!((*this).num_bump_atlases))?;
		let num_atlases = (*this).num_room_atlases + (*this).num_obj_atlases + (*this).num_bump_atlases;
		let num_atlases = num_atlases as usize;
		let images32 = unsafe { read_boxed_slice_raw(&mut get_zlib(reader)?, num_images)? };//arrays of primitives
		let images16 = unsafe { read_boxed_slice_raw(&mut get_zlib(reader)?, num_images)? };
		let misc_images = unsafe { read_boxed_slice_raw(&mut get_zlib(reader)?, 2)?.try_into().ok().unwrap() };//exactly 2
		Ok(Self {
			num_room_atlases: num_room_images,
			num_obj_atlases: num_obj_images,
			num_bump_atlases: num_bump_maps,
			atlases_32bit: images32,
			atlases_16bit: images16,
			misc_images,
		})
	}
}

#[derive(Readable, Clone, Debug)]
pub struct Level {
	#[flat] pub version: u32,
	
}

// #[derive(Readable, Clone, Debug)]
// pub struct LevelData {
// 	#[skip(4)]
// 	#[list(u16)]
// 	pub rooms: Box<[Room<Tr4>]>,
// 	#[list(u32)]
// 	pub floor_data: Box<[u16]>,
// 	pub meshes: Meshes<Tr4>,
// 	#[list(u32)]
// 	pub animations: Box<[Animation<Tr4>]>,
// 	#[list(u32)]
// 	pub state_changes: Box<[StateChange]>,
// 	#[list(u32)]
// 	pub anim_dispatches: Box<[AnimDispatch]>,
// 	#[list(u32)]
// 	pub anim_commands: Box<[u16]>,
// 	pub mesh_node_data: MeshNodeData,
// 	pub frame_data: FrameData,
// 	#[list(u32)]
// 	pub models: Box<[Model]>,
// 	#[list(u32)]
// 	pub static_meshes: Box<[StaticMesh]>,
// 	pub spr: [u8; 3],
// 	#[list(u32)]
// 	pub sprite_textures: Box<[SpriteTexture]>,
// 	#[list(u32)]
// 	pub sprite_sequences: Box<[SpriteSequence]>,
// 	#[list(u32)]
// 	pub cameras: Box<[Camera]>,
// 	#[list(u32)]
// 	pub flyby_cameras: Box<[FlybyCamera]>,
// 	#[list(u32)]
// 	pub sound_sources: Box<[SoundSource]>,
// 	pub box_data: BoxDataTr234,
// 	#[list(u32)]
// 	pub animated_textures: Box<[u16]>,
// 	pub animated_textures_uv_count: u8,
// 	pub tex: [u8; 3],
// 	#[list(u32)]
// 	pub object_textures: Box<[ObjectTexture<Tr4>]>,
// 	#[list(u32)]
// 	pub entities: Box<[Entity<Tr4>]>,
// 	#[list(u32)]
// 	pub ais: Box<[Ai]>,
// 	#[list(u16)]
// 	pub demo_data: Box<[u8]>,
// 	pub sound_map: Box<[u16; SOUND_MAP_SIZE_TR234]>,
// 	#[list(u32)]
// 	pub sound_details: Box<[SoundDetails<Tr4>]>,
// 	#[list(u32)]
// 	pub sample_indices: Box<[u32]>,
// 	pub zero: [u8; 6],
// }

// #[derive(Readable, Clone, Debug)]
// pub struct Level {
// 	pub version: u32,
// 	pub images: Images,
// 	#[zlib] pub level_data: LevelData,
// 	#[list(u32)] pub samples: Box<[Sample]>,
// }

// bitfield! {
// 	#[derive(Readable, Clone, Copy)]
// 	pub struct RoomColor(u32);
// 	pub blue, _: 7, 0;
// 	pub green, _: 15, 8;
// 	pub red, _: 23, 16;
// 	pub alpha, _: 31, 24;
// }

// #[derive(Readable, Clone, Copy)]
// pub struct RoomLight {
// 	/// World coords
// 	pub pos: IVec3,
// 	pub color: Color3,
// 	pub light_type: u8,
// 	#[skip(1)]
// 	pub intensity: u8,
// 	pub hotspot: f32,
// 	pub falloff: f32,
// 	pub length: f32,
// 	pub cutoff: f32,
// 	pub direction: Vec3,
// }

// #[derive(Readable, Clone, Copy)]
// pub struct RoomExtra {
// 	pub water_effect: u8,
// 	pub reverb: u8,
// 	pub flip_group: u8,
// }

// #[derive(Readable, Clone, Copy)]
// pub struct AnimationComponent {
// 	/// Fixed-point
// 	pub lateral_speed: u32,
// 	/// Fixed-point
// 	pub lateral_accel: u32,
// }

// #[derive(Readable, Clone, Copy)]
// pub struct FlybyCamera {
// 	/// World coords
// 	pub pos: IVec3,
// 	pub direction: IVec3,
// 	pub chain: u8,
// 	pub index: u8,
// 	pub fov: u16,
// 	pub roll: i16,
// 	pub timer: u16,
// 	pub speed: u16,
// 	pub flags: u16,
// 	/// Index into LevelData.rooms
// 	pub room_index: u32,
// }

// bitfield! {
// 	#[derive(Readable, Clone, Copy)]
// 	pub struct ObjectTextureDetails(u16);
// 	pub mapping_correction, _: 2, 0;
// 	pub bump_mapping, _: 10, 9;
// 	/// True if room texture, false if object texture
// 	pub room_texture, _: 15;
// }

// #[derive(Readable, Clone, Copy)]
// pub struct ObjectTextureComponent {
// 	#[skip(8)]
// 	pub width: u32,
// 	pub height: u32,
// }

// #[derive(Readable, Clone, Copy)]
// pub struct Ai {
// 	/// Id into Level.models
// 	pub model_id: u16,
// 	/// Index into LevelData.rooms
// 	pub room_index: u16,
// 	/// World coords
// 	pub pos: IVec3,
// 	pub ocb: u16,
// 	pub flags: u16,
// 	pub angle: i32,
// }

// pub struct Tr4;

// impl TrVersion for Tr4 {
// 	type AnimationComponent = AnimationComponent;
// 	type EntityComponent = EntityComponentOcb;
// 	type MeshComponent = MeshComponentTr45;
// 	type ObjectTextureComponent = ObjectTextureComponent;
// 	type ObjectTextureDetails = ObjectTextureDetails;
// 	type RoomAmbientLight = RoomColor;
// 	type RoomExtra = RoomExtra;
// 	type RoomLight = RoomLight;
// 	type RoomVertexLight = RoomVertexLightTr34;
// 	type SoundDetailsComponent = SoundDetailsComponentTr345;
// }


// #[derive(Readable)]
// pub struct Sample {
// 	pub uncompressed: u32,
// 	#[list(u32)]
// 	pub data: Box<[u8]>,
// }

// pub fn read_level<R: Read>(reader: &mut R) -> Result<Level> {
// 	Level::read(reader)
// }
