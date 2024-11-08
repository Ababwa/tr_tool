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
@group(0) @binding(0) var<storage> data: array<vec4u, 65536>;

//byte offsets into data
const OBJECT_TEXTURES_OFFSET: u32 = 524288;
const TRANSFORMS_OFFSET: u32 = 655360;
const SPRITE_TEXTURES_OFFSET: u32 = 786432;
const FACE_ARRAY_MAP_OFFSET: u32 = 917504;

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
	return (get_data_u32(offset / 2) >> ((offset % 2) * 16)) & 65535;
}

struct PositionTexture {
	position: vec4f,
	texture_index: u32,
}

fn get_position_texture(face: vec2u, face_vertex_index: u32) -> PositionTexture {
	//unpack face instance
	let face_array_index = face.x & 65535;
	let face_index = face.x >> 16;
	let transform_index = face.y & 65535;
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
	let vertex_array_offset_lower = get_data_u16(face_array_offset);
	let vertex_array_offset_upper = get_data_u16(face_array_offset + 1);
	let vertex_array_offset = vertex_array_offset_lower | (vertex_array_offset_upper << 16);
	let vertex_size = get_data_u16(vertex_array_offset);
	let face_size = get_data_u16(face_array_offset + 2);
	let face_offset = face_array_offset + 3 + (face_index * face_size);
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
	let texture_index = get_data_u16(face_offset + face_size - 1);
	return PositionTexture(position, texture_index);
}

struct TexturedVTF {
	@builtin(position) position: vec4f,
	@location(0) blend_mode: u32,
	@location(1) atlas_index: u32,
	@location(2) uv: vec2f,
}

@vertex
fn textured_vs_main(
	@location(0) face_vertex_index: u32,//vertex
	@location(1) face: vec2u,//instance
) -> TexturedVTF {
	let position_texture = get_position_texture(face, face_vertex_index);
	let position = position_texture.position;
	let object_texture_index = position_texture.texture_index;
	let object_texture_offset = OBJECT_TEXTURES_OFFSET / 2 + object_texture_index * 10;//10: size of ObjectTexture in u16s
	let blend_mode = get_data_u16(object_texture_offset);
	let atlas_index = get_data_u16(object_texture_offset + 1);
	let uv_offset = (object_texture_offset + 2) + face_vertex_index * 2;
	let uv_subpixel = vec2u(
		get_data_u16(uv_offset),
		get_data_u16(uv_offset + 1),
	);
	let uv = vec2f((uv_subpixel + 128) / 256);//round to nearest whole pixel
	return TexturedVTF(position, blend_mode, atlas_index, uv);
}

struct SolidVTF {
	@builtin(position) position: vec4f,
	@location(0) color_index: u32,
}

@vertex
fn solid_vs_main(
	@location(0) face_vertex_index: u32,//vertex
	@location(1) face: vec2u,//instance
) -> SolidVTF {
	let position_texture = get_position_texture(face, face_vertex_index);
	let position = position_texture.position;
	let color_index = position_texture.texture_index;
	return SolidVTF(position, color_index);
}

struct SpriteVTF {
	@builtin(position) position: vec4f,
	@location(0) atlas_index: u32,
	@location(1) uv: vec2f,
}

@vertex
fn sprite_vs_main(
	@location(0) face_vertex_index: u32,//vertex
	@location(1) sprite: vec4i,//instance
) -> SpriteVTF {
	let uv_index = vec2u(((face_vertex_index + 1) / 2) % 2, face_vertex_index / 2);
	let sprite_texture_index = u32(sprite.w);
	let sprite_texture_offset = SPRITE_TEXTURES_OFFSET / 2 + sprite_texture_index * 8;//8: size of SpriteTexture in u16s
	let atlas_index = get_data_u16(sprite_texture_offset);
	let sprite_pos_packed = get_data_u16(sprite_texture_offset + 1);
	let sprite_pos = vec2u(sprite_pos_packed & 255, sprite_pos_packed >> 8);
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
	position_camera.y -= world_offset.y - 28;//28: move sprites up to prevent ground clipping
	let position = perspective_transform * position_camera;
	let uv_int = sprite_pos + sprite_size * uv_index;
	let uv = vec2f(uv_int);
	return SpriteVTF(position, atlas_index, uv);
}

/*
mapping:
struct Color {
	r: u8,
	g: u8,
	b: u8,
}
[Color; 256]
notes:
6-bit color
*/
@group(0) @binding(3) var<uniform> palette: array<vec4u, 48>;

/*
offset:
VVVVVVVVVVVVVVVVVVVVVVVVVVVVEEBB
V: vec4u in palette
E: u32 in vec4u
B: u8 in u32
*/
fn get_color_u8(offset: u32) -> u32 {
	return (palette[offset / 16][(offset / 4) % 4] >> ((offset % 4) * 8)) & 255;
}

fn get_color(color_index: u32) -> vec4f {
	let offset = color_index * 3;
	let color_rgb_6bit = vec3u(
		get_color_u8(offset),
		get_color_u8(offset + 1),
		get_color_u8(offset + 2),
	);
	let color_rgb = vec3f(color_rgb_6bit) / 63.0;
	let color_rgba = vec4f(color_rgb, 1.0);
	return color_rgba;
}

@group(0) @binding(4) var atlases: texture_2d_array<u32>;

const BLEND_MODE_TEST: u32 = 1;

@fragment
fn textured_fs_main(vtf: TexturedVTF) -> @location(0) vec4f {
	let color_index = textureLoad(atlases, vec2i(vtf.uv), vtf.atlas_index, 0).x;
	if vtf.blend_mode == BLEND_MODE_TEST && color_index == 0 {
		discard;
	} else {
		return get_color(color_index);
	}
}

@fragment
fn solid_fs_main(vtf: SolidVTF) -> @location(0) vec4f {
	return get_color(vtf.color_index);
}

@fragment
fn sprite_fs_main(vtf: SpriteVTF) -> @location(0) vec4f {
	let color_index = textureLoad(atlases, vec2i(vtf.uv), vtf.atlas_index, 0).x;
	if color_index == 0 {
		discard;
	} else {
		return get_color(color_index);
	}
}
