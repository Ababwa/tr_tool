struct DataOffsets {
	transforms_offset: u32,//16-byte units
	face_array_offsets_offset: u32,//4-byte units
	object_textures_offset: u32,//2-byte units
	object_texture_size: u32,//2-byte units
	sprite_textures_offset: u32,//2-byte units
	num_atlases: u32,
}

//2MB
@group(0) @binding(0) var<storage> data: array<vec4u, 131072>;
@group(0) @binding(1) var<uniform> data_offsets: DataOffsets;
@group(0) @binding(2) var<uniform> camera_transform: mat4x4f;
@group(0) @binding(3) var<uniform> perspective_transform: mat4x4f;

fn get_data_u32(offset: u32) -> u32 {
	return data[offset / 4][offset % 4];
}

fn get_data_u16(offset: u32) -> u32 {
	return (get_data_u32(offset / 2) >> ((offset % 2) * 16)) & 0xFFFF;
}

struct PositionTexture {
	position: vec4f,
	texture_index: u32,
	object_id: u32,
}

fn get_position_texture(face: vec3u, face_vertex_index: u32) -> PositionTexture {
	//unpack face instance
	let face_array_index = face.x & 0xFFFF;
	let face_index = face.x >> 16;
	let transform_index = face.y & 0xFFFF;
	let object_id = face.z;
	//transform
	let transform_offset = data_offsets.transforms_offset + transform_index * 4;
	let local_transform = mat4x4f(
		bitcast<vec4f>(data[transform_offset]),
		bitcast<vec4f>(data[transform_offset + 1]),
		bitcast<vec4f>(data[transform_offset + 2]),
		bitcast<vec4f>(data[transform_offset + 3]),
	);
	//position
	let face_array_offset = get_data_u32(data_offsets.face_array_offsets_offset + face_array_index);//4-byte units
	let vertex_array_offset = get_data_u32(face_array_offset);//4-byte units
	let vertex_size = get_data_u32(vertex_array_offset);//2-byte units
	let face_info_packed = get_data_u32(face_array_offset + 1);
	let face_size = face_info_packed & 0xFFFF;//2-byte units
	let face_texture_index_offset = face_info_packed >> 16;//2-byte units
	let face_offset = (face_array_offset + 2) * 2 + (face_index * face_size);//2-byte units
	let vertex_index = get_data_u16(face_offset + face_vertex_index);
	var vertex_relative: vec3f;
	if vertex_size == 14 {
		//TR5
		let vertex_offset = vertex_array_offset + 1 + (vertex_index * 7);//4-byte units
		vertex_relative = vec3f(
			bitcast<f32>(get_data_u32(vertex_offset)),
			bitcast<f32>(get_data_u32(vertex_offset + 1)),
			bitcast<f32>(get_data_u32(vertex_offset + 2)),
		);
	} else {
		//TR1234
		let vertex_offset = (vertex_array_offset + 1) * 2 + (vertex_index * vertex_size);//2-byte units
		let vertex_unsigned = vec3u(
			get_data_u16(vertex_offset),
			get_data_u16(vertex_offset + 1),
			get_data_u16(vertex_offset + 2),
		);
		let vertex_signed = vec3i(vertex_unsigned << vec3u(16)) >> vec3u(16);//interpret lower 16 as i16
		vertex_relative = vec3f(vertex_signed);
	}
	let vertex_absolute = local_transform * vec4f(vertex_relative, 1.0);
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
	@location(1) face: vec3u,//instance
) -> TextureVTF {
	let position_texture = get_position_texture(face, face_vertex_index);
	let position = position_texture.position;
	let object_texture_index = position_texture.texture_index & 0x3FFF;
	let object_id = position_texture.object_id;
	let object_texture_offset = data_offsets.object_textures_offset + object_texture_index * data_offsets.object_texture_size;
	// let blend_mode = get_data_u16(object_texture_offset);
	let atlas_index = get_data_u16(object_texture_offset + 1) & 0x7FFF;
	var uvs_offset: u32;
	if data_offsets.object_texture_size == 10 {
		//TR123
		uvs_offset = 2u;
	} else {
		//TR45
		uvs_offset = 3u;
	}
	let uv_offset = (object_texture_offset + uvs_offset) + face_vertex_index * 2;
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
	face: vec3u,
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
	@location(1) face: vec3u,//instance
) -> SolidVTF {
	return solid_vs(face_vertex_index, face, 0u);
}

