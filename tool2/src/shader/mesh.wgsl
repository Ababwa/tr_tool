/*
mapping:
struct ObjectTexture {
	blend_mode: u16,
	atlas_index: u16,
	uvs: [U16Vec2; 4],
}
struct SpriteTexture {
	atlas_index: u16,
	pos: U8Vec2,
	size: U16Vec2,
	world_bounds: [I16Vec2; 2],
}
struct RoomVertex {
	x: i16,
	y: i16,
	z: i16,
	light: u16,
}
struct MeshVertex {
	x: i16,
	y: i16,
	z: i16,
}
struct VertexArray<Vertex>
where Vertex: RoomVertex | MeshVertex {
	vertex_size: u16,
	vertices: [Vertex],
}
struct Quad {
	vertex_indices: [u16; 4],
	texture_index: u16,
}
struct Tri {
	vert_indices: [u16; 3],
	texture_index: u16,
}
struct FaceArray<Face>
where Face: Quad | Tri {
	vertex_array_offset: [u16; 2],//unaligned u32
	face_size: u16,
	faces: [Face],
}
struct {
	geometry: [VertexArray | FaceArray],
	object_textures: [ObjectTexture],
	transforms: [Mat4],
	sprite_textures: [SpriteTexture],
	face_array_offsets: [u32],
}
notes:
if textured, texture_index is an index into the ObjectTexture array
if solid, texture_index is an index into palette
*/
@group(0) @binding(0) var<storage> data: array<vec4u, 131072>;

//byte offsets into data
const OBJECT_TEXTURES_OFFSET: u32 = 1048576;
const TRANSFORMS_OFFSET: u32 = 1310720;
const SPRITE_TEXTURES_OFFSET: u32 = 1572864;
const FACE_ARRAY_MAP_OFFSET: u32 = 1835008;

@group(0) @binding(1) var<uniform> camera_transform: mat4x4f;
@group(0) @binding(2) var<uniform> perspective_transform: mat4x4f;

/*
offset:
VVVVVVVVVVVVVVVVVVVVVVVVVVVVVVEE
V: vec4u in data
E: u32 in vec4u
*/
fn get_data_u32(offset: u32) -> u32 {
	return data[offset / 4][offset % 4];
}

/*
offset:
VVVVVVVVVVVVVVVVVVVVVVVVVVVVVEES
V: vec4u in data
E: u32 in vec4u
S: u16 in u32
*/
fn get_data_u16(offset: u32) -> u32 {
	return (get_data_u32(offset / 2) >> ((offset % 2) * 16)) & 0xFFFF;
}

struct PositionTexture {
	position: vec4f,
	texture_index: u32,
	object_id: u32,
}

fn get_position_texture(face: vec2u, face_vertex_index: u32) -> PositionTexture {
	//unpack face instance
	let face_array_index = face.x & 0xFFFF;
	let face_index = face.x >> 16;
	let transform_index = face.y & 0xFFFF;
	let object_id = face.y >> 16;
	//transform
	let transform_offset_v4 = (TRANSFORMS_OFFSET / 64 + transform_index) * 4;
	let local_transform = mat4x4f(
		bitcast<vec4f>(data[transform_offset_v4]),
		bitcast<vec4f>(data[transform_offset_v4 + 1]),
		bitcast<vec4f>(data[transform_offset_v4 + 2]),
		bitcast<vec4f>(data[transform_offset_v4 + 3]),
	);
	//position
	let face_array_offset = get_data_u32(FACE_ARRAY_MAP_OFFSET / 4 + face_array_index);
	let vertex_array_offset = get_data_u16(face_array_offset) * 8;
	let vertex_size = get_data_u16(vertex_array_offset);
	let face_info_packed = get_data_u16(face_array_offset + 1);
	let face_size = face_info_packed & 0xFF;
	let face_texture_index_offset = face_info_packed >> 8;
	let face_offset = face_array_offset + 2 + (face_index * face_size);
	let vertex_index = get_data_u16(face_offset + face_vertex_index);
	let vertex_offset = vertex_array_offset + 1 + (vertex_index * vertex_size);
	let vertex_unsigned = vec3u(
		get_data_u16(vertex_offset),
		get_data_u16(vertex_offset + 1),
		get_data_u16(vertex_offset + 2),
	);
	let vertex_relative = vec3i(vertex_unsigned << vec3u(16)) >> vec3u(16);//interpret lower 16 as i16
	let vertex_absolute = local_transform * vec4f(vec3f(vertex_relative), 1.0);
	let position = perspective_transform * camera_transform * vertex_absolute;
	//texture
	let texture_index = get_data_u16(face_offset + face_texture_index_offset);
	return PositionTexture(position, texture_index, object_id);
}

