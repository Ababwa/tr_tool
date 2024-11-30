#[cfg(target_endian = "big")]
const _: () = panic!("big endian not supported");

mod u16_cursor;
pub mod tr1;
pub mod tr2;
pub mod tr3;
pub mod tr4;
// pub mod tr5;

use tr1::Sprite;
use u16_cursor::U16Cursor;

pub use tr_readable::Readable;

pub(crate) unsafe fn get_room_geom<V, Q, T>(geom_data: &[u16]) -> (&[V], &[Q], &[T], &[Sprite]) {
	let mut cursor = U16Cursor::new(geom_data);
	(
		cursor.u16_len_slice(),
		cursor.u16_len_slice(),
		cursor.u16_len_slice(),
		cursor.u16_len_slice(),
	)
}
