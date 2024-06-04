use std::fmt::Display;

use egui::Ui;
use glm::{vec3, Vec3};
use nalgebra_glm as glm;

use crate::util::{modal, AngleControl};

#[repr(u32)]
pub enum ObjectType {
	Sphere,
	Cube,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Scene {
	selected: usize,
	pub names: Vec<String>,
	pub radii: Vec<f32>,
	pub pos: Vec<Vec3>,
	pub rot: Vec<Vec3>,
	pub scl: Vec<Vec3>,
	pub transform_mats: Vec<glm::Mat4>,

	rename_modal: bool,
	delete_modal: bool,
	pending_rename: String,
	pending_rename_selected: usize,
}

impl Default for Scene {
	fn default() -> Self {
		Self {
			selected: 0,
			names: vec!["Default sphere".to_string()],
			radii: vec![0.5],
			pos: vec![vec3(0.0, 0.0, 0.0)],
			rot: vec![vec3(0.0, 0.0, 0.0)],
			scl: vec![vec3(1.0, 1.0, 1.0)],
			transform_mats: vec![glm::identity()],

			rename_modal: false,
			delete_modal: false,
			pending_rename: "".to_string(),
			pending_rename_selected: 0,
		}
	}
}

pub struct SceneResponse {
	pub focused: bool,
}

impl Scene {
	pub fn window(&mut self, egui: &egui::Context) -> SceneResponse {
		let mut focused = false;

		egui::Window::new("Scene")
			.movable(false)
			.resizable(false)
			.anchor(egui::Align2::RIGHT_TOP, [-20.0, 20.0])
			.show(egui, |ui| {
				let modal_open = self.rename_modal || self.delete_modal;

				self.object_selection_menu(ui, modal_open);
				self.object_renaming_button(egui, ui, modal_open, &mut focused);
				self.object_deletion_button(egui, ui, modal_open);

				// {{{ position, rotation, scale
				let drag_speed =
					ui.input(|i| if i.modifiers.shift { 0.01 } else { 0.1 });

				let mut obj_modified = false;

				ui.label("\nPosition");
				ui.horizontal(|ui| {
					for (i, axis) in (0..3).zip("XYZ".chars()) {
						let value = ui.add(
							egui::DragValue::new(
								&mut self.pos[self.selected].as_mut_slice()[i],
							)
							.prefix(format!("{axis}: "))
							.speed(drag_speed),
						);
						obj_modified |= value.dragged() || value.has_focus();
					}
				});

				ui.label("\nRotation");
				ui.horizontal(|ui| {
					for (i, axis) in (0..3).zip("XYZ".chars()) {
						let value = ui.add(
							egui::DragValue::new(
								&mut self.rot[self.selected].as_mut_slice()[i],
							)
							.prefix(format!("{axis}: "))
							.speed(drag_speed)
							.angle(),
						);
						obj_modified |= value.dragged() || value.has_focus();
					}
				});

				ui.label("\nScale");
				ui.horizontal(|ui| {
					for (i, axis) in (0..3).zip("XYZ".chars()) {
						let value = ui.add(
							egui::DragValue::new(
								&mut self.scl[self.selected].as_mut_slice()[i],
							)
							.prefix(format!("{axis}: "))
							.suffix("×")
							.speed(drag_speed),
						);
						obj_modified |= value.dragged() || value.has_focus();
					}
				});

				if obj_modified {
					self.recalculate_transforms();
				}
				// }}}
			});

		SceneResponse { focused }
	}

	// {{{ object selection menu
	fn object_selection_menu(&mut self, ui: &mut Ui, modal_open: bool) {
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
					if !modal_open && value.clicked() {
						self.selected = i;
					}
				}
			});
	}
	// }}}

	// {{{ object renaming button
	fn object_renaming_button(
		&mut self,
		egui: &egui::Context,
		ui: &mut Ui,
		modal_open: bool,
		focused: &mut bool,
	) {
		if ui.button("Rename").clicked() && !modal_open {
			self.rename_modal = true;
			self.pending_rename.clone_from(&self.names[self.selected]);
		}

		let mut do_rename = false;

		modal(
			egui,
			format!("Rename '{}'", &self.names[self.selected]),
			&mut self.rename_modal,
			|ui| {
				ui.label("New name:");
				*focused |= ui
					.text_edit_singleline(&mut self.pending_rename)
					.has_focus();
			},
			crate::util::empty_ui,
			|| do_rename = true,
		);

		if do_rename {
			self.names[self.selected].clone_from(&self.pending_rename);
		}
	}
	// }}}

	// {{{ object deletion button
	fn object_deletion_button(
		&mut self,
		egui: &egui::Context,
		ui: &mut Ui,
		modal_open: bool,
	) {
		// we put the condition after because we want the button to still appear
		if ui.button("Delete").clicked() && !modal_open {
			self.delete_modal = true;
		}

		let mut do_delete = false;

		modal(
			egui,
			format!("Delete '{}'?", self.names[self.selected]),
			&mut self.delete_modal,
			|ui| {
				ui.label("Delete this object?");
			},
			crate::util::red_hover_button,
			|| do_delete = true,
		);

		if do_delete {
			// TODO delete
		}
	}
	// }}}

	pub fn recalculate_transforms(&mut self) {
		for (i, mat) in self.transform_mats.iter_mut().enumerate() {
			*mat = glm::identity();
			// in reverse order because of right-multiplying
			glm::rotate(mat, self.rot[i].x, &Vec3::x_axis());
			glm::rotate(mat, self.rot[i].y, &Vec3::y_axis());
			glm::rotate(mat, self.rot[i].z, &Vec3::z_axis());
			glm::scale(mat, &self.scl[i]);
			glm::translate(mat, &self.pos[i]);
		}
	}
}
