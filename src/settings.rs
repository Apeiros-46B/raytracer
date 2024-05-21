use std::f32::consts::PI;

use egui::Slider;

#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Settings {
	pub sky: SkySettings,
	pub render: RenderSettings,
	// pub camera: Camera,
	pub sphere_x: f32,
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
			sun_rotation: PI / 4.0,
			sun_elevation: PI / 4.0,
			background_color: [0.0, 0.0, 0.0],
		}
	}
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct RenderSettings {
	pub gizmos: bool,
	pub denoise: bool,
	pub max_bounces: u32,
	pub render_scale: u32,
}

impl Default for RenderSettings {
	fn default() -> Self {
		Self {
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
	pub fn window(&mut self, egui: &egui::Context) {
		egui::Window::new("Settings").show(egui, |ui| {
			ui.collapsing("World settings", |ui| {
				ui.horizontal(|ui| {
					ui.label("Background color:");
					ui.color_edit_button_rgb(&mut self.sky.background_color);
				});

				ui.horizontal(|ui| {
					ui.label("Sun warmth:");
					ui.add(
						Slider::new(&mut self.sky.sun_warmth, 1200.0..=12000.0).suffix("K"),
					);
				});

				ui.horizontal(|ui| {
					ui.label("Sun strength:");
					ui.add(Slider::new(&mut self.sky.sun_strength, 0.0..=10.0));
				});

				ui.horizontal(|ui| {
					ui.label("Sun elevation:");
					ui.drag_angle(&mut self.sky.sun_elevation);
				});

				ui.horizontal(|ui| {
					ui.label("Sun rotation:");
					ui.drag_angle(&mut self.sky.sun_rotation);
				});

				// DEBUG
				ui.horizontal(|ui| {
					ui.label("Sphere X:");
					ui.add(egui::DragValue::new(&mut self.sphere_x).speed(0.005));
				});
			});

			ui.collapsing("Render settings", |ui| {
				ui.checkbox(&mut self.render.gizmos, "Show gizmos");
				ui.checkbox(&mut self.render.denoise, "Denoising");

				ui.horizontal(|ui| {
					ui.label("Max ray bounces:");
					ui.add(Slider::new(&mut self.render.max_bounces, 1..=10));
				});
			});
		});
	}
}
