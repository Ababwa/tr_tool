use std::{f32::consts::{FRAC_PI_2, FRAC_PI_4, PI}, ops::Range, time::Duration};
use glam::{EulerRot, Mat4, Vec3};
use tr_model::tr1;
use wgpu::{
	BindGroup, Buffer, CommandEncoder, Device, IndexFormat, Queue, RenderPass, RenderPipeline, Texture,
	TextureView, TextureViewDescriptor,
};
use winit::{
	dpi::{PhysicalPosition, PhysicalSize}, event::{ElementState, MouseButton}, keyboard::KeyCode,
	window::{CursorGrabMode, Window},
};
use crate::{
	as_bytes::AsBytes, boxed_slice::Bsf, gfx,
	level_parse::{BindGroups, LayerOffsets, LevelData, MeshOffsets, RoomRenderData},
	render_resources::{BindingBuffers, GeomOffsets, PipelineGroup, RenderResources, Viewport},
};
use super::{draw_window, file_dialog::FileDialog, keys::KeyStates, Ui};

#[derive(Clone, Copy, PartialEq, Eq)]
enum TextureModeTag {
	Palette,
	Bit16,
	Bit32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SolidModeTag {
	Bit24,
	Bit32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TextureTabTag {
	Palette,
	Bit16,
	Bit32,
	Misc,
}

struct TextureMode {
	pipeline_group: PipelineGroup,
	bind_group: BindGroup,
	tag: TextureModeTag,
}

struct SolidMode {
	pipeline: RenderPipeline,
	bind_group: BindGroup,
	tag: SolidModeTag,
}

struct TextureTab {
	pipeline: RenderPipeline,
	bind_group: BindGroup,
	num_images: u32,
	tag: TextureTabTag,
}

#[derive(Clone, Copy)]
struct Camera {
	pos: Vec3,
	yaw: f32,
	pitch: f32,
}

//TODO: References into `level_data.room_render_data` instead of indices. (need some self-reference hack)
pub struct LoadedLevel {
	depth_view: TextureView,
	interact_view: TextureView,
	interact_texture: Texture,
	texture_mode: TextureMode,
	solid_mode: Option<SolidMode>,
	texture_tab: TextureTab,
	/// Index into `level_data.room_render_data` of room to render. If `None`, render all.
	room_index: Option<usize>,
	/**
	Indices into `level_data.room_render_data` of all rooms, given flip group toggles.
	Contains the active indices of flip groups in order, then static room indices.
	*/
	all_room_indices: Box<[usize]>,
	camera: Camera,
	mouse_control: bool,
	show_room_mesh: bool,
	show_static_meshes: bool,
	show_entity_meshes: bool,
	show_room_sprites: bool,
	show_entity_sprites: bool,
	show_render_options: bool,
	show_textures_window: bool,
	key_states: KeyStates,
	mouse_pos: PhysicalPosition<f64>,
	level_data: LevelData,
}

struct TextureCallback {
	queue: Queue,
	viewport_buffer: Buffer,
	face_vertex_index_buffer: Buffer,
	pipeline: RenderPipeline,
	bind_group: BindGroup,
}

impl egui_wgpu::CallbackTrait for TextureCallback {
	fn paint(
		&self,
		info: egui::PaintCallbackInfo,
		pass: &mut wgpu::RenderPass<'static>,
		_: &egui_wgpu::CallbackResources,
	) {
		let c = info.clip_rect_in_pixels();
		let v = info.viewport_in_pixels();
		let viewport = Viewport {
			clip: [c.left_px, c.top_px, c.width_px, c.height_px],
			view: [v.left_px, v.top_px, v.width_px, v.height_px],
		};
		self.queue.write_buffer(&self.viewport_buffer, 0, viewport.as_bytes());
		pass.set_vertex_buffer(0, self.face_vertex_index_buffer.slice(..));
		pass.set_pipeline(&self.pipeline);
		pass.set_bind_group(0, &self.bind_group, &[]);
		pass.draw(0..NUM_QUAD_VERTICES, 0..1);
	}
}

const FRAC_1_SQRT_3: f32 = 0.577350269189625764509148780501957456;
/// Initial direction of the camera.
const CAMERA_VEC: Vec3 = Vec3::splat(FRAC_1_SQRT_3);

//yaw and pitch of `CAMERA_VEC`

/// -3π/4
const START_YAW: f32 = -2.356194490192344928846982537459627163;
/// atan(1/sqrt(2))
const START_PITCH: f32 = 0.615479708670387341067464589123993688;

const NUM_QUAD_VERTICES: u32 = 4;
const NUM_TRI_VERTICES: u32 = 3;

const FORWARD_KEYS: [KeyCode; 2] = [KeyCode::KeyW, KeyCode::ArrowUp];
const BACKWARD_KEYS: [KeyCode; 2] = [KeyCode::KeyS, KeyCode::ArrowDown];
const LEFT_KEYS: [KeyCode; 2] = [KeyCode::KeyA, KeyCode::ArrowLeft];
const RIGHT_KEYS: [KeyCode; 2] = [KeyCode::KeyD, KeyCode::ArrowRight];
const UP_KEYS: [KeyCode; 2] = [KeyCode::KeyQ, KeyCode::PageUp];
const DOWN_KEYS: [KeyCode; 2] = [KeyCode::KeyE, KeyCode::PageDown];
const FAST_KEYS: [KeyCode; 2] = [KeyCode::ShiftLeft, KeyCode::ShiftRight];
const SLOW_KEYS: [KeyCode; 2] = [KeyCode::ControlLeft, KeyCode::ControlRight];

const FORWARD_VEC: Vec3 = Vec3::NEG_Z;
const BACKWARD_VEC: Vec3 = Vec3::Z;
const LEFT_VEC: Vec3 = Vec3::X;
const RIGHT_VEC: Vec3 = Vec3::NEG_X;
const DOWN_VEC: Vec3 = Vec3::Y;
const UP_VEC: Vec3 = Vec3::NEG_Y;

const ALL: &'static str = "All";

impl TextureModeTag {
	fn label(self) -> &'static str {
		match self {
			TextureModeTag::Palette => "Palette",
			TextureModeTag::Bit16 => "16 Bit",
			TextureModeTag::Bit32 => "32 Bit",
		}
	}
	
	fn to_tab(self) -> TextureTabTag {
		match self {
			TextureModeTag::Palette => TextureTabTag::Palette,
			TextureModeTag::Bit16 => TextureTabTag::Bit16,
			TextureModeTag::Bit32 => TextureTabTag::Bit32,
		}
	}
}

impl SolidModeTag {
	fn label(self) -> &'static str {
		match self {
			SolidModeTag::Bit24 => "24 Bit",
			SolidModeTag::Bit32 => "32 Bit",
		}
	}
}

