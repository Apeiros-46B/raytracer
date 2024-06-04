use egui::Slider;

use crate::util::{AngleControl, DataResponse};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Settings {
	pub world: WorldSettings,
	pub render: RenderSettings,

	#[serde(skip)]
	pub response: SettingsResponse,

	#[serde(skip)]
	data_modal: bool,
}

impl Default for Settings {
	fn default() -> Self {
		Self {
			world: WorldSettings::default(),
			render: RenderSettings::default(),
			response: Self::first_response(),
			data_modal: false,
		}
	}
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

#[derive(Clone, Copy, Default)]
pub struct SettingsResponse {
	pub focused: bool,
	pub clear_data: bool,
	pub screenshot: bool,
	pub sun_angle_changed: bool,
}

impl DataResponse<SettingsResponse> for Settings {
	fn first_response() -> SettingsResponse {
		// sun angle needs to be calculated once first
		// but should NOT be recalculated every frame
		SettingsResponse {
			focused: false,
			clear_data: false,
			screenshot: false,
			sun_angle_changed: true,
		}
	}

	fn reset_response(&mut self) {
		self.response = SettingsResponse::default();
	}
}

impl Settings {
	pub fn window(&mut self, egui: &egui::Context, frame_index: u32) {
		egui::Window::new("Settings")
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
						self.response.focused |= ui
							.color_edit_button_rgb(&mut self.world.sky_color)
							.has_focus();
					});

					ui.horizontal(|ui| {
						ui.label("Sun warmth:");
						self.response.focused |= ui
							.add(
								Slider::new(&mut self.world.sun_warmth, 1200.0..=12000.0)
									.suffix("K"),
							)
							.has_focus();
					});

					ui.horizontal(|ui| {
						ui.label("Sun strength:");
						self.response.focused |= ui
							.add(Slider::new(&mut self.world.sun_strength, 0.0..=10.0))
							.has_focus();
					});

					ui.horizontal(|ui| {
						ui.label("Sun elevation:");
						let angle = ui.drag_angle(&mut self.world.sun_elevation);
						self.response.focused |= angle.has_focus();
						self.response.sun_angle_changed |= angle.changed();
					});

					ui.horizontal(|ui| {
						ui.label("Sun rotation:");
						let angle = ui.drag_angle(&mut self.world.sun_rotation);
						self.response.focused |= angle.has_focus();
						self.response.sun_angle_changed |= angle.changed();
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
						self.response.focused |= ui
							.add(Slider::new(&mut self.render.max_bounces, 1..=10))
							.has_focus();
					});

					ui.horizontal(|ui| {
						ui.label("Field of view:");
						self.response.focused |= ui
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

				if ui.button("Temporarily hide windows").clicked() {
					self.response.screenshot = true;
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
						self.response.clear_data = true;
					},
				);
				// }}}
			});
	}
}
