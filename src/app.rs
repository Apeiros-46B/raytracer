use std::sync::Arc;

use egui::mutex::Mutex;
use nalgebra_glm as glm;

use crate::{
	camera::Camera, render::Raytracer, scene::Scene, settings::Settings,
};

pub struct RaytracingApp {
	pub renderer: Arc<Mutex<Raytracer>>,
	pub data: Arc<Mutex<PersistentData>>,

	default_data: PersistentData,
	screenshot_time: Option<f32>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct PersistentData {
	pub settings: Settings,
	pub camera: Camera,
	pub scene: Scene,
}

impl PersistentData {
	fn new(scr_size: glm::Vec2) -> Self {
		let mut this = Self {
			settings: Settings::default(),
			camera: Camera::new(scr_size),
			scene: Scene::default(),
		};

		this.scene.new_object();

		this
	}
}

const DATA_KEY: &str = "raytracer_data";

impl RaytracingApp {
	pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
		let scr_size = cc.egui_ctx.screen_rect().size();
		let scr_size = glm::vec2(scr_size.x, scr_size.y);

		// {{{ initialize persistent data
		let mut data = PersistentData::new(scr_size);
		let default_data = data.clone();

		if let Some(storage) = cc.storage {
			if let Some(value) = eframe::get_value(storage, DATA_KEY) {
				data = value;
			}
		}
		// }}}

		// obtain contexts
		let gl = cc.gl.as_ref().expect("obtaining GL context failed");
		let egui = &cc.egui_ctx;

		// {{{ reduce window shadow size
		egui.set_visuals(egui::Visuals {
			window_shadow: egui::epaint::Shadow {
				offset: egui::Vec2::splat(0.0),
				blur: 16.0,
				color: egui::Color32::from_black_alpha(64),
				..Default::default()
			},
			..Default::default()
		});
		// }}}

		// reset window positions
		egui.memory_mut(|mem| mem.reset_areas());

		Self {
			renderer: Arc::new(Mutex::new(Raytracer::new(
				gl,
				&data.camera, // needed to initialize ray directions texture
				scr_size,
			))),
			data: Arc::new(Mutex::new(data)),
			default_data,
			screenshot_time: None,
		}
	}
}

impl eframe::App for RaytracingApp {
	fn save(&mut self, storage: &mut dyn eframe::Storage) {
		eframe::set_value(storage, DATA_KEY, &*self.data.lock());
	}

	fn update(&mut self, egui: &egui::Context, frame: &mut eframe::Frame) {
		let mut data = self.data.lock();

		// {{{ draw windows
		// draw settings window
		let frame_index = self.renderer.lock().frame_index;
		if self.screenshot_time.is_none() {
			data.settings.window(egui, frame_index);
		}
		let settings_response = data.settings.response;

		// draw scene window
		if self.screenshot_time.is_none() {
			data.scene.window(egui);
		}
		let scene_response = data.scene.response;
		// }}}

		// {{{ respond
		// prepare screenshot if requested
		if settings_response.screenshot {
			self.screenshot_time = Some(0.0);
		}

		// clear data if requested
		if settings_response.clear_data {
			*data = self.default_data.clone();
		}

		// fixes error with simultaneous mutable borrow of self field
		drop(data);

		// save data if requested
		if settings_response.save_data {
			self.save(frame.storage_mut().unwrap());
		}
		// }}}

		// main painting
		egui::CentralPanel::default().show(egui, |ui| {
			self.paint(ui, settings_response.focused || scene_response.focused);
		});

		// request repaint so our path tracing continues sampling without activity
		egui.request_repaint_of(egui.viewport_id());

		// count up to 5 seconds. when 5 seconds are over, windows are shown again
		let dt = egui.input(|i| i.unstable_dt);
		if let Some(time) = self.screenshot_time.as_mut() {
			*time += dt;
			if *time > 5.0 {
				self.screenshot_time = None;
			}
		}
	}

	fn on_exit(&mut self, gl: Option<&eframe::glow::Context>) {
		if let Some(gl) = gl {
			self.renderer.lock().destroy(gl);
		}
	}
}
