#![feature(prelude_import)]
#[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
pub use tr_readable::Readable2;
pub mod tr1 {
    use std::{
        io::{Read, Result},
        mem::size_of, ptr::addr_of_mut,
    };
    use bitfield::bitfield;
    use glam::{I16Vec2, I16Vec3, IVec2, IVec3, U16Vec2};
    use glam_traits::ext::U8Vec2;
    use nonmax::{NonMaxU16, NonMaxU8};
    use shared::MinMax;
    use tr_readable::{read_boxed_slice_flat, read_flat, read_len, Readable2};
    pub const ATLAS_SIZE: usize = 256;
    pub const NUM_PIXELS: usize = ATLAS_SIZE * ATLAS_SIZE;
    pub const PALETTE_SIZE: usize = 256;
    pub const COLOR_SIZE: usize = 3;
    pub const SOUND_MAP_SIZE: usize = 256;
    pub const LIGHT_MAP_SIZE: usize = 32;
    pub const ZONE_MULT: usize = 6;
    pub mod blend_mode {
        pub const OPAQUE: u16 = 0;
        pub const TEST: u16 = 1;
    }
    #[repr(packed)]
    pub struct NoAlign<T: Copy>(T);
    #[automatically_derived]
    impl<T: ::core::clone::Clone + ::core::marker::Copy + Copy> ::core::clone::Clone
    for NoAlign<T> {
        #[inline]
        fn clone(&self) -> NoAlign<T> {
            NoAlign(::core::clone::Clone::clone(&{ self.0 }))
        }
    }
    #[automatically_derived]
    impl<T: ::core::marker::Copy + Copy> ::core::marker::Copy for NoAlign<T> {}
    impl<T: Copy> NoAlign<T> {
        pub fn get(&self) -> T {
            self.0
        }
        pub fn set(&mut self, val: T) {
            self.0 = val;
        }
    }
    #[repr(C)]
    pub struct RoomVertex {
        /// Relative to room
        pub pos: I16Vec3,
        pub light: u16,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for RoomVertex {
        #[inline]
        fn clone(&self) -> RoomVertex {
            let _: ::core::clone::AssertParamIsClone<I16Vec3>;
            let _: ::core::clone::AssertParamIsClone<u16>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for RoomVertex {}
    #[repr(C)]
    pub struct Face<const N: usize> {
        /// If room face, index into Room.vertices
        pub vertex_indices: [u16; N],
        /// If textured, index into Level.object_textures
        /// If solid, index into Level.palette
        pub texture_index: u16,
    }
    #[automatically_derived]
    impl<const N: usize> ::core::clone::Clone for Face<N> {
        #[inline]
        fn clone(&self) -> Face<N> {
            let _: ::core::clone::AssertParamIsClone<[u16; N]>;
            let _: ::core::clone::AssertParamIsClone<u16>;
            *self
        }
    }
    #[automatically_derived]
    impl<const N: usize> ::core::marker::Copy for Face<N> {}
    pub type Quad = Face<4>;
    pub type Tri = Face<3>;
    #[repr(C)]
    pub struct Sprite {
        /// Index into Room.vertices
        pub vertex_index: u16,
        /// Index into Level.sprite_textures
        pub texture_index: u16,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for Sprite {
        #[inline]
        fn clone(&self) -> Sprite {
            let _: ::core::clone::AssertParamIsClone<u16>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for Sprite {}
    #[repr(C)]
    pub struct Portal {
        /// Index into Level.rooms
        pub adjoining_room_index: u16,
        pub normal: I16Vec3,
        /// Relative to room
        pub vertices: [I16Vec3; 4],
    }
    #[automatically_derived]
    impl ::core::clone::Clone for Portal {
        #[inline]
        fn clone(&self) -> Portal {
            let _: ::core::clone::AssertParamIsClone<u16>;
            let _: ::core::clone::AssertParamIsClone<I16Vec3>;
            let _: ::core::clone::AssertParamIsClone<[I16Vec3; 4]>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for Portal {}
    #[repr(C)]
    pub struct Sector {
        /// Index into Level.floor_data
        pub floor_data_index: u16,
        /// Index into BoxData.boxes
        pub box_index: Option<NonMaxU16>,
        /// Index into Level.rooms
        pub room_below_id: Option<NonMaxU8>,
        pub floor: i8,
        /// Index into Level.rooms
        pub room_above_index: Option<NonMaxU8>,
        pub ceiling: i8,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for Sector {
        #[inline]
        fn clone(&self) -> Sector {
            let _: ::core::clone::AssertParamIsClone<u16>;
            let _: ::core::clone::AssertParamIsClone<Option<NonMaxU16>>;
            let _: ::core::clone::AssertParamIsClone<Option<NonMaxU8>>;
            let _: ::core::clone::AssertParamIsClone<i8>;
            let _: ::core::clone::AssertParamIsClone<Option<NonMaxU8>>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for Sector {}
    pub struct Sectors {
        pub num_sectors: U16Vec2,
        pub sectors: Box<[Sector]>,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for Sectors {
        #[inline]
        fn clone(&self) -> Sectors {
            Sectors {
                num_sectors: ::core::clone::Clone::clone(&self.num_sectors),
                sectors: ::core::clone::Clone::clone(&self.sectors),
            }
        }
    }
    impl Readable2 for Sectors {
        unsafe fn read<R: Read>(reader: &mut R, this: *mut Self) -> Result<()> {
            read_flat(reader, &raw mut (*this).num_sectors)?;
            let len = (*this).num_sectors.element_product() as usize;
            read_boxed_slice_flat(reader, &raw mut (*this).sectors, len)?;
            Ok(())
        }
    }
    #[repr(C)]
    pub struct Light {
        pub pos: NoAlign<IVec3>,
        pub brightness: u16,
        pub fallout: NoAlign<u32>,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for Light {
        #[inline]
        fn clone(&self) -> Light {
            let _: ::core::clone::AssertParamIsClone<NoAlign<IVec3>>;
            let _: ::core::clone::AssertParamIsClone<u16>;
            let _: ::core::clone::AssertParamIsClone<NoAlign<u32>>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for Light {}
    #[repr(C)]
    pub struct RoomStaticMesh {
        /// World coords
        pub pos: NoAlign<IVec3>,
        /// Units are 1/65536 of a rotation
        pub rotation: u16,
        pub light: u16,
        /// Matched to StaticMesh.id in Level.static_meshes
        pub static_mesh_id: u16,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for RoomStaticMesh {
        #[inline]
        fn clone(&self) -> RoomStaticMesh {
            let _: ::core::clone::AssertParamIsClone<NoAlign<IVec3>>;
            let _: ::core::clone::AssertParamIsClone<u16>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for RoomStaticMesh {}
    impl<T> ::bitfield::BitRange<T> for RoomFlags
    where
        u16: ::bitfield::BitRange<T>,
    {
        fn bit_range(&self, msb: usize, lsb: usize) -> T {
            self.0.bit_range(msb, lsb)
        }
    }
    impl<T> ::bitfield::BitRangeMut<T> for RoomFlags
    where
        u16: ::bitfield::BitRangeMut<T>,
    {
        fn set_bit_range(&mut self, msb: usize, lsb: usize, value: T) {
            self.0.set_bit_range(msb, lsb, value);
        }
    }
    pub struct RoomFlags(pub u16);
    #[automatically_derived]
    impl ::core::clone::Clone for RoomFlags {
        #[inline]
        fn clone(&self) -> RoomFlags {
            let _: ::core::clone::AssertParamIsClone<u16>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for RoomFlags {}
    impl RoomFlags {
        pub fn water(&self) -> bool {
            use ::bitfield::Bit;
            self.bit(0)
        }
    }
    pub struct Room {
        /// World coord
        #[flat]
        pub x: i32,
        /// World coord
        #[flat]
        pub z: i32,
        #[flat]
        pub y_bottom: i32,
        #[flat]
        pub y_top: i32,
        #[flat]
        #[list(u32)]
        pub geom_data: Box<[u16]>,
        #[flat]
        #[list(u16)]
        pub portals: Box<[Portal]>,
        #[delegate]
        pub sectors: Sectors,
        #[flat]
        pub ambient_light: u16,
        #[flat]
        #[list(u16)]
        pub lights: Box<[Light]>,
        #[flat]
        #[list(u16)]
        pub room_static_meshes: Box<[RoomStaticMesh]>,
        /// Index into Level.rooms
        #[flat]
        pub flip_room_index: Option<NonMaxU16>,
        #[flat]
        pub flags: RoomFlags,
    }
    impl tr_readable::Readable2 for Room {
        unsafe fn read<R: std::io::Read>(
            reader: &mut R,
            this: *mut Self,
        ) -> std::io::Result<()> {
            const _: () = unsafe {
                let i = std::mem::MaybeUninit::<Room>::uninit();
                if !((&raw const (*i.as_ptr()).x)
                    .add(1)
                    .byte_offset_from(&raw const (*i.as_ptr()).z) == 0)
                {
                    {
                        ::core::panicking::panic_fmt(
                            format_args!("adjacent flat fields not contiguous: #f1, #f2"),
                        );
                    }
                }
                if !((&raw const (*i.as_ptr()).z)
                    .add(1)
                    .byte_offset_from(&raw const (*i.as_ptr()).y_bottom) == 0)
                {
                    {
                        ::core::panicking::panic_fmt(
                            format_args!("adjacent flat fields not contiguous: #f1, #f2"),
                        );
                    }
                }
                if !((&raw const (*i.as_ptr()).y_bottom)
                    .add(1)
                    .byte_offset_from(&raw const (*i.as_ptr()).y_top) == 0)
                {
                    {
                        ::core::panicking::panic_fmt(
                            format_args!("adjacent flat fields not contiguous: #f1, #f2"),
                        );
                    }
                }
            };
            tr_readable::read_range_flat(
                reader,
                &raw mut (*this).x,
                (&raw mut (*this).y_top).add(1),
            )?;
            let mut len = tr_readable::read_len::<_, u32>(reader)?;
            tr_readable::read_boxed_slice_flat(reader, &raw mut (*this).geom_data, len)?;
            let mut len = tr_readable::read_len::<_, u16>(reader)?;
            tr_readable::read_boxed_slice_flat(reader, &raw mut (*this).portals, len)?;
            tr_readable::Readable2::read(reader, &raw mut (*this).sectors)?;
            tr_readable::read_flat(reader, &raw mut (*this).ambient_light)?;
            let mut len = tr_readable::read_len::<_, u16>(reader)?;
            tr_readable::read_boxed_slice_flat(reader, &raw mut (*this).lights, len)?;
            let mut len = tr_readable::read_len::<_, u16>(reader)?;
            tr_readable::read_boxed_slice_flat(
                reader,
                &raw mut (*this).room_static_meshes,
                len,
            )?;
            const _: () = unsafe {
                let i = std::mem::MaybeUninit::<Room>::uninit();
                if !((&raw const (*i.as_ptr()).flip_room_index)
                    .add(1)
                    .byte_offset_from(&raw const (*i.as_ptr()).flags) == 0)
                {
                    {
                        ::core::panicking::panic_fmt(
                            format_args!("adjacent flat fields not contiguous: #f1, #f2"),
                        );
                    }
                }
            };
            tr_readable::read_range_flat(
                reader,
                &raw mut (*this).flip_room_index,
                (&raw mut (*this).flags).add(1),
            )?;
            Ok(())
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for Room {
        #[inline]
        fn clone(&self) -> Room {
            Room {
                x: ::core::clone::Clone::clone(&self.x),
                z: ::core::clone::Clone::clone(&self.z),
                y_bottom: ::core::clone::Clone::clone(&self.y_bottom),
                y_top: ::core::clone::Clone::clone(&self.y_top),
                geom_data: ::core::clone::Clone::clone(&self.geom_data),
                portals: ::core::clone::Clone::clone(&self.portals),
                sectors: ::core::clone::Clone::clone(&self.sectors),
                ambient_light: ::core::clone::Clone::clone(&self.ambient_light),
                lights: ::core::clone::Clone::clone(&self.lights),
                room_static_meshes: ::core::clone::Clone::clone(
                    &self.room_static_meshes,
                ),
                flip_room_index: ::core::clone::Clone::clone(&self.flip_room_index),
                flags: ::core::clone::Clone::clone(&self.flags),
            }
        }
    }
    #[repr(C)]
    pub struct Animation {
        /// Byte offset into Level.frame_data
        pub frame_byte_offset: u32,
        /// 30ths of a second
        pub frame_duration: u8,
        pub num_frames: u8,
        pub state: u16,
        /// Fixed-point
        pub speed: u32,
        /// Fixed-point
        pub accel: u32,
        pub frame_start: u16,
        pub frame_end: u16,
        pub next_anim: u16,
        pub next_frame: u16,
        pub num_state_changes: u16,
        pub state_change_id: u16,
        pub num_anim_commands: u16,
        pub anim_command_id: u16,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for Animation {
        #[inline]
        fn clone(&self) -> Animation {
            let _: ::core::clone::AssertParamIsClone<u32>;
            let _: ::core::clone::AssertParamIsClone<u8>;
            let _: ::core::clone::AssertParamIsClone<u16>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for Animation {}
    #[repr(C)]
    pub struct StateChange {
        pub state: u16,
        pub num_anim_dispatches: u16,
        pub anim_dispatch_id: u16,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for StateChange {
        #[inline]
        fn clone(&self) -> StateChange {
            let _: ::core::clone::AssertParamIsClone<u16>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for StateChange {}
    #[repr(C)]
    pub struct AnimDispatch {
        pub low_frame: u16,
        pub high_frame: u16,
        pub next_anim_id: u16,
        pub next_frame_id: u16,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for AnimDispatch {
        #[inline]
        fn clone(&self) -> AnimDispatch {
            let _: ::core::clone::AssertParamIsClone<u16>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for AnimDispatch {}
    #[repr(C)]
    pub struct Model {
        pub id: NoAlign<u32>,
        pub num_meshes: u16,
        /// Index into Level.mesh_offsets
        pub mesh_offset_index: u16,
        /// Offset into Level.mesh_node_data
        pub mesh_node_offset: NoAlign<u32>,
        /// Byte offset into Level.frame_data
        pub frame_byte_offset: NoAlign<u32>,
        /// Index into Level.animations
        pub anim_index: Option<NonMaxU16>,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for Model {
        #[inline]
        fn clone(&self) -> Model {
            let _: ::core::clone::AssertParamIsClone<NoAlign<u32>>;
            let _: ::core::clone::AssertParamIsClone<u16>;
            let _: ::core::clone::AssertParamIsClone<NoAlign<u32>>;
            let _: ::core::clone::AssertParamIsClone<NoAlign<u32>>;
            let _: ::core::clone::AssertParamIsClone<Option<NonMaxU16>>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for Model {}
    #[repr(C)]
    pub struct BoundBox {
        pub x: MinMax<i16>,
        pub y: MinMax<i16>,
        pub z: MinMax<i16>,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for BoundBox {
        #[inline]
        fn clone(&self) -> BoundBox {
            let _: ::core::clone::AssertParamIsClone<MinMax<i16>>;
            let _: ::core::clone::AssertParamIsClone<MinMax<i16>>;
            let _: ::core::clone::AssertParamIsClone<MinMax<i16>>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for BoundBox {}
    #[repr(C)]
    pub struct StaticMesh {
        pub id: u32,
        /// Index into Level.mesh_offsets
        pub mesh_offset_index: u16,
        pub visibility: BoundBox,
        pub collision: BoundBox,
        pub flags: u16,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for StaticMesh {
        #[inline]
        fn clone(&self) -> StaticMesh {
            let _: ::core::clone::AssertParamIsClone<u32>;
            let _: ::core::clone::AssertParamIsClone<u16>;
            let _: ::core::clone::AssertParamIsClone<BoundBox>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for StaticMesh {}
    #[repr(C)]
    pub struct ObjectTexture {
        pub blend_mode: u16,
        /// Index into Level.atlases
        pub atlas_index: u16,
        /// Units are 1/256 of a pixel
        pub uvs: [U16Vec2; 4],
    }
    #[automatically_derived]
    impl ::core::clone::Clone for ObjectTexture {
        #[inline]
        fn clone(&self) -> ObjectTexture {
            let _: ::core::clone::AssertParamIsClone<u16>;
            let _: ::core::clone::AssertParamIsClone<[U16Vec2; 4]>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for ObjectTexture {}
    #[repr(C)]
    pub struct SpriteTexture {
        /// Index into Level.atlases
        pub atlas_index: u16,
        pub pos: U8Vec2,
        pub size: U16Vec2,
        pub world_bounds: [I16Vec2; 2],
    }
    #[automatically_derived]
    impl ::core::clone::Clone for SpriteTexture {
        #[inline]
        fn clone(&self) -> SpriteTexture {
            let _: ::core::clone::AssertParamIsClone<u16>;
            let _: ::core::clone::AssertParamIsClone<U8Vec2>;
            let _: ::core::clone::AssertParamIsClone<U16Vec2>;
            let _: ::core::clone::AssertParamIsClone<[I16Vec2; 2]>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for SpriteTexture {}
    #[repr(C)]
    pub struct SpriteSequence {
        pub id: u32,
        pub neg_length: i16,
        /// Index into Level.sprite_textures
        pub sprite_index: u16,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for SpriteSequence {
        #[inline]
        fn clone(&self) -> SpriteSequence {
            let _: ::core::clone::AssertParamIsClone<u32>;
            let _: ::core::clone::AssertParamIsClone<i16>;
            let _: ::core::clone::AssertParamIsClone<u16>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for SpriteSequence {}
    #[repr(C)]
    pub struct Camera {
        /// World coords
        pub pos: IVec3,
        /// Index into Level.rooms
        pub room_index: u16,
        pub flags: u16,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for Camera {
        #[inline]
        fn clone(&self) -> Camera {
            let _: ::core::clone::AssertParamIsClone<IVec3>;
            let _: ::core::clone::AssertParamIsClone<u16>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for Camera {}
    #[repr(C)]
    pub struct SoundSource {
        /// World coords
        pub pos: IVec3,
        pub sound_id: u16,
        pub flags: u16,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for SoundSource {
        #[inline]
        fn clone(&self) -> SoundSource {
            let _: ::core::clone::AssertParamIsClone<IVec3>;
            let _: ::core::clone::AssertParamIsClone<u16>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for SoundSource {}
    #[repr(C)]
    pub struct TrBox {
        /// Sectors
        pub z: MinMax<u32>,
        pub x: MinMax<u32>,
        pub y: i16,
        pub overlap: u16,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for TrBox {
        #[inline]
        fn clone(&self) -> TrBox {
            let _: ::core::clone::AssertParamIsClone<MinMax<u32>>;
            let _: ::core::clone::AssertParamIsClone<MinMax<u32>>;
            let _: ::core::clone::AssertParamIsClone<i16>;
            let _: ::core::clone::AssertParamIsClone<u16>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for TrBox {}
    pub struct BoxData {
        pub boxes: Box<[TrBox]>,
        pub overlap_data: Box<[u16]>,
        pub zone_data: Box<[u16]>,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for BoxData {
        #[inline]
        fn clone(&self) -> BoxData {
            BoxData {
                boxes: ::core::clone::Clone::clone(&self.boxes),
                overlap_data: ::core::clone::Clone::clone(&self.overlap_data),
                zone_data: ::core::clone::Clone::clone(&self.zone_data),
            }
        }
    }
    impl Readable2 for BoxData {
        unsafe fn read<R: Read>(reader: &mut R, this: *mut Self) -> Result<()> {
            let boxes_len = read_len::<_, u32>(reader)?;
            read_boxed_slice_flat(reader, &raw mut (*this).boxes, boxes_len)?;
            let overlaps_len = read_len::<_, u32>(reader)?;
            read_boxed_slice_flat(reader, &raw mut (*this).overlap_data, overlaps_len)?;
            read_boxed_slice_flat(
                reader,
                &raw mut (*this).zone_data,
                boxes_len * ZONE_MULT,
            )?;
            Ok(())
        }
    }
    #[repr(C)]
    pub struct Entity {
        /// Matched to Model.id in Level.models
        pub model_id: u16,
        /// Index into Level.rooms
        pub room_index: u16,
        /// World coords
        pub pos: NoAlign<IVec3>,
        /// Units are 1/65536th of a rotation
        pub rotation: u16,
        /// If None, use mesh light
        pub brightness: Option<NonMaxU16>,
        pub flags: u16,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for Entity {
        #[inline]
        fn clone(&self) -> Entity {
            let _: ::core::clone::AssertParamIsClone<u16>;
            let _: ::core::clone::AssertParamIsClone<NoAlign<IVec3>>;
            let _: ::core::clone::AssertParamIsClone<Option<NonMaxU16>>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for Entity {}
    #[repr(C)]
    pub struct CinematicFrame {
        pub target: I16Vec3,
        pub pos: I16Vec3,
        pub fov: i16,
        pub roll: i16,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for CinematicFrame {
        #[inline]
        fn clone(&self) -> CinematicFrame {
            let _: ::core::clone::AssertParamIsClone<I16Vec3>;
            let _: ::core::clone::AssertParamIsClone<i16>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for CinematicFrame {}
    #[repr(C)]
    pub struct SoundDetails {
        /// Index into Level.sample_indices
        pub sample_index: u16,
        pub volume: u16,
        pub chance: u16,
        pub details: u16,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for SoundDetails {
        #[inline]
        fn clone(&self) -> SoundDetails {
            let _: ::core::clone::AssertParamIsClone<u16>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for SoundDetails {}
    pub struct Level {
        #[flat]
        pub version: u32,
        #[flat]
        #[list(u32)]
        pub atlases: Box<[[u8; NUM_PIXELS]]>,
        #[flat]
        pub unused: u32,
        #[delegate]
        #[list(u16)]
        pub rooms: Box<[Room]>,
        #[flat]
        #[list(u32)]
        pub floor_data: Box<[u16]>,
        #[flat]
        #[list(u32)]
        pub mesh_data: Box<[u16]>,
        /// Byte offsets into Level.mesh_data
        #[flat]
        #[list(u32)]
        pub mesh_offsets: Box<[u32]>,
        #[flat]
        #[list(u32)]
        pub animations: Box<[Animation]>,
        #[flat]
        #[list(u32)]
        pub state_changes: Box<[StateChange]>,
        #[flat]
        #[list(u32)]
        pub anim_dispatches: Box<[AnimDispatch]>,
        #[flat]
        #[list(u32)]
        pub anim_commands: Box<[u16]>,
        #[flat]
        #[list(u32)]
        pub mesh_node_data: Box<[u32]>,
        #[flat]
        #[list(u32)]
        pub frame_data: Box<[u16]>,
        #[flat]
        #[list(u32)]
        pub models: Box<[Model]>,
        #[flat]
        #[list(u32)]
        pub static_meshes: Box<[StaticMesh]>,
        #[flat]
        #[list(u32)]
        pub object_textures: Box<[ObjectTexture]>,
        #[flat]
        #[list(u32)]
        pub sprite_textures: Box<[SpriteTexture]>,
        #[flat]
        #[list(u32)]
        pub sprite_sequences: Box<[SpriteSequence]>,
        #[flat]
        #[list(u32)]
        pub cameras: Box<[Camera]>,
        #[flat]
        #[list(u32)]
        pub sound_sources: Box<[SoundSource]>,
        #[delegate]
        pub box_data: BoxData,
        #[flat]
        #[list(u32)]
        pub animated_textures: Box<[u16]>,
        #[flat]
        #[list(u32)]
        pub entities: Box<[Entity]>,
        #[flat]
        #[boxed]
        pub light_map: Box<[[u8; PALETTE_SIZE]; LIGHT_MAP_SIZE]>,
        #[flat]
        #[boxed]
        pub palette: Box<[[u8; COLOR_SIZE]; PALETTE_SIZE]>,
        #[flat]
        #[list(u16)]
        pub cinematic_frames: Box<[CinematicFrame]>,
        #[flat]
        #[list(u16)]
        pub demo_data: Box<[u8]>,
        #[flat]
        #[boxed]
        pub sound_map: Box<[u16; SOUND_MAP_SIZE]>,
        #[flat]
        #[list(u32)]
        pub sound_details: Box<[SoundDetails]>,
        #[flat]
        #[list(u32)]
        pub sample_data: Box<[u8]>,
        #[flat]
        #[list(u32)]
        pub sample_indices: Box<[u32]>,
    }
    impl tr_readable::Readable2 for Level {
        unsafe fn read<R: std::io::Read>(
            reader: &mut R,
            this: *mut Self,
        ) -> std::io::Result<()> {
            tr_readable::read_flat(reader, &raw mut (*this).version)?;
            let mut len = tr_readable::read_len::<_, u32>(reader)?;
            tr_readable::read_boxed_slice_flat(reader, &raw mut (*this).atlases, len)?;
            tr_readable::read_flat(reader, &raw mut (*this).unused)?;
            let mut len = tr_readable::read_len::<_, u16>(reader)?;
            tr_readable::read_boxed_slice_delegate(reader, &raw mut (*this).rooms, len)?;
            let mut len = tr_readable::read_len::<_, u32>(reader)?;
            tr_readable::read_boxed_slice_flat(
                reader,
                &raw mut (*this).floor_data,
                len,
            )?;
            let mut len = tr_readable::read_len::<_, u32>(reader)?;
            tr_readable::read_boxed_slice_flat(reader, &raw mut (*this).mesh_data, len)?;
            let mut len = tr_readable::read_len::<_, u32>(reader)?;
            tr_readable::read_boxed_slice_flat(
                reader,
                &raw mut (*this).mesh_offsets,
                len,
            )?;
            let mut len = tr_readable::read_len::<_, u32>(reader)?;
            tr_readable::read_boxed_slice_flat(
                reader,
                &raw mut (*this).animations,
                len,
            )?;
            let mut len = tr_readable::read_len::<_, u32>(reader)?;
            tr_readable::read_boxed_slice_flat(
                reader,
                &raw mut (*this).state_changes,
                len,
            )?;
            let mut len = tr_readable::read_len::<_, u32>(reader)?;
            tr_readable::read_boxed_slice_flat(
                reader,
                &raw mut (*this).anim_dispatches,
                len,
            )?;
            let mut len = tr_readable::read_len::<_, u32>(reader)?;
            tr_readable::read_boxed_slice_flat(
                reader,
                &raw mut (*this).anim_commands,
                len,
            )?;
            let mut len = tr_readable::read_len::<_, u32>(reader)?;
            tr_readable::read_boxed_slice_flat(
                reader,
                &raw mut (*this).mesh_node_data,
                len,
            )?;
            let mut len = tr_readable::read_len::<_, u32>(reader)?;
            tr_readable::read_boxed_slice_flat(
                reader,
                &raw mut (*this).frame_data,
                len,
            )?;
            let mut len = tr_readable::read_len::<_, u32>(reader)?;
            tr_readable::read_boxed_slice_flat(reader, &raw mut (*this).models, len)?;
            let mut len = tr_readable::read_len::<_, u32>(reader)?;
            tr_readable::read_boxed_slice_flat(
                reader,
                &raw mut (*this).static_meshes,
                len,
            )?;
            let mut len = tr_readable::read_len::<_, u32>(reader)?;
            tr_readable::read_boxed_slice_flat(
                reader,
                &raw mut (*this).object_textures,
                len,
            )?;
            let mut len = tr_readable::read_len::<_, u32>(reader)?;
            tr_readable::read_boxed_slice_flat(
                reader,
                &raw mut (*this).sprite_textures,
                len,
            )?;
            let mut len = tr_readable::read_len::<_, u32>(reader)?;
            tr_readable::read_boxed_slice_flat(
                reader,
                &raw mut (*this).sprite_sequences,
                len,
            )?;
            let mut len = tr_readable::read_len::<_, u32>(reader)?;
            tr_readable::read_boxed_slice_flat(reader, &raw mut (*this).cameras, len)?;
            let mut len = tr_readable::read_len::<_, u32>(reader)?;
            tr_readable::read_boxed_slice_flat(
                reader,
                &raw mut (*this).sound_sources,
                len,
            )?;
            tr_readable::Readable2::read(reader, &raw mut (*this).box_data)?;
            let mut len = tr_readable::read_len::<_, u32>(reader)?;
            tr_readable::read_boxed_slice_flat(
                reader,
                &raw mut (*this).animated_textures,
                len,
            )?;
            let mut len = tr_readable::read_len::<_, u32>(reader)?;
            tr_readable::read_boxed_slice_flat(reader, &raw mut (*this).entities, len)?;
            tr_readable::read_boxed_flat(reader, &raw mut (*this).light_map)?;
            tr_readable::read_boxed_flat(reader, &raw mut (*this).palette)?;
            let mut len = tr_readable::read_len::<_, u16>(reader)?;
            tr_readable::read_boxed_slice_flat(
                reader,
                &raw mut (*this).cinematic_frames,
                len,
            )?;
            let mut len = tr_readable::read_len::<_, u16>(reader)?;
            tr_readable::read_boxed_slice_flat(reader, &raw mut (*this).demo_data, len)?;
            tr_readable::read_boxed_flat(reader, &raw mut (*this).sound_map)?;
            let mut len = tr_readable::read_len::<_, u32>(reader)?;
            tr_readable::read_boxed_slice_flat(
                reader,
                &raw mut (*this).sound_details,
                len,
            )?;
            let mut len = tr_readable::read_len::<_, u32>(reader)?;
            tr_readable::read_boxed_slice_flat(
                reader,
                &raw mut (*this).sample_data,
                len,
            )?;
            let mut len = tr_readable::read_len::<_, u32>(reader)?;
            tr_readable::read_boxed_slice_flat(
                reader,
                &raw mut (*this).sample_indices,
                len,
            )?;
            Ok(())
        }
    }
    pub struct RoomGeom<'a> {
        pub vertices: &'a [RoomVertex],
        pub quads: &'a [Quad],
        pub tris: &'a [Tri],
        pub sprites: &'a [Sprite],
    }
    impl Room {
        pub fn xz2(&self) -> IVec2 {
            IVec2::new(self.x, self.z)
        }
        pub fn xz3(&self) -> IVec3 {
            IVec3::new(self.x, 0, self.z)
        }
        pub fn get_geom_data(&self) -> RoomGeom {
            let verts_start = 1;
            let verts_end = verts_start
                + self.geom_data[0] as usize * (size_of::<RoomVertex>() / 2);
            let quads_start = verts_end + 1;
            let quads_end = quads_start
                + self.geom_data[verts_end] as usize * (size_of::<Quad>() / 2);
            let tris_start = quads_end + 1;
            let tris_end = tris_start
                + self.geom_data[quads_end] as usize * (size_of::<Tri>() / 2);
            let sprites_start = tris_end + 1;
            let sprites_end = sprites_start
                + self.geom_data[tris_end] as usize * (size_of::<Sprite>() / 2);
            unsafe {
                RoomGeom {
                    vertices: reinterpret::slice(
                        &self.geom_data[verts_start..verts_end],
                    ),
                    quads: reinterpret::slice(&self.geom_data[quads_start..quads_end]),
                    tris: reinterpret::slice(&self.geom_data[tris_start..tris_end]),
                    sprites: reinterpret::slice(
                        &self.geom_data[sprites_start..sprites_end],
                    ),
                }
            }
        }
    }
}