impl TextureTabTag {
	fn label(self) -> &'static str {
		match self {
			TextureTabTag::Palette => "Palette",
			TextureTabTag::Bit16 => "16 Bit",
			TextureTabTag::Bit32 => "32 Bit",
			TextureTabTag::Misc => "Misc",
		}
	}
	
	fn scroll_id(self) -> u8 {
		match self {
			TextureTabTag::Palette | TextureTabTag::Bit16 | TextureTabTag::Bit32 => 0,
			TextureTabTag::Misc => 1,
		}
	}
}

fn direction(yaw: f32, pitch: f32) -> Vec3 {
	let (yaw_sin, yaw_cos) = yaw.sin_cos();
	let (pitch_sin, pitch_cos) = pitch.sin_cos();
	Vec3::new(-pitch_cos * yaw_sin, pitch_sin, -pitch_cos * yaw_cos)
}

fn make_camera_transform(camera: &Camera) -> Mat4 {
	Mat4::from_euler(EulerRot::XYZ, camera.pitch, camera.yaw, PI) * Mat4::from_translation(-camera.pos)
}

//TODO: Test z_near.
fn make_perspective_transform(window_size: PhysicalSize<u32>) -> Mat4 {
	Mat4::perspective_rh(FRAC_PI_4, window_size.width as f32 / window_size.height as f32, 100.0, 100000.0)
}