struct TextureVTF {
	@builtin(position) position: vec4f,
	@location(0) atlas_index: u32,
	@location(1) uv: vec2f,
	@location(2) object_id: u32,
}

@vertex
fn texture_vs_main(
	@location(0) face_vertex_index: u32,//vertex
	@location(1) face: vec2u,//instance
) -> TextureVTF {
	let position_texture = get_position_texture(face, face_vertex_index);
	let position = position_texture.position;
	let object_texture_index = position_texture.texture_index & 0x7FFF;
	let object_id = position_texture.object_id;
	let object_textures_start = OBJECT_TEXTURES_OFFSET / 2;
	let object_texture_size = get_data_u16(object_textures_start);
	let object_texture_offset = object_textures_start + 1 + object_texture_index * object_texture_size;
	// let blend_mode = get_data_u16(object_texture_offset);
	let atlas_index = get_data_u16(object_texture_offset + 1) & 0x7FFF;
	let uv_offset = (object_texture_offset + 2 + (object_texture_size % 2)) + face_vertex_index * 2;
	let uv_subpixel = vec2u(
		get_data_u16(uv_offset),
		get_data_u16(uv_offset + 1),
	);
	let uv = vec2f((uv_subpixel + 128) / 256);//round to nearest whole pixel
	return TextureVTF(position, atlas_index, uv, object_id);
}

struct SolidVTF {
	@builtin(position) position: vec4f,
	@location(0) color_index: u32,
	@location(1) object_id: u32,
}

fn solid_vs(
	face_vertex_index: u32,
	face: vec2u,
	mode: u32,//0: use 24-bit palette index, 1: use 32-bit palette index
) -> SolidVTF {
	let position_texture = get_position_texture(face, face_vertex_index);
	let position = position_texture.position;
	let color_index = (position_texture.texture_index >> (mode * 8)) & 0xFF;
	let object_id = position_texture.object_id;
	return SolidVTF(position, color_index, object_id);
}

@vertex
fn solid_24bit_vs_main(
	@location(0) face_vertex_index: u32,//vertex
	@location(1) face: vec2u,//instance
) -> SolidVTF {
	return solid_vs(face_vertex_index, face, 0u);
}

@vertex
fn solid_32bit_vs_main(
	@location(0) face_vertex_index: u32,//vertex
	@location(1) face: vec2u,//instance
) -> SolidVTF {
	return solid_vs(face_vertex_index, face, 1u);
}

