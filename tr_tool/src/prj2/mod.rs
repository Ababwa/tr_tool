mod enums;

use core::str;
use std::{fmt::Debug, io::{Error, Result, Write}, path::{self, PathBuf}};
use enums::*;
use glam::{i16vec2, I16Vec3, Vec3, Vec3Swizzles};
use tr_model::tr1;
use crate::{as_bytes::AsBytes, palette_images_to_rgba, tr_traits::Room};

const PRJ2: &[u8; 4] = b"PRJ2";

//type is a discriminant to enable multiple blanket impls
trait ToBytes<T> {
	type Bytes: AsRef<[u8]>;
	fn to_bytes(self) -> Result<Self::Bytes>;
}

// impl ToBytes<[(); 0]> for &[u8] {
// 	type Bytes = Self;
// 	fn to_bytes(self) -> Result<Self::Bytes> {
// 		Ok(self)
// 	}
// }

impl ToBytes<[(); 0]> for &str {
	type Bytes = Self;
	fn to_bytes(self) -> Result<Self::Bytes> {
		Ok(self)
	}
}

impl ToBytes<[(); 0]> for PathBuf {
	type Bytes = String;
	fn to_bytes(self) -> Result<Self::Bytes> {
		Ok(self.into_os_string().into_string().expect("not UTF-8"))
	}
}

struct Leb128<T>(T);
struct Leb128Bytes([u8; 9], u8);

impl AsRef<[u8]> for Leb128Bytes {
	fn as_ref(&self) -> &[u8] {
		&self.0[..self.1 as usize]
	}
}

impl<T> ToBytes<[(); 0]> for Leb128<T> where T: TryInto<i64>, T::Error: Debug {
	type Bytes = Leb128Bytes;
	fn to_bytes(self) -> Result<Self::Bytes> {
		let mut bytes = [0; 9];
		let len = (&mut bytes[..]).leb128(self.0)?;
		Ok(Leb128Bytes(bytes, len as u8))
	}
}

impl<F: FnOnce(&mut Vec<u8>) -> Result<()>> ToBytes<[(); 0]> for F {
	type Bytes = Vec<u8>;
	fn to_bytes(self) -> Result<Self::Bytes> {
		let mut vec = vec![];
		self(&mut vec)?;
		Ok(vec)
	}
}

struct AsBytesRef<T>(T);

impl<T: AsBytes> AsRef<[u8]> for AsBytesRef<T> {
	fn as_ref(&self) -> &[u8] {
		self.0.as_bytes()
	}
}

impl<T: AsBytes> ToBytes<[(); 1]> for T {
	type Bytes = AsBytesRef<T>;
	fn to_bytes(self) -> Result<Self::Bytes> {
		Ok(AsBytesRef(self))
	}
}

struct ChunkWriter<'a, T>(&'a mut T);

impl<'a, T: Write> ChunkWriter<'a, T> {
	fn chunk<A, D: ToBytes<A>>(&mut self, id: &[u8], data: D) -> Result<()> {
		self.0.leb128(id.len())?;
		self.0.write_all(id)?;
		let data = data.to_bytes()?;
		let data = data.as_ref();
		self.0.leb128(data.len())?;
		self.0.write_all(&data)?;
		Ok(())
	}
}

trait WriteExt: Write + Sized {
	fn leb128<N>(&mut self, num: N) -> Result<usize> where N: TryInto<i64>, N::Error: Debug {
		leb128::write::signed(self, num.try_into().unwrap())
	}
	
	fn chunk_stream<F: FnOnce(ChunkWriter<Self>) -> Result<()>>(&mut self, f: F) -> Result<()> {
		f(ChunkWriter(self))?;
		self.write_all(&[0])?;
		Ok(())
	}
}

impl<W: Write> WriteExt for W {}

type V<'a> = &'a mut Vec<u8>;

fn room_type(room_flags: tr1::RoomFlags) -> RoomType {
	if room_flags.water() {
		RoomType::Water
	} else {
		RoomType::Normal
	}
}

const SECTOR_FLAGS_CHUNK_ID: &[u8] = &[0];
const SECTOR_FLOOR_CHUNK_ID: &[u8] = &[7];
const SECTOR_CEILING_CHUNK_ID: &[u8] = &[8];
const SECTOR_FLOOR_DIVISIONS_CHUNK_ID: &[u8] = &[9];
const SECTOR_CEILING_DIVISIONS_CHUNK_ID: &[u8] = &[10];

