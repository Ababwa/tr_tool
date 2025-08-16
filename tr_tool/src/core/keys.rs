use winit::keyboard::KeyCode;

const STATE_BYTES: usize = 32;//`KeyCode` has 256 possible values, 32 * 8 = 256 bits.

/// Space and time-efficient key state tracker.
pub struct KeyStates {
	bytes: [u8; STATE_BYTES],
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
	
	pub const fn any(&self, key_codes: &[KeyCode]) -> bool {
		let mut index = 0;
		while index < key_codes.len() {
			if self.get(key_codes[index]) {
				return true;
			}
			index += 1;
		}
		false
	}
}