fn get_texture_mode(rr: &RenderResources, bind_groups: &BindGroups) -> TextureMode {
	//prefer higher bit textures
	if let Some(texture_32bit_bg) = &bind_groups.texture_32bit_bg {
		TextureMode {
			pipeline_group: rr.texture_32bit_plg.clone(),
			bind_group: texture_32bit_bg.clone(),
			tag: TextureModeTag::Bit32,
		}
	} else if let Some(texture_16bit_bg) = &bind_groups.texture_16bit_bg {
		TextureMode {
			pipeline_group: rr.texture_16bit_plg.clone(),
			bind_group: texture_16bit_bg.clone(),
			tag: TextureModeTag::Bit16,
		}
	} else if let Some(palette_bg) = &bind_groups.palette_bg {
		TextureMode {
			pipeline_group: rr.texture_palette_plg.clone(),
			bind_group: palette_bg.clone(),
			tag: TextureModeTag::Palette,
		}
	} else {
		panic!("no texture");
	}
}

fn get_solid_mode(rr: &RenderResources, bind_groups: &BindGroups) -> Option<SolidMode> {
	if let Some(solid_32bit_bg) = &bind_groups.solid_32bit_bg {
		let solid_mode = SolidMode {
			pipeline: rr.solid_32bit_pl.clone(),
			bind_group: solid_32bit_bg.clone(),
			tag: SolidModeTag::Bit32,
		};
		Some(solid_mode)
	} else if let Some(palette_bg) = &bind_groups.palette_bg {
		let solid_mode = SolidMode {
			pipeline: rr.solid_24bit_pl.clone(),
			bind_group: palette_bg.clone(),
			tag: SolidModeTag::Bit24,
		};
		Some(solid_mode)
	} else {
		None
	}
}

fn lock_cursor(window: &Window) {
	let Err(e1) = window.set_cursor_grab(CursorGrabMode::Confined) else { return; };
	let Err(e2) = window.set_cursor_grab(CursorGrabMode::Locked) else { return; };
	panic!("cursor grab: ({}, {})", e1, e2);
}

trait RangeGetter<T> {
	fn quads(offsets: &T) -> Range<u32>;
	fn tris(offsets: &T) -> Range<u32>;
}

macro_rules! range_getter {
	($type:ty, $name:ty, $quads:ident, $tris:ident, $get:ident) => {
		impl RangeGetter<$type> for $name {
			fn quads(offsets: &$type) -> Range<u32> { offsets.$quads.$get() }
			fn tris(offsets: &$type) -> Range<u32> { offsets.$tris.$get() }
		}
	};
}

struct Solid;
struct Opaque;
struct Additive;

range_getter!(MeshOffsets, Solid, solid_quads, solid_tris, clone);
range_getter!(MeshOffsets, Opaque, textured_quads, textured_tris, opaque);
range_getter!(MeshOffsets, Additive, textured_quads, textured_tris, additive);
range_getter!(LayerOffsets, Opaque, quads, tris, opaque_obverse);
range_getter!(LayerOffsets, [Opaque], quads, tris, opaque_reverse);
range_getter!(LayerOffsets, Additive, quads, tris, additive_obverse);
range_getter!(LayerOffsets, [Additive], quads, tris, additive_reverse);

fn draw_mesh<R: RangeGetter<MeshOffsets>>(pass: &mut RenderPass, mesh: &MeshOffsets) {
	pass.draw(0..NUM_QUAD_VERTICES, R::quads(mesh));
	pass.draw(0..NUM_TRI_VERTICES, R::tris(mesh));
}

fn draw_statics<R: RangeGetter<MeshOffsets>>(pass: &mut RenderPass, static_meshes: &[MeshOffsets]) {
	for static_mesh in static_meshes {
		draw_mesh::<R>(pass, static_mesh);
	}
}

fn draw_entities<R: RangeGetter<MeshOffsets>>(pass: &mut RenderPass, entities: &[Box<[MeshOffsets]>]) {
	for entity_meshes in entities {
		for entity_mesh in entity_meshes {
			draw_mesh::<R>(pass, entity_mesh);
		}
	}
}

