use std::{fs, path::PathBuf};
use egui_file_dialog::{DialogState, FileDialog};

#[derive(Clone, Copy, PartialEq, Eq)]
enum State<T> {
	SelectingLevel,
	SavingTexture(T),//index into texture_bind_group
}

pub struct FileDialogWrapper<T> {
	file_dialog: FileDialog,
	state: Option<State<T>>,
	level_dir: Option<PathBuf>,
	texture_dir: Option<PathBuf>,
}

fn read_dirs(level_dir: &mut Option<PathBuf>, texture_dir: &mut Option<PathBuf>) -> Option<()> {
	let dirs = fs::read_to_string("dir").ok()?;
	let mut dirs = dirs.lines();
	*level_dir = Some(dirs.next()?.into());
	*texture_dir = Some(dirs.next()?.into());
	Some(())
}

impl<T> FileDialogWrapper<T> {
	pub fn new() -> Self {
		let mut level_dir = None;
		let mut texture_dir = None;
		read_dirs(&mut level_dir, &mut texture_dir);
		Self {
			file_dialog: FileDialog::new(),
			state: None,
			level_dir,
			texture_dir,
		}
	}
	
	pub fn update(&mut self, ctx: &egui::Context) {
		self.file_dialog.update(ctx);
		if matches!(self.file_dialog.state(), DialogState::Closed | DialogState::Cancelled) {
			self.state = None;
		}
	}
	
	fn save_dirs(&self) {
		let [level_dir, texture_dir] = [&self.level_dir, &self.texture_dir].map(|dir| {
			dir.as_ref().map(|dir| dir.as_os_str().as_encoded_bytes()).unwrap_or_default()
		});
		if let Err(e) = fs::write("dir", [level_dir, b"\n", texture_dir].concat()) {
			eprintln!("failed to save dir: {}", e);
		}
	}
	
	fn try_initiate(&mut self, state: State<T>) {
		if self.state.is_none() {
			let (dir, fd_fn): (_, fn(&mut FileDialog)) = match state {
				State::SelectingLevel => (&self.level_dir, FileDialog::select_file),
				State::SavingTexture(_) => (&self.texture_dir, FileDialog::save_file),
			};
			if let Some(dir) = dir {
				self.file_dialog.config_mut().initial_directory = dir.clone();
			}
			self.state = Some(state);
			fd_fn(&mut self.file_dialog);
		}
	}
	
	pub fn is_closed(&self) -> bool {
		self.state.is_none()
	}
	
	pub fn select_level(&mut self) {
		self.try_initiate(State::SelectingLevel);
	}
	
	pub fn save_texture(&mut self, arg: T) {
		self.try_initiate(State::SavingTexture(arg));
	}
	
	pub fn get_level_path(&mut self) -> Option<PathBuf> {
		if let Some(State::SelectingLevel) = self.state {
			let path = self.file_dialog.take_selected()?;
			let save_path = path.parent().unwrap_or(&path);
			self.level_dir = Some(save_path.to_owned());
			self.save_dirs();
			self.state = None;
			Some(path)
		} else {
			None
		}
	}
	
	pub fn get_texture_path(&mut self) -> Option<(PathBuf, T)> {
		match self.state.take() {
			Some(State::SavingTexture(arg)) => {
				let Some(path) = self.file_dialog.take_selected() else {
					self.state = Some(State::SavingTexture(arg));
					return None;
				};
				let save_path = path.parent().unwrap_or(&path);
				self.texture_dir = Some(save_path.to_owned());
				self.save_dirs();
				self.state = None;
				Some((path, arg))
			},
			other => {
				self.state = other;
				None
			},
		}
	}
}
