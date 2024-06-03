use egui::Color32;

pub fn red_hover_button(ui: &mut egui::Ui) {
	ui.visuals_mut().widgets.hovered.weak_bg_fill =
		Color32::from_rgb(240, 84, 84);
	ui.visuals_mut().widgets.hovered.bg_stroke.color =
		Color32::from_rgb(240, 84, 84);
	ui.visuals_mut().widgets.hovered.fg_stroke.color =
		Color32::from_rgb(10, 10, 10);
}

// TODO: get all that modal code and abstract it away here
