use winit::keyboard::KeyCode;

const KEY_GROUP_MAX: usize = 2;

///Space-efficient list of key codes
#[derive(Clone, Copy)]
pub struct KeyGroup {
	key_codes: [KeyCode; KEY_GROUP_MAX],
	len: u8,
}

impl KeyGroup {
	pub fn new(keys: &[KeyCode]) -> Self {
		let mut key_codes = [KeyCode::Backquote; KEY_GROUP_MAX];
		key_codes[..keys.len()].copy_from_slice(keys);
		Self { key_codes, len: keys.len() as u8 }
	}
	
	pub fn key_codes(&self) -> &[KeyCode] {
		&self.key_codes[..self.len as usize]
	}
}

const STATE_BYTES: usize = 32;//KeyCode has 256 possible values, 32 * 8 = 256 bits

pub struct KeyStates {
	bytes: [u8; STATE_BYTES],
}

impl KeyStates {
	pub fn new() -> Self {
		Self { bytes: [0; STATE_BYTES] }
	}
	
	pub fn get(&self, key_code: KeyCode) -> bool {
		let index = key_code as usize;
		(self.bytes[index / 8] >> (index % 8)) & 1 == 1
	}
	
	pub fn set(&mut self, key_code: KeyCode, val: bool) {
		let index = key_code as usize;
		self.bytes[index / 8] = (self.bytes[index / 8] & !(1 << (index % 8))) | ((val as u8) << (index % 8));
	}
	
	pub fn any(&self, key_group: KeyGroup) -> bool {
		key_group.key_codes().iter().any(|&key_code| self.get(key_code))
	}
}