fn draw_layers<R>(pass: &mut RenderPass, layers: &[LayerOffsets])
where R: RangeGetter<LayerOffsets>, [R]: RangeGetter<LayerOffsets> {
	for layer in layers {
		pass.draw(0..NUM_QUAD_VERTICES, R::quads(layer));
		pass.draw(0..NUM_TRI_VERTICES, R::tris(layer));
		pass.draw_indexed(0..NUM_QUAD_VERTICES, 0, <[R]>::quads(layer));
		pass.draw_indexed(0..NUM_TRI_VERTICES, 0, <[R]>::tris(layer));
	}
}

#[derive(Clone, Copy)]
struct Rooms<'a> {
	rooms: &'a [RoomRenderData],
	indices: &'a [usize],
}

struct RoomIter<'a> {
	rooms: &'a [RoomRenderData],
	indices_iter: std::slice::Iter<'a, usize>,
}

impl<'a> Iterator for RoomIter<'a> {
	type Item = &'a RoomRenderData;
	fn next(&mut self) -> Option<Self::Item> {
		let &index = self.indices_iter.next()?;
		Some(&self.rooms[index])
	}
}

impl<'a> IntoIterator for Rooms<'a> {
	type Item = &'a RoomRenderData;
	type IntoIter = RoomIter<'a>;
	fn into_iter(self) -> Self::IntoIter {
		Self::IntoIter {
			rooms: self.rooms,
			indices_iter: self.indices.iter(),
		}
	}
}

fn write_buffers(
	window_size: PhysicalSize<u32>,
	queue: &Queue,
	buffers: &BindingBuffers,
	level_data: &LevelData,
	camera: &Camera,
) {
	let geom_offsets = GeomOffsets {
		transforms_offset: level_data.geom.transforms_offset,
		face_array_offsets_offset: level_data.geom.face_array_offsets_offset,
		object_textures_offset: level_data.geom.object_textures_offset,
		sprite_textures_offset: level_data.geom.sprite_textures_offset,
		object_texture_size: level_data.object_texture_size,
		num_atlases: level_data.num_atlases,
	};
	let camera_transform = make_camera_transform(camera);
	let perspective_transform = make_perspective_transform(window_size);
	queue.write_buffer(&buffers.geom_buffer, 0, (&*level_data.geom.buffer).as_bytes());
	queue.write_buffer(&buffers.geom_offsets_buffer, 0, geom_offsets.as_bytes());
	queue.write_buffer(&buffers.camera_transform_buffer, 0, camera_transform.as_bytes());
	queue.write_buffer(&buffers.perspective_transform_buffer, 0, perspective_transform.as_bytes());
}

fn room_str(room_index: usize) -> String {
	format!("Room {}", room_index)
}

fn texture_mode_ui(
	ui: Ui,
	texture_mode: &mut TextureMode,
	pipeline_group: &PipelineGroup,
	bind_group: &Option<BindGroup>,
	tag: TextureModeTag,
) {
	if let Some(bind_group) = bind_group {
		if ui.selectable_value(&mut texture_mode.tag, tag, tag.label()).changed() {
			texture_mode.pipeline_group = pipeline_group.clone();
			texture_mode.bind_group = bind_group.clone();
		}
	}
}

fn texture_modes_ui(ui: Ui, rr: &RenderResources, texture_mode: &mut TextureMode, bgs: &BindGroups) {
	texture_mode_ui(ui, texture_mode, &rr.texture_palette_plg, &bgs.palette_bg, TextureModeTag::Palette);
	texture_mode_ui(ui, texture_mode, &rr.texture_16bit_plg, &bgs.texture_16bit_bg, TextureModeTag::Bit16);
	texture_mode_ui(ui, texture_mode, &rr.texture_32bit_plg, &bgs.texture_32bit_bg, TextureModeTag::Bit32);
}

fn solid_mode_ui(
	ui: Ui,
	solid_mode: &mut SolidMode,
	pipeline: &RenderPipeline,
	bind_group: &BindGroup,
	tag: SolidModeTag,
) {
	if ui.selectable_value(&mut solid_mode.tag, tag, tag.label()).changed() {
		solid_mode.pipeline = pipeline.clone();
		solid_mode.bind_group = bind_group.clone();
	}
}

fn texture_tab_ui(
	ui: Ui,
	texture_tab: &mut TextureTab,
	pipeline_group: &PipelineGroup,
	bind_group: &BindGroup,
	tag: TextureTabTag,
	num_images: u32,
) {
	if ui.selectable_value(&mut texture_tab.tag, tag, tag.label()).changed() {
		texture_tab.pipeline = pipeline_group.flat_pl.clone();
		texture_tab.bind_group = bind_group.clone();
		texture_tab.num_images = num_images;
	}
}

