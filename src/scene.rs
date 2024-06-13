use egui::{DragValue, Ui};
use glm::{inverse, vec3, Mat4, Vec3};
use nalgebra_glm as glm;

use crate::util::{modal, AngleControl, Reset};

#[derive(
	Clone, Copy, bytemuck::NoUninit, serde::Serialize, serde::Deserialize,
)]
#[repr(u32)]
pub enum ObjectType {
	Sphere,
	Cube,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Scene {
	selected: usize,
	pub names: Vec<String>,
	pub types: Vec<ObjectType>,
	pub pos: Vec<Vec3>,
	pub rot: Vec<Vec3>,
	pub scl: Vec<Vec3>,

	pub transforms: Vec<Mat4>,
	pub inv_transforms: Vec<Mat4>,
	pub trans_transforms: Vec<Mat4>,
	pub rot_transforms: Vec<Mat4>,
	pub inv_rot_transforms: Vec<Mat4>,

	#[serde(skip)]
	pub response: SceneResponse,

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
			types: vec![ObjectType::Sphere],
			pos: vec![vec3(0.0, 0.0, 0.0)],
			rot: vec![vec3(0.0, 0.0, 0.0)],
			scl: vec![vec3(1.0, 1.0, 1.0)],
			transforms: vec![glm::identity()],
			inv_transforms: vec![glm::identity()],
			trans_transforms: vec![glm::identity()],
			rot_transforms: vec![glm::identity()],
			inv_rot_transforms: vec![glm::identity()],

			response: SceneResponse::default(),

			rename_modal: false,
			delete_modal: false,
			pending_rename: "".to_string(),
			pending_rename_selected: 0,
		}
	}
}

#[derive(Clone, Copy, Default)]
pub struct SceneResponse {
	pub focused: bool,
	pub changed: bool,
}
impl Reset for SceneResponse {}

// {{{ generate transformation UI functions
macro_rules! transform_ui_for {
	($prop:ident) => {
		paste::paste! {
			fn [<transform_ $prop>](
				&mut self,
				ui: &mut Ui,
				label: &'static str,
				drag_speed: f64,
				obj_changed: &mut bool,
				extra: impl Fn(DragValue) -> DragValue,
			) {
				ui.label(label);
				ui.horizontal(|ui| {
					for (i, axis) in (0..3).zip("XYZ".chars()) {
						let response = ui.add(extra(
							DragValue::new(&mut self.$prop[self.selected].as_mut_slice()[i])
								.prefix(format!("{axis}: "))
								.speed(drag_speed),
						));
						*obj_changed |= response.changed();
						self.response.focused |= response.has_focus();
					}
				});
			}
		}
	};
}
// }}}

impl Scene {
	pub fn len(&self) -> usize {
		self.names.len()
	}

	pub fn window(&mut self, egui: &egui::Context) {
		egui::Window::new("Scene")
			.resizable(false)
			.anchor(egui::Align2::RIGHT_TOP, [-16.0, 16.0])
			.show(egui, |ui| {
				let modal_open = self.rename_modal || self.delete_modal;

				self.object_selection_menu(ui, modal_open);
				self.object_renaming_button(egui, ui, modal_open);
				self.object_deletion_button(egui, ui, modal_open);
				self.transformation_interface(ui);
			});
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
				self.response.focused |= ui
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

	// {{{ position, rotation, scale
	fn transformation_interface(&mut self, ui: &mut Ui) {
		ui.collapsing("Transform", |ui| {
			let drag_speed = ui.input(|i| if i.modifiers.shift { 0.01 } else { 0.1 });

			let mut changed = false;

			self.transform_pos(ui, "Position", drag_speed, &mut changed, |drag| drag);
			self.transform_rot(ui, "Rotation", drag_speed, &mut changed, |drag| {
				drag.angle()
			});
			self.transform_scl(ui, "Scale", drag_speed, &mut changed, |drag| {
				drag.suffix("×")
			});

			if changed {
				self.recalc_transforms();
			}
		});
	}
	// }}}

	transform_ui_for!(pos);
	transform_ui_for!(rot);
	transform_ui_for!(scl);

	pub fn recalc_transforms(&mut self) {
		for i in 0..self.len() {
			let mut rot_mat = glm::identity();
			rot_mat = glm::rotate_z(&rot_mat, self.rot[i].z);
			rot_mat = glm::rotate_y(&rot_mat, self.rot[i].y);
			rot_mat = glm::rotate_x(&rot_mat, self.rot[i].x);

			let mut mat = glm::identity();
			mat = glm::translate(&mat, &self.pos[i]);
			mat *= rot_mat;
			mat = glm::scale(&mat, &self.scl[i]);

			self.transforms[i] = mat;
			self.inv_transforms[i] = inverse(&mat);
			self.trans_transforms[i] = glm::transpose(&mat);
			self.rot_transforms[i] = rot_mat;
			self.inv_rot_transforms[i] = glm::transpose(&rot_mat);
		}

		self.response.changed = true;
	}
}
