use std::fmt::{Display, Formatter};

use egui::{ComboBox, DragValue, Slider, Ui};
use glm::{identity, inverse, vec3, Mat4, Vec3};
use nalgebra_glm as glm;

use crate::{
	selectable_values,
	util::{modal, AngleControl, Reset, UpdateResponse},
};

// {{{ state
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Scene {
	pub selected: usize,

	// object properties
	pub name: Vec<String>,
	pub ty: Vec<ObjectType>,
	pub position: Vec<Vec3>,
	pub rotation: Vec<Vec3>,
	pub scale: Vec<Vec3>,

	// object material properties
	pub mat_ty: Vec<MaterialType>,
	pub mat_color: Vec<[f32; 3]>,
	pub mat_roughness: Vec<f32>,
	pub mat_emissive_strength: Vec<f32>,
	pub mat_transmissive_opacity: Vec<f32>,
	pub mat_transmissive_ior: Vec<f32>,

	// cached object transforms
	pub transform: Vec<Mat4>,
	pub inv_transform: Vec<Mat4>,
	pub normal_transform: Vec<Mat4>,

	#[serde(skip)]
	pub response: SceneResponse,

	rename_modal: bool,
	delete_modal: bool,
	pending_rename: String,
	pending_rename_selected: usize,
}

#[derive(
	Clone,
	Copy,
	Debug,
	PartialEq,
	Eq,
	bytemuck::NoUninit,
	serde::Serialize,
	serde::Deserialize,
)]
#[repr(u32)]
pub enum ObjectType {
	Sphere = 0,
	Box = 1,
}

impl Display for ObjectType {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{self:?}")
	}
}

#[derive(
	Clone,
	Copy,
	Debug,
	PartialEq,
	Eq,
	bytemuck::NoUninit,
	serde::Serialize,
	serde::Deserialize,
)]
#[repr(u32)]
pub enum MaterialType {
	Solid = 0,
	Emissive = 1,
	Transmissive = 2,
}

impl Display for MaterialType {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			MaterialType::Solid => write!(f, "Solid"),
			MaterialType::Emissive => write!(f, "Light source"),
			MaterialType::Transmissive => write!(f, "Glass"),
		}
	}
}
// }}}

// {{{ response
#[derive(Clone, Copy)]
pub struct SceneResponse {
	pub focused: bool,
	pub changed: bool,
}

impl Default for SceneResponse {
	fn default() -> Self {
		Self {
			focused: false,
			changed: true,
		}
	}
}

impl Reset for SceneResponse {
	fn reset_state() -> Self {
		Self {
			changed: false,
			..Default::default()
		}
	}
}
// }}}

