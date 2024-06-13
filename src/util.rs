use egui::{Color32, Ui};
use nalgebra::Const;

pub fn red_hover_button(ui: &mut Ui) {
	ui.visuals_mut().widgets.hovered.weak_bg_fill =
		Color32::from_rgb(240, 84, 84);
	ui.visuals_mut().widgets.hovered.bg_stroke.color =
		Color32::from_rgb(240, 84, 84);
	ui.visuals_mut().widgets.hovered.fg_stroke.color =
		Color32::from_rgb(10, 10, 10);
}

pub fn empty_ui(_ui: &mut Ui) {}

// {{{ modal
pub fn modal(
	egui: &egui::Context,
	title: impl Into<egui::WidgetText>,
	cond: &mut bool,
	pre_ui: impl FnOnce(&mut Ui),
	mid_ui: impl FnOnce(&mut Ui),
	confirm_callback: impl FnOnce(),
) {
	if *cond {
		egui::Window::new(title)
			.collapsible(false)
			.resizable(false)
			.anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
			.show(egui, |ui| {
				pre_ui(ui);

				ui.horizontal(|ui| {
					if ui.button("Cancel").highlight().clicked() {
						*cond = false;
					}

					mid_ui(ui);

					if ui.button("Confirm").clicked() {
						*cond = false;
						confirm_callback();
					}
				});
			});
	}
}
// }}}

// {{{ angle controller UI modifier
pub trait AngleControl {
	fn angle(self) -> Self;
}

impl AngleControl for egui::Slider<'_> {
	fn angle(self) -> Self {
		self
			.suffix("°")
			.custom_formatter(|n, _| n.to_degrees().round().to_string())
			.custom_parser(|s| s.parse().map(|n: f64| n.to_radians()).ok())
	}
}

impl AngleControl for egui::DragValue<'_> {
	fn angle(self) -> Self {
		self
			.suffix("°")
			.custom_formatter(|n, _| n.to_degrees().round().to_string())
			.custom_parser(|s| s.parse().map(|n: f64| n.to_radians()).ok())
	}
}
// }}}

// slice of nalgebra vectors or matrices -> slice of f32s
pub fn flatten_mats<T, const R: usize, const C: usize>(
	src: &[nalgebra::Matrix<
		T,
		Const<R>,
		Const<C>,
		nalgebra::ArrayStorage<T, R, C>,
	>],
) -> &[T] {
	unsafe {
		let ptr = src.as_ptr() as *const T;
		std::slice::from_raw_parts(ptr, src.len() * R * C)
	}
}

pub trait Reset {
	fn reset(&mut self) where Self: Default {
		*self = Self::default();
	}
}
