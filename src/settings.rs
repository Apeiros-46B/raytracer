use std::f32::consts::PI;

use egui::Slider;

#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Settings {
	pub sky: SkySettings,
	pub render: RenderSettings,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SkySettings {
	pub sun_size: f32,
	pub sun_warmth: f32,
	pub sun_strength: f32,
	pub sun_rotation: f32,
	pub sun_elevation: f32,
	pub background_color: [f32; 3],
}

impl Default for SkySettings {
	fn default() -> Self {
		Self {
			sun_size: 1.0,
			sun_warmth: 5200.0,
			sun_strength: 1.0,
			sun_rotation: 45.0_f32.to_radians(),
			sun_elevation: 45.0_f32.to_radians(),
			background_color: [0.0, 0.0, 0.0],
		}
	}
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct RenderSettings {
	pub fov: f32,
	pub gizmos: bool,
	pub denoise: bool,
	pub max_bounces: u32,
	pub render_scale: u32,
}

impl Default for RenderSettings {
	fn default() -> Self {
		Self {
			fov: 80.0_f32.to_radians(),
			gizmos: true,
			denoise: true,
			max_bounces: 5,
			render_scale: 1,
		}
	}
}

// #[derive(serde::Deserialize, serde::Serialize)]
// #[serde(default)]
// pub struct Camera {
// 	proj: [f32; 16],
// 	inverse_proj: [f32; 16],
// 	view: [f32; 16],
// 	inverse_view: [f32; 16],
// }

// impl Default for Camera {
// 	fn default() -> Self {
// 		Self::new(80.0, near_clip, far_clip)
// 	}
// }

// impl Camera {
// 	fn new(v_fov: f32, near_clip: f32, far_clip: f32) -> Self {
// 		Self {

// 		}
// 	}
// }

impl Settings {
	// return: whether or not typing widgets are focused
	pub fn window(&mut self, egui: &egui::Context) -> bool {
		let mut focused = false;

		egui::Window::new("Settings").show(egui, |ui| {
			let frametime = ui.input(|i| i.unstable_dt);
			ui.label(format!(
				"Frametime: {}ms ({} FPS)",
				(frametime * 1000.0),
				(1.0 / frametime).round(),
			));

			ui.collapsing("World settings", |ui| {
				ui.horizontal(|ui| {
					ui.label("Background color:");
					focused |= ui
						.color_edit_button_rgb(&mut self.sky.background_color)
						.has_focus();
				});

				ui.horizontal(|ui| {
					ui.label("Sun warmth:");
					focused |= ui
						.add(
							Slider::new(&mut self.sky.sun_warmth, 1200.0..=12000.0)
								.suffix("K"),
						)
						.has_focus();
				});

				ui.horizontal(|ui| {
					ui.label("Sun strength:");
					focused |= ui
						.add(Slider::new(&mut self.sky.sun_strength, 0.0..=10.0))
						.has_focus();
				});

				ui.horizontal(|ui| {
					ui.label("Sun elevation:");
					focused |= ui.drag_angle(&mut self.sky.sun_elevation).has_focus();
				});

				ui.horizontal(|ui| {
					ui.label("Sun rotation:");
					focused |= ui.drag_angle(&mut self.sky.sun_rotation).has_focus();
				});
			});

			ui.collapsing("Render settings", |ui| {
				ui.checkbox(&mut self.render.gizmos, "Show gizmos");
				ui.checkbox(&mut self.render.denoise, "Denoising");

				ui.horizontal(|ui| {
					ui.label("Max ray bounces:");
					focused |= ui
						.add(Slider::new(&mut self.render.max_bounces, 1..=10))
						.has_focus();
				});

				ui.horizontal(|ui| {
					ui.label("Field of view:");
					focused |= ui
						.add(
							Slider::new(
								&mut self.render.fov,
								(50.0_f32.to_radians())..=(120.0_f32.to_radians()),
							)
							.suffix("°")
							.custom_formatter(|n, _| n.to_degrees().round().to_string())
							.custom_parser(|s| s.parse().map(|n: f64| n.to_radians()).ok()),
						)
						.has_focus();
				});
			});
		});

		focused
	}
}
