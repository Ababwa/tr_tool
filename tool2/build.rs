use winresource::WindowsResource;

fn main() {
	if cfg!(target_os = "windows") {
		WindowsResource::new().set_icon("icon16.ico").compile().expect("compile with icon");
	}
}
