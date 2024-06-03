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

	pending_rename: Option<String>,
	delete_confirmation: bool,
}

impl Default for Scene {
	fn default() -> Self {
		Self {
			selected: 0,
			names: vec!["Default sphere".to_string()],
			radii: vec![0.5],
			pos: vec![0.0, 0.0, 0.0],

			pending_rename: None,
			delete_confirmation: false,
		}
	}
}

pub struct SceneResponse {
	pub focused: bool,
}

impl Scene {
	pub fn window(&mut self, egui: &egui::Context) -> SceneResponse {
		let mut focused = false;

		egui::Window::new("Scene").show(egui, |ui| {
			// {{{ select object
			egui::ComboBox::new("scene_object_selector", "")
				.selected_text(if self.names.is_empty() {
					"No objects"
				} else {
					&self.names[self.selected]
				})
				.show_ui(ui, |ui| {
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
				});
			// }}}

			// {{{ rename objects
			if ui.button("Rename").clicked() {
				self.pending_rename = Some(self.names[self.selected].clone());
			}

			if self.pending_rename.is_some() {
				egui::Window::new("Rename object")
					.collapsible(false)
					.resizable(false)
					.anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
					.show(egui, |ui| {
						ui.label("New name:");
						focused |= ui
							.text_edit_singleline(self.pending_rename.as_mut().unwrap())
							.has_focus();

						ui.horizontal(|ui| {
							if ui.button("Cancel").highlight().clicked() {
								self.pending_rename = None;
							}

							if ui.button("Confirm").clicked() {
								self.names[self.selected] =
									self.pending_rename.as_ref().unwrap().clone();
								self.pending_rename = None;
							}
						})
					});
			}
			// }}}

			// {{{ delete object
			if ui.button("Delete").clicked() {
				self.delete_confirmation = true;
			}

			if self.delete_confirmation {
				egui::Window::new(format!("Delete '{}'", self.names[self.selected]))
					.collapsible(false)
					.resizable(false)
					.anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
					.show(egui, |ui| {
						ui.label("Delete this object?");

						ui.horizontal(|ui| {
							if ui.button("Cancel").highlight().clicked() {
								self.delete_confirmation = false;
							}

							crate::util::red_hover_button(ui);

							if ui.button("Confirm").clicked() {
								self.delete_confirmation = false;
								// TODO
							}
						})
					});
			}
			// }}}
		});

		SceneResponse { focused }
	}
}
