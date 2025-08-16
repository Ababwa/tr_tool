use std::{fs, io, path::PathBuf, str::FromStr};
use egui_file_dialog::DialogState;

/// Wrapper for file dialog to abstract details, in case of crate change, etc.
pub struct FileDialog {
	file_dialog: egui_file_dialog::FileDialog,
}

const STATE_PATH: &str = "state";

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

fn read_last_path() -> Option<PathBuf> {
	let state_str = match fs::read_to_string(STATE_PATH) {
		Ok(state_str) => state_str,
		Err(e) => {
			if e.kind() != io::ErrorKind::NotFound {
				eprintln!("failed to read file dialog state: {}", e);
			}
			return None;
		},
	};
	let path = match PathBuf::from_str(&state_str) {
		Ok(path) => path,
		Err(e) => {
			eprintln!("file dialog state not a path: {}", e);
			return None;
		},
	};
	Some(path)
}

impl FileDialog {
	pub fn new() -> Self {
		let mut file_dialog = egui_file_dialog::FileDialog::new();
		if let Some(path) = read_last_path() {
			file_dialog.config_mut().initial_directory = path;
		}
		Self {
			file_dialog,
		}
	}
	
	pub fn pick_level_file(&mut self) {
		if self.is_closed() {
			ensure_exists(&mut self.file_dialog.config_mut().initial_directory);
			self.file_dialog.pick_file();
		}
	}
	
	pub fn get_level_path(&mut self) -> Option<PathBuf> {
		let path = self.file_dialog.take_picked();
		if let Some(path) = &path {
			if let Err(e) = fs::write(STATE_PATH, path.as_os_str().as_encoded_bytes()) {
				eprintln!("failed to write file dialog state: {}", e);
			}
		}
		path
	}
	
	pub fn update(&mut self, ctx: &egui::Context) {
		self.file_dialog.update(ctx);
	}
	
	//TODO: can be const once `FileDialog::state` is const
	pub fn is_closed(&self) -> bool {
		let state = self.file_dialog.state();
		!matches!(state, DialogState::Open)
	}
}
