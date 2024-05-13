use std::sync::Arc;

// use eframe::glow::HasContext;
use egui::mutex::Mutex;

use crate::{
	render::Raytracer,
	settings::Settings,
};

pub struct RaytracingApp {
	pub renderer: Arc<Mutex<Raytracer>>,
	pub settings: Settings,
}

impl RaytracingApp {
	pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
		let mut settings = Settings::default();

		if let Some(storage) = cc.storage {
			if let Some(value) = eframe::get_value(storage, "settings") {
				settings = value;
			}
		}

		let gl = cc.gl.as_ref().expect("failed to obtain GL context");

		// if cfg!(target_arch = "wasm32") {
		//  TODO: this check fails
		// 	assert!(gl.supported_extensions().contains("EXT_disjoint_timer_query"));
		// } else {
		// 	assert!(gl.supported_extensions().contains("GL_ARB_timer_query"));
		// }

		// remove comically large window shadow
		cc.egui_ctx.set_visuals(egui::Visuals {
			window_shadow: egui::epaint::Shadow {
				offset: egui::Vec2::splat(0.0),
				blur: 16.0,
				color: egui::Color32::from_black_alpha(64),
				..Default::default()
			},
			..Default::default()
		});

		Self {
			renderer: Arc::new(Mutex::new(Raytracer::new(gl))),
			settings,
		}
	}
}

impl eframe::App for RaytracingApp {
	// fn save(&mut self, storage: &mut dyn eframe::Storage) {
	// 	eframe::set_value(storage, eframe::APP_KEY, self);
	// }

	fn update(&mut self, egui: &egui::Context, _frame: &mut eframe::Frame) {
		self.settings.window(egui);

		egui::CentralPanel::default().show(egui, |ui| {
			self.paint(ui);
		});

		egui.request_repaint_of(egui.viewport_id());
	}

	fn on_exit(&mut self, gl: Option<&eframe::glow::Context>) {
		if let Some(gl) = gl {
			self.renderer.lock().destroy(gl);
		}
	}
}
