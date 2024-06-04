use egui::Slider;

use crate::util::AngleControl;

#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Settings {
	pub world: WorldSettings,
	pub render: RenderSettings,

	#[serde(skip)]
	data_modal: bool,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct WorldSettings {
	pub sun_size: f32,
	pub sun_warmth: f32,
	pub sun_strength: f32,
	pub sun_rotation: f32,
	pub sun_elevation: f32,
	pub sky_color: [f32; 3],
}

impl Default for WorldSettings {
	fn default() -> Self {
		Self {
			sun_size: 1.0,
			sun_warmth: 5200.0,
			sun_strength: 1.0,
			sun_rotation: 45.0_f32.to_radians(),
			sun_elevation: 45.0_f32.to_radians(),
			sky_color: [0.0, 0.0, 0.0],
		}
	}
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct RenderSettings {
	pub fov: f32,
	pub denoise: bool,
	pub lighting: bool,
	pub lock_camera: bool,
	pub max_bounces: u32,
	pub render_scale: u32,
}

impl Default for RenderSettings {
	fn default() -> Self {
		Self {
			fov: crate::camera::DEFAULT_FOV_DEG.to_radians(),
			denoise: true,
			lighting: false,
			lock_camera: false,
			max_bounces: 5,
			render_scale: 1,
		}
	}
}

#[derive(Clone, Copy)]
pub struct SettingsResponse {
	pub focused: bool,
	pub clear_data: bool,
	pub screenshot: bool,
}

impl Settings {
	pub fn window(
		&mut self,
		egui: &egui::Context,
		frame_index: u32,
	) -> SettingsResponse {
		let mut focused = false;
		let mut clear_data = false;
		let mut screenshot = false;

		egui::Window::new("Settings")
			.movable(false)
			.resizable(false)
			.show(egui, |ui| {
				// {{{ performance stats
				let frametime = ui.input(|i| i.unstable_dt);
				ui.label(format!(
					"Frametime: {:.4}ms ({} FPS)",
					(frametime * 1000.0),
					(1.0 / frametime).round(),
				));
				// }}}

				// {{{ world settings
				ui.collapsing("World settings", |ui| {
					ui.horizontal(|ui| {
						ui.label("Background color:");
						focused |= ui
							.color_edit_button_rgb(&mut self.world.sky_color)
							.has_focus();
					});

					ui.horizontal(|ui| {
						ui.label("Sun warmth:");
						focused |= ui
							.add(
								Slider::new(&mut self.world.sun_warmth, 1200.0..=12000.0)
									.suffix("K"),
							)
							.has_focus();
					});

					ui.horizontal(|ui| {
						ui.label("Sun strength:");
						focused |= ui
							.add(Slider::new(&mut self.world.sun_strength, 0.0..=10.0))
							.has_focus();
					});

					ui.horizontal(|ui| {
						ui.label("Sun elevation:");
						focused |= ui.drag_angle(&mut self.world.sun_elevation).has_focus();
					});

					ui.horizontal(|ui| {
						ui.label("Sun rotation:");
						focused |= ui.drag_angle(&mut self.world.sun_rotation).has_focus();
					});
				});
				// }}}

				// {{{ render settings
				ui.collapsing("Render settings", |ui| {
					ui.horizontal(|ui| {
						ui.checkbox(&mut self.render.lighting, "Realistic lighting");
						if self.render.lighting {
							ui.label(format!("(sample {frame_index})"));
						}
					});

					if self.render.lighting {
						ui.checkbox(&mut self.render.denoise, "Denoising");
					}

					ui.checkbox(
						&mut self.render.lock_camera,
						"Lock camera (useful when rendering)",
					);

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
								.angle(),
							)
							.has_focus();
					});
				});
				// }}}

				// take screenshot button
				if ui.button("Save screenshot").clicked() {
					screenshot = true;
				}

				// {{{ clear data button
				if ui.button("Clear all data").clicked() {
					self.data_modal = true;
				}

				crate::util::modal(
					egui,
					"Clear all data?",
					&mut self.data_modal,
					|ui| {
						ui.label("This will delete:");
						ui.label("- Scene objects and associated materials");
						ui.label("- Camera parameters");
						ui.label("- Saved settings");
					},
					crate::util::red_hover_button,
					|| {
						clear_data = true;
					},
				);
				// }}}
			});

		SettingsResponse {
			focused,
			clear_data,
			screenshot,
		}
	}
}
