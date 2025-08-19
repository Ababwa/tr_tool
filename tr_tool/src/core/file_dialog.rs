use std::{fs, io, path::PathBuf};
use egui_file_dialog::DialogState;

enum State {
	Level,
	Texture,
}

/// Wrapper for file dialog to abstract details, in case of crate change, etc.
pub struct FileDialog {
	file_dialog: egui_file_dialog::FileDialog,
	level_path: PathBuf,
	texture_path: PathBuf,
	state: State,
}

const STATE_PATH: &str = "state.txt";

fn ensure_exists(path: &mut PathBuf) {
	loop {
		if path.exists() {
			break;
		}
		if !path.pop() {
			*path = PathBuf::from(".");
			break;
		}
	}
}

fn set_dir(file_dialog: &mut egui_file_dialog::FileDialog, path: &mut PathBuf) {
	ensure_exists(path);
	file_dialog.config_mut().initial_directory = path.clone();
}

fn read_state(level_path: &mut PathBuf, texture_path: &mut PathBuf) -> io::Result<()> {
	let state_str = fs::read_to_string(STATE_PATH)?;
	let Some(split) = state_str.find('\n') else {
		return Ok(());
	};
	*level_path = state_str[..split].into();
	*texture_path = state_str[split + 1..].into();
	Ok(())
}

fn save_state(level_path: &PathBuf, texture_path: &PathBuf) -> io::Result<()> {
	let level_path_bytes = level_path.as_os_str().as_encoded_bytes();
	let texture_path_bytes = texture_path.as_os_str().as_encoded_bytes();
	let bytes = [level_path_bytes, b"\n", texture_path_bytes].concat();
	fs::write(STATE_PATH, bytes)
}

impl FileDialog {
	pub fn new() -> Self {
		let mut file_dialog = egui_file_dialog::FileDialog::new();
		file_dialog.config_mut().opening_mode = egui_file_dialog::OpeningMode::AlwaysInitialDir;
		let mut level_path = PathBuf::from(".");
		let mut texture_path = PathBuf::from(".");
		if let Err(e) = read_state(&mut level_path, &mut texture_path) {
			eprintln!("error reading file dialog state: {}", e);
		}
		Self {
			file_dialog,
			level_path,
			texture_path,
			state: State::Level,//doesn't matter
		}
	}
	
	fn save_state(&self) {
		if let Err(e) = save_state(&self.level_path, &self.texture_path) {
			eprintln!("failed to save file dialog state: {}", e);
		}
	}
	
	pub fn pick_level_file(&mut self) {
		if !self.is_open() {
			set_dir(&mut self.file_dialog, &mut self.level_path);
			self.file_dialog.pick_file();
			self.state = State::Level;
		}
	}
	
	pub fn get_level_path(&mut self) -> Option<PathBuf> {
		if let State::Level = self.state {
			let path = self.file_dialog.take_picked();
			if let Some(path) = &path {
				self.level_path = path.clone();
				self.save_state();
			}
			path
		} else {
			None
		}
	}
	
	pub fn save_texture(&mut self) {
		if !self.is_open() {
			set_dir(&mut self.file_dialog, &mut self.texture_path);
			self.file_dialog.save_file();
			self.state = State::Texture;
		}
	}
	
	pub fn get_texture_path(&mut self) -> Option<PathBuf> {
		if let State::Texture = self.state {
			let path = self.file_dialog.take_picked();
			if let Some(path) = &path {
				self.texture_path = path.clone();
				self.save_state();
			}
			path
		} else {
			None
		}
	}
	
	pub fn update(&mut self, ctx: &egui::Context) {
		self.file_dialog.update(ctx);
	}
	
	//TODO: can be const once `FileDialog::state` is const
	pub fn is_open(&self) -> bool {
		let state = self.file_dialog.state();
		matches!(state, DialogState::Open)
	}
}