fn snap_vert(v: I16Vec3) -> Option<I16Vec3> {
	const DIVISOR: I16Vec3 = I16Vec3::new(1024, 256, 1024);
	I16Vec3::ZERO.cmpeq(v % DIVISOR).all().then_some(v / DIVISOR)
}

struct SnappedFace<const N: usize> {
	vertices: [I16Vec3; N],
	object_texture_index: u16,
}

fn snap_face_verts<const N: usize>(
	vertices: &[Option<I16Vec3>], face_vertex_indices: [u16; N],
) -> Option<[I16Vec3; N]> {
	let mut face_vertices = [I16Vec3::ZERO; N];
	for i in 0..N {
		face_vertices[i] = vertices[face_vertex_indices[i] as usize]?;
	}
	Some(face_vertices)
}

fn snap_face<const N: usize>(
	snapped_verts: &[Option<I16Vec3>], face_vertex_indices: [u16; N], object_texture_index: u16,
) -> Option<SnappedFace<N>> {
	Some(SnappedFace {
		vertices: snap_face_verts(&snapped_verts, face_vertex_indices)?,
		object_texture_index,
	})
}

// fn get_dir(verts: &[I16Vec3], dim: usize) -> i16 {
// 	let d1 = (dim + 1) % 3;
// 	let d2 = (dim + 2) % 3;
// 	let a1 = verts[1][d1] - verts[0][d1];
// 	let a2 = verts[1][d2] - verts[0][d2];
// 	let b1 = verts[2][d1] - verts[0][d1];
// 	let b2 = verts[2][d2] - verts[0][d2];
// 	(a2 * b1 - a1 * b2).signum()
// }

/*
x increases east
y increases down
z increases north
*/

const X: usize = 0;
const Y: usize = 1;
const Z: usize = 2;

#[derive(Clone, Copy)]
struct SectorQuad {
	/// xnzp, xpzp, xpzn, xnzn
	corner_heights: [i16; 4],
	object_texture_index: u16,
}

struct SnappedFaceRef<'a> {
	vertices: &'a [I16Vec3],
	object_texture_index: u16,
}

impl<'a, const N: usize> From<&'a SnappedFace<N>> for SnappedFaceRef<'a> {
	fn from(face: &'a SnappedFace<N>) -> Self {
		Self {
			vertices: &face.vertices,
			object_texture_index: face.object_texture_index,
		}
	}
}

fn floor_ceiling_quad(
	snapped_quads: &[SnappedFace<4>], sec_x: i16, sec_z: i16,
) -> [Option<SectorQuad>; 2] {
	let sec_pos = i16vec2(sec_x, sec_z);
	let mut floor_ceiling = [None, None];
	let mut fc_indices = &[0, 1][..];
	'outer: for &SnappedFace { vertices, object_texture_index } in snapped_quads {
		let Some(xnzn) = vertices.iter().position(|v| v.xz() == sec_pos) else {
			continue;
		};
		let xpzp = (xnzn + 2) % 4;
		if vertices[xpzp].xz() != sec_pos + 1 {
			continue;
		}
		let other_corners = [(xnzn + 1) % 4, (xnzn + 3) % 4];
		for &fc_index in fc_indices {
			let xnzp = other_corners[fc_index];
			let xpzn = other_corners[1 - fc_index];
			if {
				vertices[xnzp].xz() == i16vec2(sec_x, sec_z + 1) &&
				vertices[xpzn].xz() == i16vec2(sec_x + 1, sec_z)
			} {
				floor_ceiling[fc_index] = Some(SectorQuad {
					corner_heights: [xnzp, xpzp, xpzn, xnzn].map(|i| vertices[i].y),
					object_texture_index,
				});
				fc_indices = &fc_indices[(1 - fc_index)..][..fc_indices.len() - 1];
				if fc_indices.is_empty() {
					break 'outer;
				} else {
					break;
				}
			}
		}
	}
	floor_ceiling
}

const LEFT: usize = 0;
const RIGHT: usize = 1;

struct WallFace {
	/// y values of a wall segment: `[[left top, left bottom], [right top, right bottom]]`
	heights: [[i16; 2]; 2],
	object_texture_index: u16,
}

