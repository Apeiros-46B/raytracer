use egui::{DragValue, Ui};
use glm::{identity, inverse, vec3, Mat4, Vec3};
use nalgebra_glm as glm;

use crate::util::{modal, AngleControl, Reset, UpdateResponse};

#[derive(
	Clone, Copy, Debug, bytemuck::NoUninit, serde::Serialize, serde::Deserialize,
)]
#[repr(u32)]
pub enum ObjectType {
	Sphere = 0,
	Box = 1,
}

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Scene {
	selected: usize,

	// object properties
	pub names: Vec<String>,
	pub types: Vec<ObjectType>,
	pub pos: Vec<Vec3>,
	pub rot: Vec<Vec3>,
	pub scl: Vec<Vec3>,

	// object material properties
	pub mat_colors: Vec<Vec3>,

	// cached object transforms
	pub transforms: Vec<Mat4>,
	pub inv_transforms: Vec<Mat4>,
	pub normal_transforms: Vec<Mat4>,

	#[serde(skip)]
	pub response: SceneResponse,

	rename_modal: bool,
	delete_modal: bool,
	pending_rename: String,
	pending_rename_selected: usize,
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
						let drag = ui.add(extra(
							DragValue::new(&mut self.$prop[self.selected].as_mut_slice()[i])
								.prefix(format!("{axis}: "))
								.speed(drag_speed),
						));
						*obj_changed |= drag.changed();
						self.update_response(drag);
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

				self.object_management_interface(ui, modal_open);

				if self.len() > 0 {
					self.object_renaming_button(egui, ui, modal_open);
					self.object_deletion_button(egui, ui, modal_open);
					self.transformation_interface(ui);
				}
			});
	}

	// {{{ object management interface
	fn object_management_interface(&mut self, ui: &mut Ui, modal_open: bool) {
		ui.horizontal(|ui| {
			ui.label("Select object:");
			egui::ComboBox::new("scene_object_selector", "")
				.selected_text(if self.names.is_empty() {
					"Scene is empty"
				} else {
					&self.names[self.selected]
				})
				.show_ui(ui, |ui| {
					for i in 0..self.len() {
						let value = ui.selectable_value(
							&mut &self.selected,
							&i,
							&self.names[i],
						);
						if !modal_open && value.clicked() {
							self.selected = i;
						}
						self.update_response(value);
					}
				});
		});

		if ui.button("New object").clicked() {
			self.new_object();
			self.set_changed(true);
		}
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
			self.set_changed(true);
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
			self.delete_object();
			self.set_changed(true);
		}
	}
	// }}}

	// {{{ position, rotation, scale
	fn transformation_interface(&mut self, ui: &mut Ui) {
		if self.len() == 0 {
			return;
		}

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
				self.set_changed(true);
			}
		});
	}
	// }}}

	transform_ui_for!(pos);
	transform_ui_for!(rot);
	transform_ui_for!(scl);

	pub fn new_object(&mut self) {
		let ty = ObjectType::Sphere;

		self.names.push(format!("Unnamed {ty:?}"));
		self.types.push(ty);
		self.pos.push(vec3(0.0, 0.0, 0.0));
		self.rot.push(vec3(0.0, 0.0, 0.0));
		self.scl.push(vec3(1.0, 1.0, 1.0));

		self.mat_colors.push(vec3(1.0, 1.0, 1.0));

		self.transforms.push(glm::identity());
		self.inv_transforms.push(glm::identity());
		self.normal_transforms.push(glm::identity());

		self.selected = self.len() - 1;
	}

	pub fn delete_object(&mut self) {
		let i = self.selected;

		self.names.remove(i);
		self.types.remove(i);
		self.pos.remove(i);
		self.rot.remove(i);
		self.scl.remove(i);

		self.mat_colors.remove(i);

		self.transforms.remove(i);
		self.inv_transforms.remove(i);
		self.normal_transforms.remove(i);

		self.selected = i.saturating_sub(1);
	}

	fn recalc_transforms(&mut self) {
		for i in 0..self.len() {
			let pos = glm::translate(&identity(), &self.pos[i]);

			let mut rot = identity();
			rot = glm::rotate_z(&rot, self.rot[i].z);
			rot = glm::rotate_y(&rot, self.rot[i].y);
			rot = glm::rotate_x(&rot, self.rot[i].x);

			let scl = glm::scale(&identity(), &self.scl[i]);

			// rightmost transforms are applied first
			// (due to how matrix multiplication works)
			let mat = pos * rot * scl;

			self.transforms[i] = mat;
			self.inv_transforms[i] = inverse(&mat);

			// normals are transformed:
			// - without translation
			// - with rotation
			// - with inverted scale (reciprocal of scale factors)
			self.normal_transforms[i] = rot * inverse(&scl);
		}
	}
}

impl UpdateResponse for Scene {
	fn set_focused(&mut self, focused: bool) {
		self.response.focused |= focused;
	}

	fn set_changed(&mut self, changed: bool) {
		self.response.changed |= changed;
	}
}