fn texture_tab_ui_test(
	ui: Ui,
	texture_tab: &mut TextureTab,
	pipeline_group: &PipelineGroup,
	bind_group: &Option<BindGroup>,
	tag: TextureTabTag,
	num_images: u32,
) {
	if let Some(bind_group) = bind_group {
		texture_tab_ui(ui, texture_tab, pipeline_group, bind_group, tag, num_images);
	}
}

fn texture_tabs_ui(ui: Ui, texture_tab: &mut TextureTab, bgs: &BindGroups, rr: &RenderResources, na: u32) {
	use texture_tab_ui_test as tab;
	tab(ui, texture_tab, &rr.texture_palette_plg, &bgs.palette_bg, TextureTabTag::Palette, na);
	tab(ui, texture_tab, &rr.texture_16bit_plg, &bgs.texture_16bit_bg, TextureTabTag::Bit16, na);
	tab(ui, texture_tab, &rr.texture_32bit_plg, &bgs.texture_32bit_bg, TextureTabTag::Bit32, na);
	if let &Some((ref bind_group, num_images)) = &bgs.misc_images_bg {
		texture_tab_ui(ui, texture_tab, &rr.texture_32bit_plg, bind_group, TextureTabTag::Misc, num_images);
	}
}

impl LoadedLevel {
	pub fn new(
		window_size: PhysicalSize<u32>,
		device: &Device,
		queue: &Queue,
		rr: &RenderResources,
		level_data: LevelData,
	) -> Self {
		let pos = match level_data.room_render_data.first() {
			Some(rrd) => rrd.center - rrd.radius * CAMERA_VEC,
			None => Vec3::ZERO,
		};
		let camera = Camera {
			pos,
			yaw: START_YAW,
			pitch: START_PITCH,
		};
		let flip_rooms = match level_data.flip_groups.last() {
			Some(flip_group) => flip_group.offset + flip_group.room_indices.len() / 2,
			None => 0,
		};
		let num_all_rooms = flip_rooms + level_data.static_room_indices.len();
		let mut all_room_indices = Bsf::new(num_all_rooms);
		for flip_group in &level_data.flip_groups {
			all_room_indices.extend_copy(flip_group.active_indices());
		}
		all_room_indices.extend_copy(&level_data.static_room_indices);
		let interact_texture = gfx::interact_texture(device, window_size);
		write_buffers(window_size, queue, &rr.binding_buffers, &level_data, &camera);
		let texture_mode = get_texture_mode(rr, &level_data.bind_groups);
		let texture_tab = TextureTab {
			pipeline: texture_mode.pipeline_group.flat_pl.clone(),
			bind_group: texture_mode.bind_group.clone(),
			num_images: level_data.num_atlases,
			tag: texture_mode.tag.to_tab(),
		};
		Self {
			depth_view: gfx::depth_view(device, window_size),
			interact_view: interact_texture.create_view(&TextureViewDescriptor::default()),
			interact_texture,
			texture_mode,
			solid_mode: get_solid_mode(rr, &level_data.bind_groups),
			texture_tab,
			room_index: None,
			all_room_indices: all_room_indices.into_boxed_slice(),
			camera,
			mouse_control: false,
			show_room_mesh: true,
			show_static_meshes: true,
			show_entity_meshes: true,
			show_room_sprites: true,
			show_entity_sprites: true,
			show_render_options: true,
			show_textures_window: true,
			key_states: KeyStates::new(),
			mouse_pos: PhysicalPosition::default(),
			level_data,
		}
	}
	
	pub fn resize(
		&mut self,
		device: &Device,
		queue: &Queue,
		perspective_transform_buffer: &Buffer,
		window_size: PhysicalSize<u32>,
	) {
		self.depth_view = gfx::depth_view(device, window_size);
		self.interact_texture = gfx::interact_texture(device, window_size);
		self.interact_view = self.interact_texture.create_view(&TextureViewDescriptor::default());
		let perspective_transform = make_perspective_transform(window_size);
		queue.write_buffer(perspective_transform_buffer, 0, perspective_transform.as_bytes());
	}
	