impl WallFace {
	fn height(&self) -> i16 {
		let [[a, b], [c, d]] = self.heights;
		a + b + c + d
	}
	
	fn side_height(&self, side: usize) -> i16 {
		let [a, b] = self.heights[side];
		a + b
	}
}

const XN: usize = 0;
const ZP: usize = 1;
const XP: usize = 2;
const ZN: usize = 3;

/**
Get wall faces for a sector.  
Wall vertices facing the camera are clockwise.  
Quads:
```
0-1    1-2    2-3    3-0
| | or | | or | | or | |
3-2    0-3    1-0    2-1
```
Tris:
```
(left-hugging)         (right-hugging)
0      1      2        0      1      2
|\  or |\  or |\  or  /| or  /| or  /|
2-1    0-2    1-0    2-1    0-2    1-0
```
There is always a top and bottom line between the sector edges.  
Quads:
```
L-R

L-R
```
Tris:
```
L        R
 \  or  /
L-R    L-R
```
*/
fn get_sector_walls(
	snapped_quads: &[SnappedFace<4>], snapped_tris: &[SnappedFace<3>], sec_x: i16, sec_z: i16,
) -> [Vec<WallFace>; 4] {
	let mut walls = [const { Vec::<WallFace>::new() }; 4];//xn, zp, xp, zn
	for SnappedFaceRef { vertices, object_texture_index } in Iterator::chain(
		snapped_quads.iter().map(SnappedFaceRef::from),
		snapped_tris.iter().map(SnappedFaceRef::from),
	) {
		let Some((side, dim, left, right, _)) = [
			(XN, X, sec_z + 1, sec_z, sec_x),
			(ZP, Z, sec_x + 1, sec_x, sec_z + 1),
			(XP, X, sec_z, sec_z + 1, sec_x + 1),
			(ZN, Z, sec_x, sec_x + 1, sec_z),
		].into_iter().find(|&(_, dim, _, _, plane)| vertices.iter().all(|v| v[dim] == plane)) else {
			continue;//not on any of the four sector wall planes
		};
		let other_dim = 2 - dim;
		let mut top = None;
		let mut bottom = None;
		'verts: for i in 0..vertices.len() {
			let a = vertices[i];
			let b = vertices[(i + 1) % vertices.len()];
			let face_side = [a, b];
			let side_other_dim = face_side.map(|v| v[other_dim]);
			for (bounds, top_bottom) in [([left, right], &mut top), ([right, left], &mut bottom)] {
				if side_other_dim == bounds {
					assert!(top_bottom.is_none());
					*top_bottom = Some(face_side.map(|v| v.y));
					continue 'verts;
				}
			}
		}
		let (Some([left_top, right_top]), Some([right_bottom, left_bottom])) = (top, bottom) else {
			continue;//not bound by this sector
		};
		let top_height = left_top + right_top;
		let bottom_height = left_bottom + right_bottom;
		assert!(top_height != bottom_height);
		if top_height > bottom_height {
			continue;//facing wrong way
		}
		let height = top_height + bottom_height;
		let wall = &mut walls[side];
		let pos = wall.binary_search_by_key(&height, |wall_face| wall_face.height()).unwrap_err();
		wall.insert(
			pos,
			WallFace { heights: [[left_top, left_bottom], [right_top, right_bottom]], object_texture_index },
		);
	}
	walls
}

struct TextureToAdd {
	sector_face: SectorFace,
	object_texture_index: u16,
}


fn overlap([left_top, left_bottom]: [i16; 2], [right_top, right_bottom]: [i16; 2]) -> bool {
	left_top < right_bottom && left_bottom > right_top
}

const CW: isize = -1;
const CCW: isize = 1;

fn find_connection(left: &[WallFace], right: &[WallFace], dir: isize, offset: isize) -> Option<isize> {
	for left_i in 0..left.len() {
		let left_edge = left[left_i].heights[RIGHT];
		if left_edge[0] != left_edge[1] {
			for right_i in 0..right.len() {
				let right_edge = right[right_i].heights[LEFT];
				if right_edge[0] != right_edge[1] {
					if overlap(left_edge, right_edge) {
						return Some((left_i as isize - right_i as isize) * dir + offset);
					}
				}
			}
		}
	}
	None
}