@vertex
fn solid_32bit_vs_main(
	@location(0) face_vertex_index: u32,//vertex
	@location(1) face: vec3u,//instance
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
	let sprite_texture_offset = data_offsets.sprite_textures_offset + sprite_texture_index * 8;//8: size of SpriteTexture in u16s
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
	position_camera.y -= world_offset.y - 0;//constant: move sprites up to prevent ground clipping
	let position = perspective_transform * position_camera;
	let uv_int = sprite_pos + sprite_size * uv_index;
	let uv = vec2f(uv_int);
	return TextureVTF(position, atlas_index, uv, object_id);
}

struct Out {
	@location(0) color: vec4f,
	@location(1) object_id: u32,
}

//each texel is a color channel
@group(0) @binding(4) var palette: texture_1d<u32>;
@group(0) @binding(5) var atlases: texture_2d_array<u32>;

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

fn get_pixel(atlas_index: u32, uv: vec2f) -> u32 {
	return textureLoad(atlases, vec2i(uv), atlas_index, 0).x;
}

fn get_palette_color_24bit(color_index: u32) -> vec4f {
	if color_index == 0 {
		discard;
	} else {
		return get_palette_color(color_index, 3u, 63.0);
	}
}

fn get_color_16bit(color: u32) -> vec4f {
	if (color & 0x8000) == 0 {
		discard;
	} else {
		return to_f32_color(
			(color >> 10) & 0x1F,
			(color >> 5) & 0x1F,
			color & 0x1F,
			31.0,
		);
	}
}

fn get_color_32bit(color: u32) -> vec4f {
	if (color & 0xFF000000) == 0 {
		discard;
	} else {
		return to_f32_color(
			(color >> 16) & 0xFF,
			(color >> 8) & 0xFF,
			color & 0xFF,
			255.0,
		);
	}
}

@fragment
fn texture_palette_fs_main(vtf: TextureVTF) -> Out {
	let color_index = get_pixel(vtf.atlas_index, vtf.uv);
	let color = get_palette_color_24bit(color_index);
	return Out(color, vtf.object_id);
}

@fragment
fn texture_16bit_fs_main(vtf: TextureVTF) -> Out {
	let color_16bit = get_pixel(vtf.atlas_index, vtf.uv);
	let color = get_color_16bit(color_16bit);
	return Out(color, vtf.object_id);
}

@fragment
fn texture_32bit_fs_main(vtf: TextureVTF) -> Out {
	let color_32bit = get_pixel(vtf.atlas_index, vtf.uv);
	let color = get_color_32bit(color_32bit);
	return Out(color, vtf.object_id);
}

//==== flat texture ====

struct Viewport {
	width: u32,
	height: u32,
}

@group(0) @binding(6) var<uniform> viewport: Viewport;

struct FlatVTF {
	@builtin(position) position: vec4f,
	@location(0) pixel: vec2f,
}

@vertex
fn flat_vs_main(@location(0) vertex: u32) -> FlatVTF {
	let uv = vec2u(((vertex + 1) / 2) % 2, vertex / 2);
	let height = data_offsets.num_atlases * 256;
	let pixel = uv * vec2u(256, height);
	let uv_i = vec2i(uv);
	let pos = vec2i(2 * uv_i.x - 1, 1 - 2 * uv_i.y);
	return FlatVTF(vec4f(vec2f(pos), 0, 1), vec2f(pixel));
}

fn get_pixel2(pixel: vec2f) -> u32 {
	let pixel_int = vec2i(pixel);
	let atlas_pixel = vec2i(pixel_int.x, pixel_int.y % 256);
	let atlas_index = pixel_int.y / 256;
	return textureLoad(atlases, atlas_pixel, atlas_index, 0).x;
}

@fragment
fn flat_palette_fs_main(vtf: FlatVTF) -> @location(0) vec4f {
	let color_index = get_pixel2(vtf.pixel);
	let color = get_palette_color_24bit(color_index);
	return color;
}

@fragment
fn flat_16bit_fs_main(vtf: FlatVTF) -> @location(0) vec4f {
	let color_16bit = get_pixel2(vtf.pixel);
	let color = get_color_16bit(color_16bit);
	return color;
}

@fragment
fn flat_32bit_fs_main(vtf: FlatVTF) -> @location(0) vec4f {
	let color_32bit = get_pixel2(vtf.pixel);
	let color = get_color_32bit(color_32bit);
	return color;
}
