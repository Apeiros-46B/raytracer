// use std::time::Duration;

use eframe::{
	egui_glow,
	glow::{self, Context, HasContext},
	// glow::Query,
};

use crate::app::RaytracingApp;

#[derive(Clone, Copy)]
pub struct Uniforms {
	scr_size: egui::Vec2,
	sky_color: [f32; 3],
	max_bounces: u32,
}

// pub fn render_stats_window(egui: &egui::Context, frame_time: Duration) {
// 	egui::Window::new("Render Stats").show(egui, |ui| {
// 		let ms = frame_time.as_millis();
// 		let fps = if ms == 0 { 0 } else { 1000 / ms };
// 		ui.label(format!("FPS: {fps}"));
// 		ui.label(format!("Frame time: {frame_time:#?}"))
// 	});
// }

impl RaytracingApp {
	pub fn paint(&mut self, ui: &mut egui::Ui) {
		let screen = ui.clip_rect();
		let renderer = self.renderer.clone();

		let uniforms = Uniforms {
			scr_size: screen.size(),
			sky_color: self.settings.sky_color,
			max_bounces: self.settings.max_bounces,
		};

		let callback = egui::PaintCallback {
			rect: screen,
			callback: std::sync::Arc::new(egui_glow::CallbackFn::new(
				move |_info, painter| {
					renderer.lock().paint(painter.gl(), uniforms);
				},
			)),
		};
		ui.painter().add(callback);
	}
}

pub struct Raytracer {
	program: glow::Program,
	vertex_array: glow::VertexArray,
	// timer_query: Query,
	// timer_query_nanos: u64,
	// pub frame_time: Duration,
}

impl Raytracer {
	pub fn new(gl: &Context) -> Self {
		let shader_version = if cfg!(target_arch = "wasm32") {
			"#version 300 es"
		} else {
			"#version 330"
		};

		let sources = [
			(glow::VERTEX_SHADER, include_str!("shaders/vsh.glsl")),
			(glow::FRAGMENT_SHADER, include_str!("shaders/fsh.glsl")),
		];

		unsafe {
			let program = gl.create_program().expect("failed to create program");

			// {{{ shader instantiation boilerplate
			let shaders: Vec<_> = sources
				.iter()
				.map(|(shader_type, shader_source)| {
					let shader = gl
						.create_shader(*shader_type)
						.expect("failed to create shader");
					gl.shader_source(
						shader,
						&format!("{shader_version}\n{shader_source}"),
					);
					gl.compile_shader(shader);
					assert!(
						gl.get_shader_compile_status(shader),
						"failed to compile {shader_type}: {}",
						gl.get_shader_info_log(shader)
					);
					gl.attach_shader(program, shader);
					shader
				})
				.collect();

			gl.link_program(program);
			assert!(
				gl.get_program_link_status(program),
				"{}",
				gl.get_program_info_log(program)
			);

			for shader in shaders {
				gl.detach_shader(program, shader);
				gl.delete_shader(shader);
			}
			// }}}

			let vertex_array = gl
				.create_vertex_array()
				.expect("failed to create vertex array");

			// let timer_query = gl
			// 	.create_query()
			// 	.expect("failed to create timer query object");

			Self {
				program,
				vertex_array,
				// timer_query,
				// timer_query_nanos: 0,
				// frame_time: Duration::default(),
			}
		}
	}

	pub fn destroy(&self, gl: &Context) {
		unsafe {
			gl.delete_program(self.program);
			gl.delete_vertex_array(self.vertex_array);
		}
	}

	pub fn paint(&mut self, gl: &Context, uniforms: Uniforms) {
		unsafe {
			// gl.begin_query(glow::TIME_ELAPSED, self.timer_query);

			gl.use_program(Some(self.program));
			self.uniforms(gl, uniforms);
			gl.bind_vertex_array(Some(self.vertex_array));
			gl.draw_arrays(glow::TRIANGLES, 0, 3);

			// gl.end_query(glow::TIME_ELAPSED);

			// let mut available = 0;
			// while available == 0 {
			// 	available = gl.get_query_parameter_u32(
			// 		self.timer_query,
			// 		glow::QUERY_RESULT_AVAILABLE,
			// 	);
			// }

			// the query result is in nanoseconds
			// gl.get_query_parameter_u64_with_offset(
			// 	self.timer_query,
			// 	glow::QUERY_RESULT,
			// 	(&mut self.timer_query_nanos) as *mut u64 as usize,
			// );
			// self.frame_time = Duration::from_nanos(self.timer_query_nanos);
		}
	}

	pub fn uniforms(&mut self, gl: &Context, uniforms: Uniforms) {
		unsafe {
			gl.uniform_2_f32(
				gl.get_uniform_location(self.program, "u_scr_size").as_ref(),
				uniforms.scr_size.x,
				uniforms.scr_size.y,
			);

			gl.uniform_3_f32(
				gl.get_uniform_location(self.program, "u_sky_color").as_ref(),
				uniforms.sky_color[0],
				uniforms.sky_color[1],
				uniforms.sky_color[2],
			);

			gl.uniform_1_u32(
				gl.get_uniform_location(self.program, "u_max_bounces").as_ref(),
				uniforms.max_bounces,
			);
		}
	}
}