const TOP: usize = 0;
const BOTTOM: usize = 1;

fn find_closest(left: &[WallFace], right: &[WallFace], dir: isize) -> Option<(isize, i16)> {
	// if left.last()?.heights[RIGHT][BOTTOM] <= right.first()?.heights[LEFT][TOP] {
	// 	return Some(left.len() as isize - 1);
	// }
	// if left.first()?.heights[RIGHT][TOP] >= right.last()?.heights[LEFT][BOTTOM] {
	// 	return Some(1 - right.len() as isize);
	// }
	if left.is_empty() || right.is_empty() {
		return None;
	}
	let mut closest_dist = i16::MAX;
	let mut closest = 0;
	for left_i in 0..left.len() {
		let left_height = left[left_i].side_height(RIGHT);
		for right_i in 0..right.len() {
			let right_height = right[right_i].side_height(LEFT);
			let dist = (left_height - right_height).abs();
			if dist <= closest_dist {
				closest_dist = dist;
				closest = left_i as isize - right_i as isize;
			}
		}
	}
	Some((closest * dir, closest_dist))
}

/*
side indices:
+1+
0 2
+3+
corner indices:
0-1
| |
3-2
*/

fn get_corner_heights(
	walls: &[Vec<WallFace>; 4], offsets: &[usize; 4], layer: usize, top_bottom: usize,
) -> [i16; 4] {
	const NULL: i16 = i16::MAX;
	let mut corner_heights = [NULL; 4];
	for corner in 0..4 {
		let left_i = (corner + 1) % 4;
		let right_i = corner;
		let [left, right] = [(left_i, RIGHT), (right_i, LEFT)].map(|(i, side)| {
			layer
				.checked_sub(offsets[i])
				.and_then(|face_index| walls[i].get(face_index))
				.map(|face| face.heights[side][top_bottom])
		});
		match (left, right) {
			(Some(left), Some(right)) => corner_heights[corner] = [left, right][(left > right) as usize ^ top_bottom],
			(Some(height), None) | (None, Some(height)) => corner_heights[corner] = height,
			_ => {},
		}
	}
	assert!(corner_heights != [NULL; 4]);
	for corner in 0..4 {
		match [
			corner_heights[(corner + 3) % 4],
			corner_heights[corner],
			corner_heights[(corner + 1) % 4],
		] {
			[NULL, NULL, NULL] => corner_heights[corner] = corner_heights[(corner + 2) % 4],
			[NULL, NULL, a] => corner_heights[corner] = a,
			[a, NULL, NULL] => corner_heights[corner] = a,
			[a, NULL, b] => corner_heights[corner] = (a + b) / 2,
			_ => {},
		}
	}
	corner_heights
}

#[derive(Default)]
struct WallsResult {
	wall: Option<bool>,
	divisions: Vec<[i16; 4]>,
}

