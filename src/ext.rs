use std::future::Future;
use pollster::block_on;

pub trait Wait: Future {
	fn wait(self) -> Self::Output;
}

impl<T: Future> Wait for T {
	fn wait(self) -> Self::Output {
		block_on(self)
	}
}
