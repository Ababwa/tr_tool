use std::fmt::{Debug, Display};

pub trait PrintError {
	fn print_err<M: Display>(self, msg: M);
}

impl<E: Display> PrintError for Result<(), E> {
	fn print_err<M: Display>(self, msg: M) {
		if let Err(e) = self {
			eprintln!("{}: {}", msg, e);
		}
	}
}

pub trait PrintDebug {
	fn print_err_dbg<M: Display>(self, msg: M);
}

impl<E: Debug> PrintDebug for Result<(), E> {
	fn print_err_dbg<M: Display>(self, msg: M) {
		if let Err(e) = self {
			eprintln!("{}: {:?}", msg, e);
		}
	}
}
