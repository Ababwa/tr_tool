use anyhow::Result;
use image::{RgbaImage, Rgba};

const IMG_DIM: u32 = tr_reader::tr4_model::IMG_DIM as u32;

pub trait PixelFormat<const N: usize> {
	fn to_rgba(pixel: &[u8; N]) -> [u8; 4];
}

pub struct Pixel32;

impl PixelFormat<4> for Pixel32 {
	fn to_rgba(pixel: &[u8; 4]) -> [u8; 4] {
		let [b, g, r, a] = *pixel;
		[r, g, b, a]
	}
}

pub struct Pixel16;

impl PixelFormat<2> for Pixel16 {
	fn to_rgba(pixel: &[u8; 2]) -> [u8; 4] {
		let pixel = u16::from_le_bytes(*pixel);
		let a = (pixel >> 15) as u8 * 255;
		let r = (pixel >> 7) as u8 & 248;
		let g = (pixel >> 2) as u8 & 248;
		let b = (pixel << 3) as u8;
		[r, g, b, a]
	}
}

pub fn read_img<const N: usize, P: PixelFormat<N>>(raw: &[u8]) -> RgbaImage {
	let mut img = RgbaImage::new(IMG_DIM, IMG_DIM);
	let mut pos = 0u32;
	for pixel in raw.chunks_exact(N) {
		let pixel: &[u8; N] = pixel.try_into().unwrap();
		img.put_pixel(pos % IMG_DIM, pos / IMG_DIM, Rgba(P::to_rgba(pixel)));
		pos += 1;
	}
	img
}

pub fn save_images<const N: usize, P: PixelFormat<N>>(images: Vec<&[u8]>, prefix: &str) -> Result<()> {
	for (index, raw) in images.into_iter().enumerate() {
		read_img::<N, P>(raw).save(format!("{}_{}.png", prefix, index))?;
	}
	Ok(())
}
