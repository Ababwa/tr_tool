use glam::{I16Vec2, Vec3Swizzles};
use super::{floor_ceiling::SectorSurface, SnappedFace, SnappedFaceRef};

struct WallFace {
	/// y values of a wall segment: `[[left top, left bottom], [right top, right bottom]]`
	heights: [[i16; 2]; 2],
	object_texture_index: u16,
}

/**
Get sector wall faces by side.  
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
There is always a top and bottom line of a wall face.
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
Finds faces on the eight wall planes (xn, zp, xp, xn, xnzp, xpzp, xpzn, xnzn) and returns y values of the
top and bottom left and right vertices.
*/
fn get_sides(
	snapped_quads: &[SnappedFace<4>], snapped_tris: &[SnappedFace<3>], sec_x: i16, sec_z: i16,
) -> [Vec<WallFace>; 8] {
	/// sector x, z offsets for left and right sides of faces facing xn, zp, xp, zn, xnzp, xpzp, xpzn, xnzn
	const LEFT_RIGHT_XZ: [[[i16; 2]; 2]; 8] = [
		[[0, 1], [0, 0]],
		[[1, 1], [0, 1]],
		[[1, 0], [1, 1]],
		[[0, 0], [1, 0]],
		[[1, 1], [0, 0]],
		[[1, 0], [0, 1]],
		[[0, 0], [1, 1]],
		[[0, 1], [1, 0]],
	];
	let sec = I16Vec2::new(sec_x, sec_z);
	let left_rights = LEFT_RIGHT_XZ.map(|left_right| left_right.map(|o| I16Vec2::from(o) + sec));
	let mut sides = [const { Vec::<WallFace>::new() }; 8];
	for SnappedFaceRef { vertices, object_texture_index } in Iterator::chain(
		snapped_quads.iter().map(SnappedFaceRef::from),
		snapped_tris.iter().map(SnappedFaceRef::from),
	) {
		for (side_index, [left, right]) in left_rights.into_iter().enumerate() {
			let mut top_bottom = [None, None];
			for i in 0..vertices.len() {
				let poly_side = [vertices[i], vertices[(i + 1) % vertices.len()]];
				let side_xz = poly_side.map(|v| v.xz());
				for j in 0..2 {
					if side_xz[j] == left && side_xz[1 - j] == right {
						assert!(top_bottom[j].is_none());
						top_bottom[j] = Some(poly_side.map(|v| v.y));
					}
				}
			}
			let [Some([left_top, right_top]), Some([right_bottom, left_bottom])] = top_bottom else {
				continue;//not constrained by this side
			};
			let top_height = left_top + right_top;
			let bottom_height = right_bottom + left_bottom;
			assert!(top_height != bottom_height);
			if top_height > bottom_height {
				continue;//facing wrong way
			}
			let side = &mut sides[side_index];
			let key = |&WallFace { heights: [[left_top, _], [right_top, _]], .. }| left_top + right_top;
			let pos = side.binary_search_by_key(&top_height, key).unwrap_err();
			side.insert(
				pos,
				WallFace {
					heights: [[left_top, left_bottom], [right_top, right_bottom]],
					object_texture_index,
				},
			);
		}
	}
	sides
}

const LEFT: usize = 0;
const RIGHT: usize = 1;

fn find_connection(left: &[WallFace], right: &[WallFace]) -> Option<isize> {
	for left_i in 0..left.len() {
		let [left_top, left_bottom] = left[left_i].heights[RIGHT];
		if left_top != left_bottom {
			for right_i in 0..right.len() {
				let [right_top, right_bottom] = right[right_i].heights[LEFT];
				if right_top != right_bottom {
					if left_top < right_bottom && left_bottom > right_top {
						return Some(left_i as isize - right_i as isize);
					}
				}
			}
		}
	}
	None
}

