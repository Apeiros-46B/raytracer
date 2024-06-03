// #[repr(u32)]
// pub enum IntersectionType {
// 	Spheroid,
// 	Box,
// }

// #[repr(C)]
// pub struct Object {
// 	ty: IntersectionType,
// 	hole: bool,
// 	scale: [f32; 3],
// 	rotation: [f32; 3],
// 	pos: [f32; 3],
// }

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Scene {
	selected: usize,
	pub names: Vec<String>,
	pub radii: Vec<f32>,
	pub pos: Vec<f32>, // 3 times the length
}

impl Default for Scene {
	fn default() -> Self {
		Self {
			selected: 0,
			names: vec!["Sphere".to_string(), "Sphere 2".to_string()],
			radii: vec![0.5, 0.3],
			pos: vec![0.0, 0.0, 0.0, 0.0, 1.0, 1.0],
		}
	}
}

impl Scene {
	pub fn window(&mut self, egui: &egui::Context) {
		egui::Window::new("Scene").show(egui, |ui| {
			egui::ComboBox::new("scene_object_selector", "")
				.selected_text(if self.names.is_empty() {
					"No objects"
				} else {
					&self.names[self.selected]
				})
				.show_ui(ui, |ui| {
					// see here https://stackoverflow.com/questions/72157429/egui-combobox-vector-for-selected
					for i in 0..self.names.len() {
						let value = ui.selectable_value(
							&mut &self.names[i],
							&self.names[self.selected],
							&self.names[i],
						);
						if value.clicked() {
							self.selected = i;
						}
					}
				})
		});
	}
}