// {{{ generate transformation UI functions
macro_rules! transform_ui_for {
	($prop:ident) => {
		paste::paste! {
			fn [<transform_ $prop>](
				&mut self,
				ui: &mut Ui,
				label: &'static str,
				speed: f64,
				obj_changed: &mut bool,
				extra: impl Fn(DragValue) -> DragValue,
			) {
				ui.label(label);
				ui.horizontal(|ui| {
					for (i, axis) in (0..3).zip("XYZ".chars()) {
						let drag = ui.add(extra(
							DragValue::new(&mut self.$prop[self.selected].as_mut_slice()[i])
								.prefix(format!("{axis}: "))
								.speed(speed),
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
		self.name.len()
	}

	pub fn window(&mut self, egui: &egui::Context) {
		egui::Window::new("Scene").show(egui, |ui| {
			let modal_open = self.rename_modal || self.delete_modal;

			self.object_management_interface(ui, modal_open);

			if self.len() > 0 {
				if ui.button("Duplicate").clicked() {
					self.duplicate_object();
				}

				ui.separator();

				self.object_type_menu(ui);
				self.object_renaming_button(egui, ui, modal_open);
				self.object_deletion_button(egui, ui, modal_open);
				self.transformation_interface(ui);
				self.material_interface(ui);
			}
		});
	}

	// {{{ select and add
	fn object_management_interface(&mut self, ui: &mut Ui, modal_open: bool) {
		if self.len() > 0 {
			ui.horizontal(|ui| {
				ui.label("Select object:");
				ComboBox::new("scene_object_selector", "")
					.selected_text(&self.name[self.selected])
					.show_ui(ui, |ui| {
						for i in 0..self.len() {
							let value =
								ui.selectable_value(&mut &self.selected, &i, &self.name[i]);
							if !modal_open && value.clicked() {
								self.selected = i;
							}
							self.update_response(value);
						}
					});
			});
		}

		if ui.button("New object").clicked() {
			self.new_object();
			self.set_changed(true);
		}
	}
	// }}}

	// {{{ setting type
	fn object_type_menu(&mut self, ui: &mut Ui) {
		ui.horizontal(|ui| {
			ui.label("Object type:");
			ComboBox::new("scene_object_type_selector", "")
				.selected_text(format!("{:?}", self.ty[self.selected]))
				.show_ui(
					ui,
					selectable_values! {
						target = self.ty[self.selected],
						focused = self.response.focused,
						changed = self.response.changed,
						[
							ObjectType::Sphere,
							ObjectType::Box,
						]
					},
				);
		});
	}
	// }}}

	// {{{ renaming
	fn object_renaming_button(
		&mut self,
		egui: &egui::Context,
		ui: &mut Ui,
		modal_open: bool,
	) {
		if ui.button("Rename").clicked() && !modal_open {
			self.rename_modal = true;
			self.pending_rename.clone_from(&self.name[self.selected]);
		}

		let mut do_rename = false;

		modal(
			egui,
			format!("Rename '{}'", &self.name[self.selected]),
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
			self.name[self.selected].clone_from(&self.pending_rename);
			self.set_changed(true);
		}
	}
	// }}}

	// {{{ deletion
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
			format!("Delete '{}'?", self.name[self.selected]),
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

	// {{{ transformation
	fn transformation_interface(&mut self, ui: &mut Ui) {
		if self.len() == 0 {
			return;
		}

		ui.collapsing("Transform", |ui| {
			let speed = ui.input(|i| if i.modifiers.shift { 0.01 } else { 0.1 });

			let mut changed = false;

			self.transform_position(ui, "Position", speed, &mut changed, |drag| drag);
			self.transform_rotation(ui, "Rotation", speed, &mut changed, |drag| {
				drag.angle()
			});
			self
				.transform_scale(ui, "Scale", speed, &mut changed, |drag| drag.suffix("×"));

			if changed {
				self.recalc_transforms();
				self.set_changed(true);
			}
		});
	}

	transform_ui_for!(position);
	transform_ui_for!(rotation);
	transform_ui_for!(scale);
	// }}}

	// {{{ material
	fn material_interface(&mut self, ui: &mut Ui) {
		if self.len() == 0 {
			return;
		}

		ui.collapsing("Material", |ui| {
			// {{{ select material type
			ui.horizontal(|ui| {
				ui.label("Material type:");

				ComboBox::new("scene_material_type_selector", "")
					.selected_text(format!("{}", self.mat_ty[self.selected]))
					.show_ui(
						ui,
						selectable_values! {
							target = self.mat_ty[self.selected],
							focused = self.response.focused,
							changed = self.response.changed,
							[
								MaterialType::Solid,
								MaterialType::Emissive,
								MaterialType::Transmissive,
							],
						},
					);
			});
			// }}}

			ui.horizontal(|ui| {
				ui.label("Color:");
				let color = ui.color_edit_button_rgb(&mut self.mat_color[self.selected]);
				self.update_response(color);
			});

			match self.mat_ty[self.selected] {
				MaterialType::Solid => {
					self.roughness_slider(ui);
				},
				MaterialType::Emissive => {
					ui.horizontal(|ui| {
						ui.label("Light strength:");
						let slider = ui.add(Slider::new(
							&mut self.mat_emissive_strength[self.selected],
							0.0..=10.0,
						));
						self.update_response(slider);
					});
				},
				MaterialType::Transmissive => {
					self.roughness_slider(ui);
					ui.horizontal(|ui| {
						ui.label("Opacity:");
						let slider = ui.add(Slider::new(
							&mut self.mat_transmissive_opacity[self.selected],
							0.0..=1.0,
						));
						self.update_response(slider);
					});
					ui.horizontal(|ui| {
						ui.label("Index of refraction:");
						let slider = ui.add(Slider::new(
							&mut self.mat_transmissive_ior[self.selected],
							0.0..=10.0,
						));
						self.update_response(slider);
					});
				},
			}
		});
	}

	fn roughness_slider(&mut self, ui: &mut Ui) {
		ui.horizontal(|ui| {
			ui.label("Roughness:");
			let slider = ui.add(Slider::new(
				&mut self.mat_roughness[self.selected],
				0.0..=1.0,
			));
			self.update_response(slider);
		});
	}
	// }}}

	// {{{ create, duplicate, and delete objects
	pub fn new_object(&mut self) {
		if self.len() >= 50 {
			return;
		}

		let ty = ObjectType::Sphere;

		self.name.push(format!("{ty:?}"));
		self.ty.push(ty);
		self.position.push(vec3(0.0, 0.0, 0.0));
		self.rotation.push(vec3(0.0, 0.0, 0.0));
		self.scale.push(vec3(1.0, 1.0, 1.0));

		self.mat_ty.push(MaterialType::Solid);
		self.mat_color.push([0.9, 0.9, 0.9]);
		self.mat_roughness.push(0.5);
		self.mat_emissive_strength.push(1.0);
		self.mat_transmissive_ior.push(1.333);
		self.mat_transmissive_opacity.push(0.1);

		self.transform.push(glm::identity());
		self.inv_transform.push(glm::identity());
		self.normal_transform.push(glm::identity());

		self.selected = self.len() - 1;
	}

	pub fn duplicate_object(&mut self) {
		if self.len() < 1 || self.len() >= 50 {
			return;
		}

		let i = self.selected;
		let mut name = self.name[i].clone();
		name.push_str(" copy");

		self.name.push(name);
		self.ty.push(self.ty[i]);
		self.position.push(self.position[i]);
		self.rotation.push(self.rotation[i]);
		self.scale.push(self.scale[i]);

		self.mat_ty.push(self.mat_ty[i]);
		self.mat_color.push(self.mat_color[i]);
		self.mat_roughness.push(self.mat_roughness[i]);
		self
			.mat_emissive_strength
			.push(self.mat_emissive_strength[i]);
		self.mat_transmissive_ior.push(self.mat_transmissive_ior[i]);
		self
			.mat_transmissive_opacity
			.push(self.mat_transmissive_opacity[i]);

		self.transform.push(self.transform[i]);
		self.inv_transform.push(self.inv_transform[i]);
		self.normal_transform.push(self.normal_transform[i]);

		self.selected = self.len() - 1;
	}

	pub fn delete_object(&mut self) {
		if self.len() < 1 {
			return;
		}

		let i = self.selected;

		self.name.remove(i);
		self.ty.remove(i);
		self.position.remove(i);
		self.rotation.remove(i);
		self.scale.remove(i);

		self.mat_ty.remove(i);
		self.mat_color.remove(i);
		self.mat_roughness.remove(i);
		self.mat_emissive_strength.remove(i);
		self.mat_transmissive_ior.remove(i);
		self.mat_transmissive_opacity.remove(i);

		self.transform.remove(i);
		self.inv_transform.remove(i);
		self.normal_transform.remove(i);

		self.selected = i.saturating_sub(1);
	}
	// }}}

	fn recalc_transforms(&mut self) {
		for i in 0..self.len() {
			let pos = glm::translate(&identity(), &self.position[i]);

			let mut rot = identity();
			rot = glm::rotate_z(&rot, self.rotation[i].z);
			rot = glm::rotate_y(&rot, self.rotation[i].y);
			rot = glm::rotate_x(&rot, self.rotation[i].x);

			let scl = glm::scale(&identity(), &self.scale[i]);

			// rightmost transforms are applied first
			// (due to how matrix multiplication works)
			let mat = pos * rot * scl;

			self.transform[i] = mat;
			self.inv_transform[i] = inverse(&mat);

			// normals are transformed:
			// - without translation
			// - with rotation
			// - with inverted scale (reciprocal of scale factors)
			self.normal_transform[i] = rot * inverse(&scl);
		}
	}

	pub fn with_default_scene(mut self) -> Self {
		self.new_object();

		// pretty rough metallic sphere
		self.mat_roughness[self.selected] = 0.6;

		self.new_object();

		// dark matte floor
		self.name[self.selected] = "Floor".to_string();
		self.ty[self.selected] = ObjectType::Box;
		self.position[self.selected] = vec3(0.0, -1.001, 0.0);
		self.rotation[self.selected] = vec3(0.0, 0.0, 0.0);
		self.scale[self.selected] = vec3(1000.0, 0.001, 1000.0);

		self.mat_color[self.selected] = [0.1, 0.1, 0.1];
		self.mat_roughness[self.selected] = 1.0;

		self.recalc_transforms();

		self
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