fn find_closest(left: &[WallFace], right: &[WallFace]) -> Option<(isize, i16)> {
	if left.is_empty() || right.is_empty() {
		return None;
	}
	let mut closest_dist = i16::MAX;
	let mut closest = 0;
	for left_i in 0..left.len() {
		let left_heights = left[left_i].heights[RIGHT];
		for right_i in 0..right.len() {
			let right_heights = right[right_i].heights[LEFT];
			let [top_d, bottom_d] = std::array::from_fn(|i| (left_heights[i] - right_heights[i]).abs());
			let dist = top_d + bottom_d;
			if dist <= closest_dist {
				closest_dist = dist;
				closest = left_i as isize - right_i as isize;
			}
		}
	}
	Some((closest, closest_dist))
}

/*
Sector walls:

 1
0☐2
 3

4/\5
7\/6

xn, zp, xp, xn, xnzp, xpzp, xpzn, xnzn
0,  1,  2,  3,    4,    5,    6,    7
*/

/// Every `[right, left]` sector side index pair of touching sides.
const SIDE_MEETINGS: [[usize; 2]; 12] = [
	[0, 1],//	0	|‾
	[1, 2],//	1	‾|
	[2, 3],//	2	_|
	[3, 0],//	3	|_
	
	[1, 6],//	4	‾/
	[2, 7],//	5	\|
	[3, 4],//	6	/_
	[0, 5],//	7	|\
	
	[6, 0],//	8	|/
	[7, 1],//	9	\‾
	[4, 2],//	10	/|
	[5, 3],//	11	_\
];

/// Indices into `SIDE_MEETINGS`.
const CORNER_RINGS: [&[usize]; 5] = [
	&[0, 1, 2, 3],
	&[0, 4, 8],
	&[1, 5, 9],
	&[2, 6, 10],
	&[3, 7, 11],
];

/// Sector side indices.
// const SIDE_RINGS: [&[usize]; 5] = [
// 	&[0, 1, 2, 3],
// 	&[0, 1, 6],
// 	&[1, 2, 7],
// 	&[2, 3, 4],
// 	&[3, 0, 5],
// ];

fn get_corner_heights(
	sides: &[&[WallFace]; 8], offsets: &[usize; 8], layer: usize, top_bottom: usize,
) -> [i16; 4] {
	const CORNER_SIDES: [[[usize; 2]; 2]; 4] = [
		[[0, 7], [1, 5]],
		[[1, 4], [2, 6]],
		[[2, 5], [3, 7]],
		[[3, 6], [0, 4]],
	];
	
	// let side_lens = sides.map(|s| s.len());
	// println!("side_lens: {:?}", side_lens);
	// println!("offsets: {:?}", offsets);
	// println!("layer: {}", layer);
	
	let mut corner_heights = std::array::from_fn(|c| {
		let right_left: [_; 2] = std::array::from_fn(|rl| {
			CORNER_SIDES[c][rl]
				.into_iter()
				.find_map(|side| {
					layer.checked_sub(offsets[side]).and_then(|face_index| sides[side].get(face_index))
				})
				.map(|face| face.heights[rl][top_bottom])
		});
		match right_left {
			[Some(right), Some(left)] => Some([left, right][(left > right) as usize ^ top_bottom]),//min if top, max if bottom
			[Some(height), None] => Some(height),
			[None, Some(height)] => Some(height),
			_ => None,
		}
	});
	assert!(corner_heights != [None; 4]);
	for corner in 0..4 {
		match [
			corner_heights[(corner + 3) % 4],
			corner_heights[corner],
			corner_heights[(corner + 1) % 4],
		] {
			[None, None, None] => corner_heights[corner] = corner_heights[(corner + 2) % 4],
			[a, None, None] | [None, None, a] => corner_heights[corner] = a,
			[Some(a), None, Some(b)] => corner_heights[corner] = Some((a + b) / 2),
			_ => {},
		}
	}
	corner_heights.map(|h| h.unwrap())
}