	fn toggle(&self, key_codes: &[KeyCode]) -> f32 {
		self.key_states.any(key_codes) as u8 as f32
	}
	
	fn frame_update(&mut self, queue: &Queue, camera_transform_buffer: &Buffer, delta_time: Duration) {
		let delta =
			self.toggle(&FORWARD_KEYS) * FORWARD_VEC +
			self.toggle(&BACKWARD_KEYS) * BACKWARD_VEC +
			self.toggle(&LEFT_KEYS) * LEFT_VEC +
			self.toggle(&RIGHT_KEYS) * RIGHT_VEC +
			self.toggle(&DOWN_KEYS) * DOWN_VEC +
			self.toggle(&UP_KEYS) * UP_VEC;
		let factor =
			5000.0 *
			5f32.powf(self.toggle(&FAST_KEYS) - self.toggle(&SLOW_KEYS)) *
			delta_time.as_secs_f32();
		self.camera.pos += factor * Mat4::from_rotation_y(self.camera.yaw).transform_point3(delta);
		let camera_transform = make_camera_transform(&self.camera);
		queue.write_buffer(camera_transform_buffer, 0, camera_transform.as_bytes());
	}
	
	fn flip_groups_ui(&mut self, ui: Ui) {
		if !self.level_data.flip_groups.is_empty() {
			let flip_group_toggles = |ui: Ui| {
				ui.label("Flip Groups");
				for flip_group in &mut self.level_data.flip_groups {
					if ui.toggle_value(&mut flip_group.flipped, flip_group.number.to_string()).changed() {
						let active_indices = flip_group.active_indices();
						let dest = &mut self.all_room_indices[flip_group.offset..][..active_indices.len()];
						dest.copy_from_slice(active_indices);
					}
				}
			};
			ui.horizontal(flip_group_toggles);
		}
	}
	
	fn rooms_ui(&mut self, ui: Ui) {
		let room_combo = egui::ComboBox::from_label("Room");
		let room_combo = match self.room_index {
			Some(index) => room_combo.selected_text(room_str(index)),
			None => room_combo.selected_text(ALL),
		};
		let mut changed = false;
		let room_values = |ui: Ui| {
			changed |= ui.selectable_value(&mut self.room_index, None, ALL).changed();
			for index in 0..self.level_data.room_render_data.len() {
				changed |= ui.selectable_value(&mut self.room_index, Some(index), room_str(index)).changed();
			}
		};
		room_combo.show_ui(ui, room_values);
		if changed {
			if let Some(index) = self.room_index {
				let RoomRenderData { center, radius, .. } = self.level_data.room_render_data[index];
				self.camera.pos = center - direction(self.camera.yaw, self.camera.pitch) * radius;
			}
		}
	}
	
	fn egui_render_modes(&mut self, ui: Ui, rr: &RenderResources) {
		let bgs = &self.level_data.bind_groups;
		let available_texture_modes =
			bgs.texture_32bit_bg.is_some() as u8 +
			bgs.texture_16bit_bg.is_some() as u8 +
			bgs.palette_bg.is_some() as u8;
		if available_texture_modes > 1 {
			let texture_mode_combo = egui::ComboBox::from_label("Texture Mode");
			let texture_mode_combo = texture_mode_combo.selected_text(self.texture_mode.tag.label());
			let texture_modes = |ui: Ui| texture_modes_ui(ui, rr, &mut self.texture_mode, bgs);
			texture_mode_combo.show_ui(ui, texture_modes);
		}
		let solid_condition = (&mut self.solid_mode, &bgs.solid_32bit_bg, &bgs.palette_bg);
		if let (Some(solid_mode), Some(solid_32bit_bg), Some(palette_bg)) = solid_condition {
			let solid_mode_combo = egui::ComboBox::from_label("Solid Color Mode");
			let solid_mode_combo = solid_mode_combo.selected_text(solid_mode.tag.label());
			let solid_modes = |ui: Ui| {
				solid_mode_ui(ui, solid_mode, &rr.solid_24bit_pl, palette_bg, SolidModeTag::Bit24);
				solid_mode_ui(ui, solid_mode, &rr.solid_32bit_pl, solid_32bit_bg, SolidModeTag::Bit32);
			};
			solid_mode_combo.show_ui(ui, solid_modes);
		}
	}
	
