use std::future::Future;

pub trait Wait: Future {
	fn wait(self) -> Self::Output;
}

impl<T: Future> Wait for T {
	fn wait(self) -> Self::Output {
		pollster::block_on(self)
	}
}