@vertex
fn sprite_vs_main(
	@location(0) face_vertex_index: u32,//vertex
	@location(1) sprite: vec4i,//instance
) -> TextureVTF {
	let uv_index = vec2u(((face_vertex_index + 1) / 2) % 2, face_vertex_index / 2);
	let sprite_texture_index = u32(sprite.w) & 0xFFFF;
	let object_id = u32(sprite.w) >> 16;
	let sprite_texture_offset = SPRITE_TEXTURES_OFFSET / 2 + sprite_texture_index * 8;//8: size of SpriteTexture in u16s
	let atlas_index = get_data_u16(sprite_texture_offset);
	let sprite_pos_packed = get_data_u16(sprite_texture_offset + 1);
	let sprite_pos = vec2u(sprite_pos_packed & 0xFF, sprite_pos_packed >> 8);
	let sprite_size_subpixel = vec2u(
		get_data_u16(sprite_texture_offset + 2),
		get_data_u16(sprite_texture_offset + 3),
	);
	let sprite_size = sprite_size_subpixel / 256;
	let world_offset_unsigned = vec2u(
		get_data_u16(sprite_texture_offset + 4 + uv_index.x * 2),
		get_data_u16(sprite_texture_offset + 5 + uv_index.y * 2),
	);
	let world_offset_int = vec2i(world_offset_unsigned << vec2u(16)) >> vec2u(16);//interpret lower 16 as i16
	let world_offset = vec2f(world_offset_int);
	let vertex = vec4f(vec3f(sprite.xyz), 1.0);
	var position_camera = camera_transform * vertex;
	position_camera.x += world_offset.x;
	position_camera.y -= world_offset.y - 24;//24: move sprites up to prevent ground clipping
	let position = perspective_transform * position_camera;
	let uv_int = sprite_pos + sprite_size * uv_index;
	let uv = vec2f(uv_int);
	return TextureVTF(position, atlas_index, uv, object_id);
}

struct Out {
	@location(0) color: vec4f,
	@location(1) interact: u32,
}

@group(0) @binding(3) var atlases: texture_2d_array<u32>;

//each texel is a color channel
@group(0) @binding(4) var palette: texture_1d<u32>;

fn to_f32_color(r: u32, g: u32, b: u32, divisor: f32) -> vec4f {
	let color_int = vec3u(r, g, b);
	let color_f = vec3f(color_int);
	let color_scaled = color_f / divisor;
	let color_rgba = vec4f(color_scaled, 1.0);
	return color_rgba;
}

fn get_palette_color(color_index: u32, color_size: u32, divisor: f32) -> vec4f {
	let offset = color_index * color_size;
	return to_f32_color(
		textureLoad(palette, offset, 0).x,
		textureLoad(palette, offset + 1, 0).x,
		textureLoad(palette, offset + 2, 0).x,
		divisor,
	);
}

@fragment
fn solid_24bit_fs_main(vtf: SolidVTF) -> Out {
	let color = get_palette_color(vtf.color_index, 3u, 63.0);
	return Out(color, vtf.object_id);
}

@fragment
fn solid_32bit_fs_main(vtf: SolidVTF) -> Out {
	let color = get_palette_color(vtf.color_index, 4u, 255.0);
	return Out(color, vtf.object_id);
}

@fragment
fn texture_palette_fs_main(vtf: TextureVTF) -> Out {
	let color_index = textureLoad(atlases, vec2i(vtf.uv), vtf.atlas_index, 0).x;
	if color_index == 0 {
		discard;
	} else {
		return Out(get_palette_color(color_index, 3u, 63.0), vtf.object_id);
	}
}

@fragment
fn texture_16bit_fs_main(vtf: TextureVTF) -> Out {
	let color = textureLoad(atlases, vec2i(vtf.uv), vtf.atlas_index, 0).x;
	if (color & 0x8000) == 0 {
		discard;
	} else {
		let color = to_f32_color(
			(color >> 10) & 0x1F,
			(color >> 5) & 0x1F,
			color & 0x1F,
			31.0,
		);
		return Out(color, vtf.object_id);
	}
}

@fragment
fn texture_32bit_fs_main(vtf: TextureVTF) -> Out {
	let color = textureLoad(atlases, vec2i(vtf.uv), vtf.atlas_index, 0).x;
	if (color & 0xFF00FF) == 0xFF00FF {
		discard;
	} else {
		let color = to_f32_color(
			(color >> 16) & 0xFF,
			(color >> 8) & 0xFF,
			color & 0xFF,
			255.0,
		);
		return Out(color, vtf.object_id);
	}
}