	fn render_options_ui(&mut self, ui: Ui, rr: &RenderResources) {
		self.egui_render_modes(ui, rr);
		self.rooms_ui(ui);
		match self.room_index {
			None => self.flip_groups_ui(ui),
			Some(index) => {
				if let Some(flip_state) = &self.level_data.room_render_data[index].flip_state {
					let word = if flip_state.original { "Original" } else { "Flipped" };
					let text = format!("{} of {}", word, flip_state.other_index);
					if ui.button(text).clicked() {
						self.room_index = Some(flip_state.other_index);
					}
				}
			},
		}
	}
	
	fn textures_ui(&mut self, ui: Ui, queue: &Queue, rr: &RenderResources) {
		let bgs = &self.level_data.bind_groups;
		let available_textures =
			bgs.texture_32bit_bg.is_some() as u8 +
			bgs.texture_16bit_bg.is_some() as u8 +
			bgs.palette_bg.is_some() as u8 +
			bgs.misc_images_bg.is_some() as u8;
		if available_textures == 1 {
			ui.label(self.texture_tab.tag.label());
		} else {
			let tabs = |ui: Ui| {
				texture_tabs_ui(ui, &mut self.texture_tab, bgs, rr, self.level_data.num_atlases);
			};
			ui.horizontal(tabs);
		}
		if ui.button("Save").clicked() {
			println!("Save texture");//TODO: save texture
		}
		ui.add_space(2.0);
		let texture = |ui: Ui| {
			const WIDTH: f32 = tr1::ATLAS_SIDE_LEN as f32;
			let height = (self.texture_tab.num_images * 256) as f32;
			let (_, rect) = ui.allocate_space(egui::vec2(WIDTH, height));
			let texture_cb = TextureCallback {
				queue: queue.clone(),
				viewport_buffer: rr.binding_buffers.viewport_buffer.clone(),
				face_vertex_index_buffer: rr.face_vertex_indices_buffer.clone(),
				pipeline: self.texture_tab.pipeline.clone(),
				bind_group: self.texture_tab.bind_group.clone(),
			};
			let paint_cb = egui_wgpu::Callback::new_paint_callback(rect, texture_cb);
			ui.painter().add(paint_cb);
		};
		let scroll_id = self.texture_tab.tag.scroll_id();
		let scroll_output = egui::ScrollArea::vertical().id_salt(scroll_id).show(ui, texture);
		let scroll_offset = scroll_output.state.offset;
		queue.write_buffer(&rr.binding_buffers.scroll_offset_buffer, 0, scroll_offset.as_bytes());
	}
	
	pub fn egui(&mut self, queue: &Queue, rr: &RenderResources, ctx: &egui::Context) {
		if self.show_render_options {
			let render_options = |ui: Ui| self.render_options_ui(ui, rr);
			self.show_render_options = draw_window(ctx, "Render Options", false, render_options);
		}
		if self.show_textures_window {
			let textures_window = |ui: Ui| self.textures_ui(ui, queue, rr);
			self.show_textures_window = draw_window(ctx, "Textures", true, textures_window);
		}
	}
	