fn get_offsets(sides: &[&[WallFace]; 8]) -> [usize; 8] {
	//first, determine deltas by polygon side-sharing
	let mut deltas = SIDE_MEETINGS.map(|right_left| {
		let [right, left] = right_left.map(|i| sides[i]);
		find_connection(left, right)
	});
	//rings with a single hole can be patched unambiguously
	//stitch together wall sides that don't share a poly side with "find_closest"
	'outer: loop {
		'rings: for ring in CORNER_RINGS {
			let mut missing = None;
			let mut sum = 0;
			for &index in ring {
				match deltas[index] {
					Some(d) => sum -= d,
					None => match missing {
						Some(_) => continue 'rings,
						None => missing = Some(index),
					},
				}
			}
			if let Some(missing) = missing {
				deltas[missing] = Some(sum);
				continue 'outer;
			}
		}
		if let Some((i, d, _)) = {
			(0..12).filter(|&i| deltas[i].is_none()).filter_map(|i| {
				let [right, left] = SIDE_MEETINGS[i].map(|s| sides[s]);
				find_closest(left, right).map(|(d, wd)| (i, d, wd))
			}).min_by_key(|&(.., wd)| wd)
		} {
			deltas[i] = Some(d);
			continue;
		}
		break;
	}
	//at this point, "None" deltas must be missing walls
	//cover disjointed walls (||, =) case:
	'gaps: for o in 0..2 {
		for i in 0..4 {
			if sides[(i + o) % 4].is_empty() ^ (i % 2 != 0) {
				continue 'gaps;
			}
		}
		//get closest both ways and pick the better match
		let (Some((d1, wd1)), Some((d2, wd2))) = (
			find_closest(sides[o + 2], sides[o]),
			find_closest(sides[o], sides[o + 2]),
		) else {
			unreachable!();
		};
		let d = if wd1 < wd2 {
			d1
		} else {
			-d2
		};
		let half1 = d / 2;
		let half2 = d - half1;
		deltas[o] = Some(half1);
		deltas[o + 1] = Some(half2);
		deltas[o + 2] = Some(-half1);
		deltas[(o + 3) % 4] = Some(-half2);
		break;
	}
	//fill gaps, must be contiguous at this point
	for ring in CORNER_RINGS {
		let mut sum = 0;
		let mut missing = 0;
		for &index in ring {
			match deltas[index] {
				Some(d) => sum -= d,
				None => missing += 1,
			}
		}
		for &index in ring {
			if let None = deltas[index] {
				let p = sum / missing;
				sum -= p;
				missing -= 1;
				deltas[index] = Some(p);
			}
		}
	}
	let deltas = deltas.map(|d| d.unwrap());
	assert!(CORNER_RINGS.iter().all(|ring| ring.iter().map(|&i| deltas[i]).sum::<isize>() == 0));
	let mut offsets = [None; 8];
	offsets[0] = Some(0);
	for i in 0..12 {
		let [right, left] = SIDE_MEETINGS[i];
		if let (None, Some(o)) = (offsets[left], offsets[right]) {
			offsets[left] = Some(o - deltas[i]);
		}
	}
	let offsets = offsets.map(|o| o.unwrap());
	let om = offsets.into_iter().min().unwrap();
	offsets.map(|o| usize::try_from(o - om).unwrap())
}

const TOP: usize = 0;
const BOTTOM: usize = 1;

/// num_divs must be at least 2
fn add_divs(divs: &mut Vec<[i16; 4]>, sides: &[&[WallFace]; 8], offsets: &[usize; 8], num_divs: usize) {
	divs.push(get_corner_heights(&sides, &offsets, 0, TOP));
	for layer in 0..num_divs - 1 {
		divs.push(get_corner_heights(&sides, &offsets, layer, BOTTOM));
	}
}

