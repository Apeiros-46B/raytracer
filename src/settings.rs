use egui::Slider;

use crate::util::{AngleControl, Reset, UpdateResponse};

// {{{ state
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Settings {
	pub world: WorldSettings,
	pub render: RenderSettings,

	#[serde(skip)]
	pub response: SettingsResponse,

	#[serde(skip)]
	data_modal: bool,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct WorldSettings {
	pub sun_size: f32,
	pub sun_strength: f32,
	pub sun_rotation: f32,
	pub sun_elevation: f32,
	pub sun_color: [f32; 3],
	pub sky_color: [f32; 3],
}

impl Default for WorldSettings {
	fn default() -> Self {
		Self {
			sun_size: 1.0,
			sun_strength: 1.0,
			sun_rotation: 45.0_f32.to_radians(),
			sun_elevation: 45.0_f32.to_radians(),
			sun_color: [1.0, 1.0, 1.0],
			sky_color: [0.6, 0.6, 0.6],
		}
	}
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct RenderSettings {
	pub fov: f32,
	pub mode: RenderMode,
	pub accumulate: bool,
	pub samples_per_frame: u32,
	pub highlight: bool,
	pub lock_camera: bool,
	pub max_bounces: u32,
}

impl Default for RenderSettings {
	fn default() -> Self {
		Self {
			fov: crate::camera::DEFAULT_FOV_DEG.to_radians(),
			mode: RenderMode::default(),
			accumulate: true,
			samples_per_frame: 1,
			highlight: false,
			lock_camera: false,
			max_bounces: 5,
		}
	}
}

#[derive(
	Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
#[repr(u32)]
pub enum RenderMode {
	Preview = 0,
	#[default]
	Realistic = 1,
	Position = 2,
	Normal = 3,
	Depth = 4,
	Fresnel = 5,
	Roughness = 6,
	RayDir = 7,
	Noise = 8,
}

impl std::fmt::Display for RenderMode {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Preview => write!(f, "Preview shading"),
			Self::Realistic => write!(f, "Realistic shading"),
			Self::Position => write!(f, "Position (debug)"),
			Self::Normal => write!(f, "Normal (debug)"),
			Self::Depth => write!(f, "Distance (debug)"),
			Self::Fresnel => write!(f, "Fresnel (debug)"),
			Self::Roughness => write!(f, "Roughness (debug)"),
			Self::RayDir => write!(f, "Ray direction (debug)"),
			Self::Noise => write!(f, "Noise (debug)"),
		}
	}
}
// }}}

// {{{ response
#[derive(Clone, Copy)]
pub struct SettingsResponse {
	pub focused: bool,
	pub screenshot: bool,
	pub save_data: bool,
	pub clear_data: bool,

	pub changed: bool,
}

impl Default for SettingsResponse {
	fn default() -> Self {
		Self {
			focused: false,
			screenshot: false,
			save_data: false,
			clear_data: false,
			changed: true,
		}
	}
}

impl Reset for SettingsResponse {
	fn reset_state() -> Self {
		Self {
			changed: false,
			..Default::default()
		}
	}
}
// }}}

impl Settings {
	pub fn window(&mut self, egui: &egui::Context, frame_index: u32) {
		egui::Window::new("Settings").show(egui, |ui| {
			// {{{ performance stats
			let frametime = ui.input(|i| i.unstable_dt);
			ui.horizontal(|ui| {
				ui.label(format!(
					"Frametime: {:.4}ms ({} FPS)",
					(frametime * 1000.0),
					(1.0 / frametime).round(),
				));

				if self.render.mode == RenderMode::Realistic && self.render.accumulate {
					ui.label(format!("(sample {})", frame_index * self.render.samples_per_frame));
				}
			});
			// }}}

			// {{{ world settings
			ui.collapsing("World settings", |ui| {
				ui.horizontal(|ui| {
					ui.label("Sky color:");
					let color = ui.color_edit_button_rgb(&mut self.world.sky_color);
					self.update_response(color);
				});

				ui.horizontal(|ui| {
					ui.label("Sun color:");
					let color = ui.color_edit_button_rgb(&mut self.world.sun_color);
					self.update_response(color);
				});

				ui.horizontal(|ui| {
					ui.label("Sun strength:");
					let slider =
						ui.add(Slider::new(&mut self.world.sun_strength, 0.0..=10.0));
					self.update_response(slider);
				});

				ui.horizontal(|ui| {
					ui.label("Sun elevation:");
					let angle = ui.drag_angle(&mut self.world.sun_elevation);
					self.update_response(angle);
				});

				ui.horizontal(|ui| {
					ui.label("Sun rotation:");
					let angle = ui.drag_angle(&mut self.world.sun_rotation);
					self.update_response(angle);
				});
			});
			// }}}

			// {{{ render settings
			ui.collapsing("Render settings", |ui| {
				ui.horizontal(|ui| {
					ui.label("Render mode:");
					// {{{ select render mode
					egui::ComboBox::new("render_mode_selector", "")
						.selected_text(format!("{}", self.render.mode))
						.show_ui(
							ui,
							crate::selectable_values! {
								target = self.render.mode,
								focused = self.response.focused,
								changed = self.response.changed,
								[
									RenderMode::Preview,
									RenderMode::Realistic,
									RenderMode::Position,
									RenderMode::Normal,
									RenderMode::Depth,
									RenderMode::Fresnel,
									RenderMode::Roughness,
									RenderMode::RayDir,
									RenderMode::Noise,
								],
							},
						);
				});
				// }}}

				{
					let checkbox =
						ui.checkbox(&mut self.render.accumulate, "Accumulate samples");
					self.update_response(checkbox);
				}

				ui.horizontal(|ui| {
					ui.label("Samples per frame");
					let slider = ui.add(Slider::new(&mut self.render.samples_per_frame, 1..=32));
					self.update_response(slider);
				});

				ui.horizontal(|ui| {
					ui.label("Max ray bounces:");
					let slider = ui.add(Slider::new(&mut self.render.max_bounces, 0..=20));
					self.update_response(slider);
				});

				{
					let checkbox =
						ui.checkbox(&mut self.render.highlight, "Highlight selected object");
					self.update_response(checkbox);
				}

				{
					let checkbox = ui.checkbox(
						&mut self.render.lock_camera,
						"Lock camera (useful when rendering)",
					);
					self.update_response(checkbox);
				}

				ui.horizontal(|ui| {
					ui.label("Field of view:");
					let slider = ui.add(
						Slider::new(
							&mut self.render.fov,
							(50.0_f32.to_radians())..=(120.0_f32.to_radians()),
						)
						.angle(),
					);
					self.update_response(slider);
				});
			});
			// }}}

			if ui.button("Temporarily hide windows").clicked() {
				self.response.screenshot = true;
			}

			if ui.button("Manually save data").clicked() {
				self.response.save_data = true;
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

impl UpdateResponse for Settings {
	fn set_focused(&mut self, focused: bool) {
		self.response.focused |= focused;
	}

	fn set_changed(&mut self, changed: bool) {
		self.response.changed |= changed;
	}
}
