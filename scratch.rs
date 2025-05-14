// enum WallFaceShape {
// 	Quad {
// 		left_top: i16,
// 		left_bottom: i16,
// 		right_top: i16,
// 		right_bottom: i16,
// 	},
// 	/// Tri hugs left edge
// 	TriLeft {
// 		left_top: i16,
// 		left_bottom: i16,
// 		right: i16,
// 	},
// 	/// Tri hugs right edge
// 	TriRight {
// 		left: i16,
// 		right_top: i16,
// 		right_bottom: i16,
// 	},
// }

// impl WallFaceShape {
// 	fn height(&self) -> i16 {
// 		match self {
// 			WallFaceShape::Quad { left_top, left_bottom, right_top, right_bottom } => {
// 				left_top + left_bottom + right_top + right_bottom
// 			},
// 			WallFaceShape::TriLeft { left_top, left_bottom, right } => {
// 				left_top + left_bottom + right * 2
// 			},
// 			WallFaceShape::TriRight { left, right_top, right_bottom } => {
// 				left * 2 + right_top + right_bottom
// 			},
// 		}
// 	}
	
// 	fn left(&self) -> WallFaceEdge {
// 		match self {
// 			&WallFaceShape::Quad { left_top, left_bottom, .. } |
// 			&WallFaceShape::TriLeft { left_top, left_bottom, .. } => {
// 				WallFaceEdge::Two { top: left_top, bottom: left_bottom }
// 			},
// 			&WallFaceShape::TriRight { left, .. } => WallFaceEdge::One(left),
// 		}
// 	}
	
// 	fn right(&self) -> WallFaceEdge {
// 		match self {
// 			&WallFaceShape::Quad { right_top, right_bottom, .. } |
// 			&WallFaceShape::TriRight { right_top, right_bottom, .. } => {
// 				WallFaceEdge::Two { top: right_top, bottom: right_bottom }
// 			},
// 			&WallFaceShape::TriLeft { right, .. } => WallFaceEdge::One(right),
// 		}
// 	}
	
// 	fn top_bottom(&self) -> WallFaceTopBottom {
// 		match self {
// 			&WallFaceShape::Quad { left_top, left_bottom, right_top, right_bottom } => {
// 				WallFaceTopBottom { left_top, left_bottom, right_top, right_bottom }
// 			},
// 			&WallFaceShape::TriLeft { left_top, left_bottom, right } => {
// 				WallFaceTopBottom { left_top, left_bottom, right_top: right, right_bottom: right }
// 			},
// 			&WallFaceShape::TriRight { left, right_top, right_bottom } => {
// 				WallFaceTopBottom { left_top: left, left_bottom: left, right_top, right_bottom }
// 			},
// 		}
// 	}
// }

// fn add_textures(textures: &mut Vec<TextureToAdd>, walls: &[Vec<WallFace>; 4], side: usize, face_index: usize, level: i8) {
// 	for i in 0..walls[side].len() {
// 		textures.push(TextureToAdd {
// 			sector_face: SectorFace::from_side_level(side, 0),
// 			object_texture_index: walls[side][i].object_texture_index,
// 		});
// 	}
	
// 	for ceiling_offset in 1..=index {
// 		let level = -(ceiling_offset as i8);
// 		let object_texture_index = walls[side][index - ceiling_offset].object_texture_index;
// 		textures.push(TextureToAdd {
// 			sector_face: SectorFace::from_side_level(side, level),
// 			object_texture_index,
// 		});
// 	}
// 	for floor_offset in 1..(walls[side].len() - index) {
// 		let level = floor_offset as i8;
// 		let object_texture_index = walls[side][index + floor_offset].object_texture_index;
// 		textures.push(TextureToAdd {
// 			sector_face: SectorFace::from_side_level(side, level),
// 			object_texture_index,
// 		});
// 	}
// }