/**
`Some(true)` if the sector is a Tomb Editor wall.  
`Some(false)` if the sector has space.  
`None` if it could not be determined.
*/
fn get_walls(floor: Option<SectorQuad>, ceiling: Option<SectorQuad>, walls: &[Vec<WallFace>; 4]) -> WallsResult {
	// let mut textures = vec![];
	match (floor, ceiling) {
		(None, None) => {
			if walls.iter().all(|wall| wall.is_empty()) {
				WallsResult::default()//either an inner wall (walls on all 4 sides) or empty space
			} else {
				let side0_index = (0..4).max_by_key(|&i| walls[i].len()).unwrap();
				let side1_index = (side0_index + 1) % 4;//left (clockwise)
				let side2_index = (side0_index + 2) % 4;//opposite
				let side3_index = (side0_index + 3) % 4;//right (counter-clockwise)
				let side0 = &walls[side0_index];
				let side1 = &walls[side1_index];
				let side2 = &walls[side2_index];
				let side3 = &walls[side3_index];
				//first, try to determine offsets by connections to side0
				let mut side1_offset = find_connection(side1, side0, CW, 0);
				let mut side2_offset = None;
				let mut side3_offset = find_connection(side0, side3, CCW, 0);
				if let Some(side1_offset) = side1_offset {
					side2_offset = find_connection(side2, side1, CW, side1_offset);
				}
				if let (None, Some(side3_offset)) = (side2_offset, side3_offset) {
					side2_offset = find_connection(side3, side2, CCW, side3_offset);
				}
				if let (None, Some(side2_offset)) = (side1_offset, side2_offset) {
					side1_offset = find_connection(side2, side1, CCW, side2_offset);
				}
				if let (None, Some(side2_offset)) = (side3_offset, side2_offset) {
					side3_offset = find_connection(side3, side2, CW, side2_offset);
				}
				//check for connections between adjacent sides and opposite side, if no side0 connection
				let mut side1_from2 = None;
				let mut side3_from2 = None;
				if let (None, None) = (side1_offset, side2_offset) {
					side1_from2 = find_connection(side2, side1, CCW, 0);
				}
				if let (None, None) = (side3_offset, side2_offset) {
					side3_from2 = find_connection(side3, side2, CW, 0);
				}
				//"glue" connected sections with a single `find_closest` using the closer of the two
				/*
				handles these cases (# = connected):
				#1  |  1# |  1#
				0 2 | 0 2 | 0 2
				 3# | #3  |  3#
				case handled below:
				#1
				0 2
				#3
				*/
				//oc: offset candidate
				match (side1_from2, side3_from2, side1_offset, side3_offset) {
					(Some(side1_from2), Some(side3_from2), ..) => {
						let (Some((side1_oc, side1_dist)), Some((side3_oc, side3_dist))) = (find_closest(side1, side0, CW), find_closest(side0, side3, CCW)) else {
							unreachable!();
						};
						if side1_dist < side3_dist {
							side1_offset = Some(side1_oc);
							side2_offset = Some(side1_oc - side1_from2);
							side3_offset = Some(side1_oc - side1_from2 + side3_from2);
						} else {
							side3_offset = Some(side3_oc);
							side2_offset = Some(side3_oc - side3_from2);
							side1_offset = Some(side3_oc - side3_from2 + side1_from2);
						}
					},
					(Some(side1_from2), None, _, Some(side3_offset)) => {
						let (Some((side1_oc, side1_dist)), Some((side2_from3, side2_dist))) = (find_closest(side1, side0, CW), find_closest(side3, side2, CCW)) else {
							unreachable!();
						};
						if side1_dist < side2_dist {
							side1_offset = Some(side1_oc);
							side2_offset = Some(side1_oc - side1_from2);
						} else {
							side2_offset = Some(side3_offset + side2_from3);
							side1_offset = Some(side3_offset + side2_from3 + side1_from2);
						}
					},
					(None, Some(side3_from2), Some(side1_offset), _) => {
						let (Some((side2_from1, side2_dist)), Some((side3_oc, side3_dist))) = (find_closest(side2, side1, CW), find_closest(side0, side3, CCW)) else {
							unreachable!();
						};
						if side2_dist < side3_dist {
							side2_offset = Some(side1_offset + side2_from1);
							side3_offset = Some(side1_offset + side2_from1 + side3_from2);
						} else {
							side3_offset = Some(side3_oc);
							side2_offset = Some(side3_oc - side3_from2);
						}
					},
					_ => {},
				}
				if let (Some(side1_offset), None, Some(side3_offset)) = (side1_offset, side2_offset, side2_offset) {
					if let (Some((side2_from1, side1_dist)), Some((side2_from3, side3_dist))) = (find_closest(side2, side1, CW), find_closest(side3, side2, CCW)) {
						if side1_dist < side3_dist {
							side2_offset = Some(side1_offset + side2_from1);
						} else {
							side2_offset = Some(side3_offset + side2_from3);
						}
					};
				}
				//at this point, any unknown offsets will be guessed using find_closest
				if let None = side1_offset {
					side1_offset = find_closest(side1, side0, CW).map(|(o, _)| o);
				}
				if let None = side3_offset {
					side3_offset = find_closest(side0, side3, CCW).map(|(o, _)| o);
				}
				match (side2_offset, side1_offset, side3_offset) {
					(None, Some(side1_offset), Some(side3_offset)) => {
						if let (Some((side2_from1, side1_dist)), Some((side2_from3, side3_dist))) = (find_closest(side2, side1, CW), find_closest(side3, side2, CCW)) {
							if side1_dist < side3_dist {
								side2_offset = Some(side1_offset + side2_from1);
							} else {
								side2_offset = Some(side3_offset + side2_from3);
							}
						}
					},
					(None, Some(side1_offset), None) => {
						side2_offset = find_closest(side2, side1, CW).map(|(o, _)| o + side1_offset);
					},
					(None, None, Some(side3_offset)) => {
						side2_offset = find_closest(side3, side2, CCW).map(|(o, _)| o + side3_offset);
					},
					_ => {},
				}
				//at this point, `None` offsets must be empty walls
				let [side1_offset, side2_offset, side3_offset] = [side1_offset, side2_offset, side3_offset].map(|o| o.unwrap_or_default());
				let mut offsets = [0, side1_offset, side2_offset, side3_offset];
				offsets.rotate_right(side0_index);
				let offset_minimum = offsets.into_iter().min().unwrap();
				let offsets: [usize; 4] = offsets.map(|o| (o - offset_minimum).try_into().unwrap());//TODO: change to `as` once confident
				/*
				ABCD are wall sides
				0|A
				1|AB D
				2| BCD
				3| B
				offsets:
				A: 0
				B: 1
				C: 2
				D: 1
				*/
				let num_divisions = (0..4).map(|i| offsets[i] + walls[i].len()).max().unwrap() + 1;
				let mut divisions = Vec::with_capacity(num_divisions);
				divisions.push(get_corner_heights(walls, &offsets, 0, TOP));
				for layer in 0..num_divisions - 1 {
					divisions.push(get_corner_heights(walls, &offsets, layer, BOTTOM));
				}
				WallsResult {
					wall: Some(true),
					divisions,
				}
			}
		},
		(Some(floor), None) => {
			WallsResult {
				wall: Some(false),
				divisions: vec![],
			}
		},
		(None, Some(ceiling)) => {
			WallsResult {
				wall: Some(false),
				divisions: vec![],
			}
		},
		(Some(floor), Some(ceiling)) => {
			WallsResult {
				wall: Some(false),
				divisions: vec![],
			}
		},
	}
}

