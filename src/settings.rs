#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Settings {
	pub sky_color: [f32; 3],
	pub max_bounces: u32,
}

impl Settings {
	pub fn window(&mut self, egui: &egui::Context) {
		egui::Window::new("Settings").show(egui, |ui| {
			ui.horizontal(|ui| {
				ui.label("Sky color:");
				ui.color_edit_button_rgb(&mut self.sky_color);
			});

			ui.horizontal(|ui| {
				ui.label("Max ray bounces:");
				ui.add(egui::Slider::new(&mut self.max_bounces, 0..=10));
			});
		});
	}
}
