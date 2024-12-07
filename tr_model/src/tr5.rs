use std::io::{Error, Read, Result, Seek, SeekFrom};
use bitfield::bitfield;
use glam::{IVec3, U16Vec2, UVec2, Vec3};
use shared::min_max::MinMax;
use tr_readable::{read_slice_get, Readable, ToLen};
use crate::{
	tr1::{
		AnimDispatch, Camera, MeshNode, NumSectors, Portal, RoomFlags, Sector, SoundSource, SpriteSequence,
		SpriteTexture, StateChange, StaticMesh, ATLAS_PIXELS,
	},
	tr2::{Color16BitArgb, TrBox},
	tr3::{RoomStaticMesh, SoundDetails},
	tr4::{
		Ai, Animation, AtlasIndexFaceType, Color32BitBgra, Entity, FaceEffects, FlybyCamera, Frame, Mesh,
		NumAtlases, Sample,
	},
};

pub const SOUND_MAP_LEN: usize = 450;

//model

#[repr(C)]
#[derive(Clone, Debug)]
pub struct RoomVertex {
	pub pos: Vec3,
	pub normal: Vec3,
	pub color: u32,//format unknown
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct NumVertexBytes(pub u32);

impl ToLen for NumVertexBytes {
	fn get_len(&self) -> Result<usize> {
		if self.0 as usize % size_of::<RoomVertex>() == 0 {
			Ok(self.0 as usize / size_of::<RoomVertex>())
		} else {
			Err(Error::other("tr5 room num vertex bytes not multiple of room vertex size"))
		}
	}
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct Light {
	pub pos: Vec3,
	pub color: Vec3,
	pub unused1: u32,
	pub in_val: f32,
	pub out_val: f32,
	pub radius_in: f32,
	pub radius_out: f32,
	pub range: f32,
	pub direction: Vec3,
	pub pos2: IVec3,
	pub direction2: IVec3,
	pub light_type: u8,
	pub unused2: [u8; 3],
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct FogBulb {
	pub pos: Vec3,
	pub color: Vec3,
	pub unused: u32,
	pub in_val: f32,
	pub out_val: f32,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct Layer {
	pub num_vertices: u16,
	pub unused1: [u16; 2],
	pub num_quads: u16,
	pub num_tris: u16,
	pub unused2: [u16; 3],
	pub bound_box: MinMax<Vec3>,
	pub unused3: [u32; 4],
}

bitfield! {
	#[repr(C)]
	#[derive(Clone, Debug)]
	pub struct EffectsFaceTexture(u16);
	pub double_sided, _: 15;
	pub object_texture_index, _: 13, 0;//unknown flag at bit 14
}

macro_rules! decl_face_type {
	($name:ident, $num_indices:literal) => {
		#[repr(C)]
		#[derive(Clone, Debug)]
		pub struct $name {
			pub vertex_indices: [u16; $num_indices],
			pub texture: EffectsFaceTexture,
			pub flags: FaceEffects,
		}
	};
}

decl_face_type!(EffectsQuad, 4);
decl_face_type!(EffectsTri, 3);

#[derive(Clone, Debug)]
pub struct LayerFaces {
	pub quads: Box<[EffectsQuad]>,
	pub tris: Box<[EffectsTri]>,
}

unsafe fn read_faces<R: Read + Seek>(
	reader: &mut R, layer_faces_ptr: *mut Box<[LayerFaces]>, layers: &[Layer], size: &u32, pos: u64,
) -> Result<()> {
	let mut layer_faces = Box::new_uninit_slice(layers.len());
	for (index, layer) in layers.iter().enumerate() {
		let quads = read_slice_get(reader, layer.num_quads as usize)?;
		let tris = read_slice_get(reader, layer.num_tris as usize)?;
		layer_faces[index].write(LayerFaces { quads, tris });
	}
	layer_faces_ptr.write(layer_faces.assume_init());
	reader.seek(SeekFrom::Start(pos + *size as u64))?;
	Ok(())
}

#[derive(Readable, Clone, Debug)]
pub struct Room {
	pub xela: [u8; 4],
	pub size: u32,
	#[save_pos(data_start)] pub unused1: [u32; 2],
	pub sectors_offset: u32,
	pub unused2: u32,
	pub room_static_meshes_offset: u32,
	pub pos1: IVec3,
	pub y_bottom1: i32,
	pub y_top1: i32,
	pub num_sectors: NumSectors,
	pub color: Color32BitBgra,
	pub num_lights: u16,
	pub num_room_static_meshes: u16,
	pub reverb: u8,
	pub flip_group: u8,
	pub water_details: u16,
	pub unused3: [u32; 5],
	pub flip_room_index: u16,
	pub flags: RoomFlags,
	pub unused4: [u32; 5],
	pub pos2: Vec3,
	pub unused5: [u32; 6],
	pub num_tris: u32,
	pub num_quads: u32,
	pub unused6: [u32; 3],
	pub num_fog_bulbs: u32,
	pub y_top2: f32,
	pub y_bottom2: f32,
	pub num_layers: u32,
	pub layers_offset: u32,
	pub vertices_offset: u32,
	pub faces_offset: u32,
	pub unused7: u32,
	pub num_vertex_bytes: NumVertexBytes,
	pub unused8: [u32; 4],
	#[save_pos(data_start2)] #[list(num_lights)] pub lights: Box<[Light]>,
	#[list(num_fog_bulbs)] pub fog_bulbs: Box<[FogBulb]>,
	#[seek(data_start2, sectors_offset)] #[list(num_sectors)] pub sectors: Box<[Sector]>,
	#[list(u16)] pub portals: Box<[Portal]>,
	#[seek(data_start2, room_static_meshes_offset)] #[list(num_room_static_meshes)] pub room_static_meshes: Box<[RoomStaticMesh]>,
	#[seek(data_start2, layers_offset)] #[list(num_layers)] pub layers: Box<[Layer]>,
	#[seek(data_start2, vertices_offset)] #[list(num_vertex_bytes)] pub vertices: Box<[RoomVertex]>,
	#[seek(data_start2, faces_offset)] #[delegate(read_faces, layers, size, data_start)] pub layer_faces: Box<[LayerFaces]>,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct Model {
	pub id: u32,
	pub num_meshes: u16,
	/// Index into `Level.mesh_offsets`.
	pub mesh_offset_index: u16,
	/// Offset into `Level.mesh_node_data`.
	pub mesh_node_offset: u32,
	/// Byte offset into `Level.frame_data`.
	pub frame_byte_offset: u32,
	/// Index into `Level.animations`.
	pub anim_index: u16,
	pub unused: u16,
}

#[repr(C, packed(2))]
#[derive(Clone, Debug)]
pub struct ObjectTexture {
	/// One of the blend modes in the `blend_mode` module.
	pub blend_mode: u16,
	/// Index into `Level.atlases`.
	pub atlas_index_face_type: AtlasIndexFaceType,
	pub flags: u16,
	/// Units are 1/256 of a pixel.
	pub uvs: [U16Vec2; 4],
	pub unused1: [u16; 4],
	pub size: UVec2,
	pub unused2: u16,
}

#[derive(Readable, Clone, Debug)]
pub struct Level {
	pub version: u32,
	pub num_atlases: NumAtlases,
	#[zlib] #[list(num_atlases)] pub atlases_32bit: Box<[[Color32BitBgra; ATLAS_PIXELS]]>,
	#[zlib] #[list(num_atlases)] pub atlases_16bit: Box<[[Color16BitArgb; ATLAS_PIXELS]]>,
	#[zlib] #[boxed] pub misc_images: Box<[[Color32BitBgra; ATLAS_PIXELS]; 3]>,
	pub lara_type: u16,
	pub weather_type: u16,
	pub padding1: [u8; 28],
	pub level_data_uncompressed_size: u32,
	pub level_data_compressed_size: u32,
	pub unused: u32,
	#[list(u32)] #[delegate] pub rooms: Box<[Room]>,
	#[list(u32)] pub floor_data: Box<[u16]>,
	#[list(u32)] pub mesh_data: Box<[u16]>,
	#[list(u32)] pub mesh_offsets: Box<[u32]>,
	#[list(u32)] pub animations: Box<[Animation]>,
	#[list(u32)] pub state_changes: Box<[StateChange]>,
	#[list(u32)] pub anim_dispatches: Box<[AnimDispatch]>,
	#[list(u32)] pub anim_commands: Box<[u16]>,
	#[list(u32)] pub mesh_node_data: Box<[u32]>,
	#[list(u32)] pub frame_data: Box<[u16]>,
	#[list(u32)] pub models: Box<[Model]>,
	#[list(u32)] pub static_meshes: Box<[StaticMesh]>,
	pub spr0: [u8; 4],
	#[list(u32)] pub sprite_textures: Box<[SpriteTexture]>,
	#[list(u32)] pub sprite_sequences: Box<[SpriteSequence]>,
	#[list(u32)] pub cameras: Box<[Camera]>,
	#[list(u32)] pub flyby_cameras: Box<[FlybyCamera]>,
	#[list(u32)] pub sound_sources: Box<[SoundSource]>,
	#[list(u32)] pub boxes: Box<[TrBox]>,
	#[list(u32)] pub overlap_data: Box<[u16]>,
	#[list(boxes)] pub zone_data: Box<[[u16; 10]]>,
	#[list(u32)] pub animated_textures: Box<[u16]>,
	pub animated_textures_uv_count: u8,
	pub tex0: [u8; 4],
	#[list(u32)] pub object_textures: Box<[ObjectTexture]>,
	#[list(u32)] pub entities: Box<[Entity]>,
	#[list(u32)] pub ais: Box<[Ai]>,
	#[list(u16)] pub demo_data: Box<[u8]>,
	#[boxed] pub sound_map: Box<[u16; SOUND_MAP_LEN]>,
	#[list(u32)] pub sound_details: Box<[SoundDetails]>,
	#[list(u32)] pub sample_indices: Box<[u32]>,
	pub padding2: [u8; 6],
	#[list(u32)] #[delegate] pub samples: Box<[Sample]>,
}

impl Level {
	pub fn get_mesh(&self, mesh_offset: u32) -> Mesh {
		Mesh::get(&self.mesh_data, mesh_offset)
	}
	
	pub fn get_mesh_nodes(&self, model: &Model) -> &[MeshNode] {
		MeshNode::get(&self.mesh_node_data, model.mesh_node_offset, model.num_meshes)
	}
	
	pub fn get_frame(&self, model: &Model) -> Frame {
		Frame::get(&self.frame_data, model.frame_byte_offset, model.num_meshes)
	}
}