	pub fn render(
		&mut self,
		queue: &Queue,
		rr: &RenderResources,
		encoder: &mut CommandEncoder,
		view: &TextureView,
		delta_time: Duration,
	) {
		self.frame_update(queue, &rr.binding_buffers.camera_transform_buffer, delta_time);
		let room_indices: &[_] = match self.room_index {
			Some(room_index) => &[room_index],
			None => &self.all_room_indices,
		};
		//TODO: Compare with allocating a vec of refs.
		let rooms = Rooms {
			rooms: &self.level_data.room_render_data,
			indices: room_indices,
		};
		let mut pass = gfx::main_render_pass(encoder, view, &self.interact_view, &self.depth_view);
		pass.set_index_buffer(rr.reverse_indices_buffer.slice(..), IndexFormat::Uint16);
		pass.set_vertex_buffer(0, rr.face_vertex_indices_buffer.slice(..));
		if self.level_data.face_instance_buffer.size() > 0 {
			pass.set_vertex_buffer(1, self.level_data.face_instance_buffer.slice(..));
			if let Some(solid_mode) = &self.solid_mode {
				pass.set_bind_group(0, &solid_mode.bind_group, &[]);
				pass.set_pipeline(&solid_mode.pipeline);
				if self.show_static_meshes {
					for room in rooms {
						draw_statics::<Solid>(&mut pass, &room.static_meshes);
					}
				}
				if self.show_entity_meshes {
					for room in rooms {
						draw_entities::<Solid>(&mut pass, &room.entity_meshes);
					}
				}
			}
			pass.set_bind_group(0, &self.texture_mode.bind_group, &[]);
			pass.set_pipeline(&self.texture_mode.pipeline_group.opaque_pl);
			if self.show_static_meshes {
				for room in rooms {
					draw_statics::<Opaque>(&mut pass, &room.static_meshes);
				}
			}
			if self.show_entity_meshes {
				for room in rooms {
					draw_entities::<Opaque>(&mut pass, &room.entity_meshes);
				}
			}
			if self.show_room_mesh {
				for room in rooms {
					draw_layers::<Opaque>(&mut pass, &room.layers);
				}
			}
			pass.set_pipeline(&self.texture_mode.pipeline_group.additive_pl);
			if self.show_static_meshes {
				for room in rooms {
					draw_statics::<Additive>(&mut pass, &room.static_meshes);
				}
			}
			if self.show_entity_meshes {
				for room in rooms {
					draw_entities::<Additive>(&mut pass, &room.entity_meshes);
				}
			}
			if self.show_room_mesh {
				for room in rooms {
					draw_layers::<Additive>(&mut pass, &room.layers);
				}
			}
		}
		if self.level_data.sprite_instance_buffer.size() > 0 {
			pass.set_vertex_buffer(1, self.level_data.sprite_instance_buffer.slice(..));
			pass.set_pipeline(&self.texture_mode.pipeline_group.sprite_pl);
			if self.show_room_sprites {
				for room in rooms {
					pass.draw(0..NUM_QUAD_VERTICES, room.room_sprites.clone());
				}
			}
			if self.show_entity_sprites {
				for room in rooms {
					pass.draw(0..NUM_QUAD_VERTICES, room.entity_sprites.clone());
				}
			}
		}
	}
	
	pub fn key(&mut self, key_code: KeyCode, state: ElementState) {
		self.key_states.set(key_code, matches!(state, ElementState::Pressed));
		match (state, key_code) {
			(ElementState::Pressed, KeyCode::KeyR) => self.show_render_options ^= true,
			(ElementState::Pressed, KeyCode::KeyT) => self.show_textures_window ^= true,
			_ => {},
		}
	}
	
	pub fn set_mouse_control(&mut self, window: &Window, on: bool) {
		match (self.mouse_control, on) {
			(true, false) => {
				window.set_cursor_visible(true);
				window.set_cursor_grab(CursorGrabMode::None).expect("cursor ungrab");
			},
			(false, true) => {
				window.set_cursor_visible(false);
				lock_cursor(window);
			},
			_ => {},
		}
		self.mouse_control = on;
	}
	
	pub fn mouse_button(
		&mut self,
		window: &Window,
		file_dialog: &FileDialog,
		state: ElementState,
		button: MouseButton,
	) {
		match (state, button) {
			(ElementState::Pressed, MouseButton::Right) => {
				if file_dialog.is_closed() {
					self.set_mouse_control(window, !self.mouse_control);
				}
			},
			_ => {},
		}
	}
	
	pub fn cursor_moved(&mut self, window: &Window, pos: PhysicalPosition<f64>) {
		if self.mouse_control {
			window.set_cursor_position(self.mouse_pos).expect("set cursor pos");
		} else {
			self.mouse_pos = pos;
		}
	}
	
	pub fn mouse_motion(&mut self, x: f32, y: f32) {
		if self.mouse_control {
			self.camera.yaw += x / 150.0;
			self.camera.pitch = (self.camera.pitch + y / 150.0).clamp(-FRAC_PI_2, FRAC_PI_2);
		}
	}
}
