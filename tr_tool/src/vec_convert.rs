pub trait VecConvert<T> {
	fn vc(self) -> T;
}

impl VecConvert<egui::Vec2> for glam::Vec2 {
	fn vc(self) -> egui::Vec2 {
		let glam::Vec2 { x, y } = self;
		egui::Vec2 { x, y }
	}
}

impl VecConvert<egui::Pos2> for glam::Vec2 {
	fn vc(self) -> egui::Pos2 {
		let glam::Vec2 { x, y } = self;
		egui::Pos2 { x, y }
	}
}