fn get_split_divs(sides: &[&[WallFace]; 8], num_ceiling: &[usize; 8]) -> (Vec<[i16; 4]>, usize) {
	let ceiling_floor = [
		std::array::from_fn(|s| &sides[s][..num_ceiling[s]]),
		std::array::from_fn(|s| &sides[s][num_ceiling[s]..]),
	];
	let ceiling_floor_offsets = ceiling_floor.map(|cf| get_offsets(&cf));
	let ceiling_floor_divs: [_; 2] = std::array::from_fn(|cf| {
		match (0..8).map(|s| ceiling_floor_offsets[cf][s] + ceiling_floor[cf][s].len()).max().unwrap() {
			0 => 0,
			divs => divs + 1,//if there is a face, add 1 for top
		}
	});
	let mut divs = Vec::with_capacity(ceiling_floor_divs[0] + ceiling_floor_divs[1]);
	for cf in 0..2 {
		if ceiling_floor_divs[cf] >= 2 {// add_divs requires at least 2 divs
			add_divs(&mut divs, &ceiling_floor[cf], &ceiling_floor_offsets[cf], ceiling_floor_divs[cf]);
		}
	}
	(divs, ceiling_floor_divs[0])
}

fn get_divisions(floor_ceiling: &[Option<SectorSurface>; 2], sides: &[&[WallFace]; 8]) -> (Vec<[i16; 4]>, usize) {
	const CORNERS_BY_SIDE: [[usize; 2]; 8] = [
		[0, 3],
		[1, 0],
		[2, 1],
		[3, 2],
		[1, 3],
		[2, 0],
		[3, 1],
		[0, 2],
	];
	if sides.iter().all(|side| side.is_empty()) {
		return (vec![], 0);
	}
	match floor_ceiling {
		[None, None] => {
			let offsets = get_offsets(&sides);
			//must be at least 1 face, add 1 for top, guaranteed 2 minimum
			let num_divs = (0..8).map(|s| offsets[s] + sides[s].len()).max().unwrap() + 1;
			let mut divs = Vec::with_capacity(num_divs);
			add_divs(&mut divs, sides, &offsets, num_divs);
			let num_ceiling = num_divs / 2;
			(divs, num_ceiling)
		},
		[Some(floor), None] => {
			let num_ceiling: [_; 8] = std::array::from_fn(|s| {
				let floor_heights_lr = CORNERS_BY_SIDE[s].map(|c| floor.corner_heights[c]);
				let side = sides[s];
				side.iter().position(|face| {
					(0..2).all(|lr| face.heights[lr].iter().all(|&h| h >= floor_heights_lr[lr]))
				}).unwrap_or(side.len())
			});
			get_split_divs(sides, &num_ceiling)
		},
		[_, Some(ceiling)] => {
			let num_ceiling: [_; 8] = std::array::from_fn(|s| {
				let ceiling_heights_lr = CORNERS_BY_SIDE[s].map(|c| ceiling.corner_heights[c]);
				let side = sides[s];
				side.iter().position(|face| {
					(0..2).any(|lr| face.heights[lr].iter().any(|&h| h >= ceiling_heights_lr[lr]))
				}).unwrap_or(side.len())
			});
			get_split_divs(sides, &num_ceiling)
		},
	}
	// let mut textures = vec![];
	// match (floor, ceiling) {
	// 	(None, None) => {
	// 		if walls.iter().all(|wall| wall.is_empty()) {
	// 			WallsResult::default()//either an inner wall (walls on all 4 sides) or empty space
	// 		} else {
	// 			let side0_index = (0..4).max_by_key(|&i| walls[i].len()).unwrap();
	// 			let side1_index = (side0_index + 1) % 4;//left (clockwise)
	// 			let side2_index = (side0_index + 2) % 4;//opposite
	// 			let side3_index = (side0_index + 3) % 4;//right (counter-clockwise)
	// 			let side0 = &walls[side0_index];
	// 			let side1 = &walls[side1_index];
	// 			let side2 = &walls[side2_index];
	// 			let side3 = &walls[side3_index];
	// 			//first, try to determine offsets by connections to side0
	// 			let mut side1_offset = find_connection(side1, side0, CW, 0);
	// 			let mut side2_offset = None;
	// 			let mut side3_offset = find_connection(side0, side3, CCW, 0);
	// 			if let Some(side1_offset) = side1_offset {
	// 				side2_offset = find_connection(side2, side1, CW, side1_offset);
	// 			}
	// 			if let (None, Some(side3_offset)) = (side2_offset, side3_offset) {
	// 				side2_offset = find_connection(side3, side2, CCW, side3_offset);
	// 			}
	// 			if let (None, Some(side2_offset)) = (side1_offset, side2_offset) {
	// 				side1_offset = find_connection(side2, side1, CCW, side2_offset);
	// 			}
	// 			if let (None, Some(side2_offset)) = (side3_offset, side2_offset) {
	// 				side3_offset = find_connection(side3, side2, CW, side2_offset);
	// 			}
	// 			//check for connections between adjacent sides and opposite side, if no side0 connection
	// 			let mut side1_from2 = None;
	// 			let mut side3_from2 = None;
	// 			if let (None, None) = (side1_offset, side2_offset) {
	// 				side1_from2 = find_connection(side2, side1, CCW, 0);
	// 			}
	// 			if let (None, None) = (side3_offset, side2_offset) {
	// 				side3_from2 = find_connection(side3, side2, CW, 0);
	// 			}
	// 			//"glue" connected sections with a single `find_closest` using the closer of the two
	// 			/*
	// 			handles these cases (# = connected):
	// 			#1  |  1# |  1#
	// 			0 2 | 0 2 | 0 2
	// 			 3# | #3  |  3#
	// 			case handled below:
	// 			#1
	// 			0 2
	// 			#3
	// 			*/
	// 			//oc: offset candidate
	// 			match (side1_from2, side3_from2, side1_offset, side3_offset) {
	// 				(Some(side1_from2), Some(side3_from2), ..) => {
	// 					let (Some((side1_oc, side1_dist)), Some((side3_oc, side3_dist))) = (find_closest(side1, side0, CW), find_closest(side0, side3, CCW)) else {
	// 						unreachable!();
	// 					};
	// 					if side1_dist < side3_dist {
	// 						side1_offset = Some(side1_oc);
	// 						side2_offset = Some(side1_oc - side1_from2);
	// 						side3_offset = Some(side1_oc - side1_from2 + side3_from2);
	// 					} else {
	// 						side3_offset = Some(side3_oc);
	// 						side2_offset = Some(side3_oc - side3_from2);
	// 						side1_offset = Some(side3_oc - side3_from2 + side1_from2);
	// 					}
	// 				},
	// 				(Some(side1_from2), None, _, Some(side3_offset)) => {
	// 					let (Some((side1_oc, side1_dist)), Some((side2_from3, side2_dist))) = (find_closest(side1, side0, CW), find_closest(side3, side2, CCW)) else {
	// 						unreachable!();
	// 					};
	// 					if side1_dist < side2_dist {
	// 						side1_offset = Some(side1_oc);
	// 						side2_offset = Some(side1_oc - side1_from2);
	// 					} else {
	// 						side2_offset = Some(side3_offset + side2_from3);
	// 						side1_offset = Some(side3_offset + side2_from3 + side1_from2);
	// 					}
	// 				},
	// 				(None, Some(side3_from2), Some(side1_offset), _) => {
	// 					let (Some((side2_from1, side2_dist)), Some((side3_oc, side3_dist))) = (find_closest(side2, side1, CW), find_closest(side0, side3, CCW)) else {
	// 						unreachable!();
	// 					};
	// 					if side2_dist < side3_dist {
	// 						side2_offset = Some(side1_offset + side2_from1);
	// 						side3_offset = Some(side1_offset + side2_from1 + side3_from2);
	// 					} else {
	// 						side3_offset = Some(side3_oc);
	// 						side2_offset = Some(side3_oc - side3_from2);
	// 					}
	// 				},
	// 				_ => {},
	// 			}
	// 			if let (Some(side1_offset), None, Some(side3_offset)) = (side1_offset, side2_offset, side2_offset) {
	// 				if let (Some((side2_from1, side1_dist)), Some((side2_from3, side3_dist))) = (find_closest(side2, side1, CW), find_closest(side3, side2, CCW)) {
	// 					if side1_dist < side3_dist {
	// 						side2_offset = Some(side1_offset + side2_from1);
	// 					} else {
	// 						side2_offset = Some(side3_offset + side2_from3);
	// 					}
	// 				};
	// 			}
	// 			//at this point, any unknown offsets will be guessed using find_closest
	// 			if let None = side1_offset {
	// 				side1_offset = find_closest(side1, side0, CW).map(|(o, _)| o);
	// 			}
	// 			if let None = side3_offset {
	// 				side3_offset = find_closest(side0, side3, CCW).map(|(o, _)| o);
	// 			}
	// 			match (side2_offset, side1_offset, side3_offset) {
	// 				(None, Some(side1_offset), Some(side3_offset)) => {
	// 					if let (Some((side2_from1, side1_dist)), Some((side2_from3, side3_dist))) = (find_closest(side2, side1, CW), find_closest(side3, side2, CCW)) {
	// 						if side1_dist < side3_dist {
	// 							side2_offset = Some(side1_offset + side2_from1);
	// 						} else {
	// 							side2_offset = Some(side3_offset + side2_from3);
	// 						}
	// 					}
	// 				},
	// 				(None, Some(side1_offset), None) => {
	// 					side2_offset = find_closest(side2, side1, CW).map(|(o, _)| o + side1_offset);
	// 				},
	// 				(None, None, Some(side3_offset)) => {
	// 					side2_offset = find_closest(side3, side2, CCW).map(|(o, _)| o + side3_offset);
	// 				},
	// 				_ => {},
	// 			}
	// 			//at this point, `None` offsets must be empty walls, so the value doesn't matter
	// 			let [side1_offset, side2_offset, side3_offset] = [side1_offset, side2_offset, side3_offset].map(|o| o.unwrap_or_default());
	// 			let mut offsets = [0, side1_offset, side2_offset, side3_offset];
	// 			offsets.rotate_right(side0_index);
	// 			let offset_minimum = offsets.into_iter().min().unwrap();
	// 			let offsets: [usize; 4] = offsets.map(|o| (o - offset_minimum).try_into().unwrap());//TODO: change to `as` once confident
	// 			/*
	// 			ABCD are wall sides
	// 			0|A
	// 			1|AB D
	// 			2| BCD
	// 			3| B
	// 			offsets:
	// 			A: 0
	// 			B: 1
	// 			C: 2
	// 			D: 1
	// 			*/
				// let num_divisions = (0..4).map(|i| offsets[i] + walls[i].len()).max().unwrap() + 1;
				// let mut divisions = Vec::with_capacity(num_divisions);
				// divisions.push(get_corner_heights(walls, &offsets, 0, TOP));
				// for layer in 0..num_divisions - 1 {
				// 	divisions.push(get_corner_heights(walls, &offsets, layer, BOTTOM));
				// }
	// 			WallsResult {
	// 				wall: Some(true),
	// 				divisions,
	// 			}
	// 		}
	// 	},
	// 	(Some(floor), None) => {
	// 		WallsResult {
	// 			wall: Some(false),
	// 			divisions: vec![],
	// 		}
	// 	},
	// 	(None, Some(ceiling)) => {
	// 		WallsResult {
	// 			wall: Some(false),
	// 			divisions: vec![],
	// 		}
	// 	},
	// 	(Some(floor), Some(ceiling)) => {
	// 		WallsResult {
	// 			wall: Some(false),
	// 			divisions: vec![],
	// 		}
	// 	},
	// }
}

pub fn get(
	snapped_quads: &[SnappedFace<4>], snapped_tris: &[SnappedFace<3>], sec_x: i16, sec_z: i16,
	floor_ceiling: &[Option<SectorSurface>; 2],
) -> (Vec<[i16; 4]>, usize) {
	let sides = get_sides(snapped_quads, snapped_tris, sec_x, sec_z);
	let sides_ref = sides.each_ref().map(|s| &s[..]);
	get_divisions(floor_ceiling, &sides_ref)
}
