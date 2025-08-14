use std::slice;
use winit::keyboard::KeyCode;

const KEY_GROUP_MAX: usize = 2;
const STATE_BYTES: usize = 32;//`KeyCode` has 256 possible values, 32 * 8 = 256 bits.

/// Space-efficient list of key codes.
pub struct KeyGroup {
	len: u8,
	key_codes: [KeyCode; KEY_GROUP_MAX],
}

/// Space and time-efficient key state tracker.
pub struct KeyStates {
	bytes: [u8; STATE_BYTES],
}

impl KeyGroup {
	pub const fn new(keys: &[KeyCode]) -> Self {
		let mut key_codes = [KeyCode::Backquote; KEY_GROUP_MAX];
		let mut index = 0;
		while index < keys.len() {
			key_codes[index] = keys[index];
			index += 1;
		}
		Self {
			key_codes,
			len: keys.len() as u8,
		}
	}
	
	pub const fn key_codes(&self) -> &[KeyCode] {
		//Safety: `new` ensures `len` <= `key_codes.len()`
		unsafe {
			slice::from_raw_parts(self.key_codes.as_ptr(), self.len as usize)
		}
	}
}

impl KeyStates {
	pub const fn new() -> Self {
		Self {
			bytes: [0; STATE_BYTES],
		}
	}
	
	pub const fn get(&self, key_code: KeyCode) -> bool {
		let k = key_code as usize;
		(self.bytes[k / 8] >> (k % 8)) & 1 == 1
	}
	
	pub const fn set(&mut self, key_code: KeyCode, pressed: bool) {
		let k = key_code as usize;
		self.bytes[k / 8] = (self.bytes[k / 8] & !(1 << (k % 8))) | ((pressed as u8) << (k % 8));
	}
	
	pub const fn any(&self, key_group: &KeyGroup) -> bool {
		let group_codes = key_group.key_codes();
		let mut index = 0;
		while index < group_codes.len() {
			if self.get(group_codes[index]) {
				return true;
			}
			index += 1;
		}
		false
	}
}
