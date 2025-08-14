mod app;
mod as_bytes;
mod boxed_slice;
mod core;
mod gfx;
mod level;
mod object_data;
mod push_get;
mod render_resources;
mod tr_traits;
mod wait;

#[cfg(target_os = "windows")]
const TASKBAR_ICON_BYTES: &[u8; 2304] = include_bytes!("res/icon24.data");
const WINDOW_ICON_BYTES: &[u8; 1024] = include_bytes!("res/icon16.data");

/// 4 MB
const GEOM_BUFFER_SIZE: usize = 4194304;

/// Round up to the nearest multiple of `N`.
const fn round_up<const N: usize>(a: usize) -> usize {
	a + (N - a % N) % N
}

fn main() {
	app::start();
}