pub fn export<W: Write>(w: &mut W, level: &tr1::Level, wad_path: &str, textures_path: &str) -> Result<()> {
	let wad_path = path::absolute(wad_path)?;
	let textures_path = path::absolute(textures_path)?;
	let rgba = palette_images_to_rgba(&level.palette, &level.atlases);
	let height = rgba.len() as u32 / 1024;
	image::save_buffer(&textures_path, &rgba, 256, height, image::ColorType::Rgba8).map_err(Error::other)?;
	w.write_all(PRJ2)?;
	w.write_all(&[0; 4])?;
	w.chunk_stream(|mut s| {
		s.chunk(b"TeSettings", |c: V| {
			c.chunk_stream(|mut s| {
				s.chunk(b"TeGameVersion", Leb128(GameVersion::Tr1))?;
				s.chunk(b"TeSoundSystem", Leb128(SoundSystem::Xml))?;
				s.chunk(b"TeWads", |c: V| {
					c.chunk_stream(|mut s| {
						s.chunk(b"TeWad", |c: V| {
							c.chunk_stream(|mut s| {
								s.chunk(b"TePath", wad_path)
							})
						})
					})
				})?;
				s.chunk(b"TeTextures", |c: V| {
					c.chunk_stream(|mut s| {
						s.chunk(b"TeLvlTexture", |c: V| {
							c.chunk_stream(|mut s| {
								s.chunk(b"TeI", Leb128(0))?;
								s.chunk(b"TePath", textures_path)?;
								Ok(())
							})
						})
					})
				})?;
				Ok(())
			})
		})?;
		s.chunk(b"TeRooms", |c: V| {
			c.chunk_stream(|mut s| {
				for (room_index, room) in level.rooms.iter().enumerate() {
					let sectors_x = room.num_sectors.x as i16;
					let sectors_z = room.num_sectors.z as i16;
					println!("room {}: sectors: x: {}, z: {}", room_index, sectors_x, sectors_z);
					let snapped_verts = room.vertices.iter().map(|v| snap_vert(v.pos)).collect::<Vec<_>>();
					let snapped_quads = room.quads.iter().filter_map(|&tr1::TexturedQuad { vertex_indices, object_texture_index }| snap_face(&snapped_verts, vertex_indices, object_texture_index)).collect::<Vec<_>>();
					let snapped_tris = room.tris.iter().filter_map(|&tr1::TexturedTri { vertex_indices, object_texture_index }| snap_face(&snapped_verts, vertex_indices, object_texture_index)).collect::<Vec<_>>();
					s.chunk(b"TeRoom", |c: V| {
						c.leb128(sectors_x)?;
						c.leb128(sectors_z)?;
						c.chunk_stream(|mut s| {
							s.chunk(b"TeI", Leb128(room_index))?;
							s.chunk(b"TePos2", room.pos().as_vec3() / 1024.0)?;
							s.chunk(b"TeAmbient", Vec3::splat(1.0 - (room.ambient_light as f32 / 8191.0)))?;
							s.chunk(b"TeRoomType", Leb128(room_type(room.flags)))?;
							if room.flip_room_index != u16::MAX {
								s.chunk(b"TeAlternate", |c: V| {
									c.chunk_stream(|mut s| {
										s.chunk(b"TeRoom", Leb128(room.flip_room_index))
									})
								})?;
							}
							s.chunk(b"TeSecs", |c: V| {
								c.chunk_stream(|mut s| {
									for (index, sector) in room.sectors.iter().enumerate() {
										let index = index as i16;
										let sec_x = index / sectors_z;
										let sec_z = index % sectors_z;
										let position = (sec_z * sectors_x + sec_x) as u32;
										if true {//(1..sectors_x - 1).contains(&sec_x) && (1..sectors_z - 1).contains(&sec_z) {
											let mut flags = SectorFlags::empty();
											let [floor, ceiling] = floor_ceiling_quad(&snapped_quads, sec_x, sec_z);
											let walls = get_sector_walls(&snapped_quads, &snapped_tris, sec_x, sec_z);
											let WallsResult { wall, divisions } = get_walls(floor, ceiling, &walls);
											if !matches!(wall, Some(false)) {
												flags |= SectorFlags::WALL;
											}
											s.chunk(b"TeS", |c: V| {
												c.write_all(position.as_bytes())?;
												c.chunk_stream(|mut s| {
													s.chunk(SECTOR_FLAGS_CHUNK_ID, Leb128(flags))?;
													if divisions.is_empty() {
														s.chunk(SECTOR_FLOOR_CHUNK_ID, |c: V| {
															c.leb128(0)?;//diagonal flags
															if let Some(floor) = floor {
																for corner_height in floor.corner_heights {
																	c.leb128(corner_height * -256)?;
																}
															} else {
																let floor = Leb128(sector.floor as i16 * -256).to_bytes()?;
																let floor = floor.as_ref();
																c.write_all(floor)?;
																c.write_all(floor)?;
																c.write_all(floor)?;
																c.write_all(floor)?;
															}
															Ok(())
														})?;
														s.chunk(SECTOR_CEILING_CHUNK_ID, |c: V| {
															c.leb128(0)?;//diagonal flags
															if let Some(ceiling) = ceiling {
																for corner_height in ceiling.corner_heights {
																	c.leb128(corner_height * -256)?;
																}
															} else {
																let ceiling = Leb128(sector.ceiling as i16 * -256).to_bytes()?;
																let ceiling = ceiling.as_ref();
																c.write_all(ceiling)?;
																c.write_all(ceiling)?;
																c.write_all(ceiling)?;
																c.write_all(ceiling)?;
															}
															Ok(())
														})?;
													} else {
														let ceiling_divs = divisions.len() / 2;
														let floor_divs = divisions.len() - ceiling_divs;
														s.chunk(SECTOR_CEILING_DIVISIONS_CHUNK_ID, |c: V| {
															c.leb128(ceiling_divs)?;
															for &div in divisions[..ceiling_divs].iter().rev() {
																for h in div {
																	c.leb128(h * -256)?;
																}
															}
															Ok(())
														})?;
														s.chunk(SECTOR_FLOOR_DIVISIONS_CHUNK_ID, |c: V| {
															c.leb128(floor_divs)?;
															for &div in &divisions[ceiling_divs..] {
																for h in div {
																	c.leb128(h * -256)?;
																}
															}
															Ok(())
														})?;
													}
													Ok(())
												})?;
												Ok(())
											})?;
										}
									}
									Ok(())
								})
							})?;
							Ok(())
						})
					})?;
				}
				Ok(())
			})
		})?;
		Ok(())
	})
}
