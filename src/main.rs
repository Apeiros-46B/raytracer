#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use raytracer::RaytracingApp;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
	env_logger::init();

	let native_options = eframe::NativeOptions {
		viewport: egui::ViewportBuilder::default()
			.with_inner_size([400.0, 300.0])
			.with_min_inner_size([300.0, 220.0])
			.with_icon(
				eframe::icon_data::from_png_bytes(
					&include_bytes!("../assets/icon-256.png")[..],
				)
				.expect("loading icon failed"),
			),
		..Default::default()
	};
	eframe::run_native(
		"Raytracer",
		native_options,
		Box::new(|cc| Box::new(RaytracingApp::new(cc))),
	)
}

#[cfg(target_arch = "wasm32")]
fn main() {
	eframe::WebLogger::init(log::LevelFilter::Debug).ok();

	let web_options = eframe::WebOptions::default();

	wasm_bindgen_futures::spawn_local(async {
		eframe::WebRunner::new()
			.start(
				"gl_canvas",
				web_options,
				Box::new(|cc| Box::new(RaytracingApp::new(cc))),
			)
			.await
			.expect("starting eframe failed");
	});
}
