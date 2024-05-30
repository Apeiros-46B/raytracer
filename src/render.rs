use eframe::{
	egui_glow,
	glow::{self, Context, HasContext},
};

use crate::{
	app::RaytracingApp, camera::Camera, scene::Spheres, uniform::Uniforms,
};

impl RaytracingApp {
	pub fn paint(&mut self, ui: &mut egui::Ui, text_focused: bool) {
		let scr = ui.clip_rect();
		let scr_size = scr.size();

		let fov = self.settings.render.fov;
		let renderer = self.renderer.clone();

		// TODO: remove the whole uniforms struct and just let apply_uniforms take
		// a clone of settings
		let uniforms = Uniforms::new(&self.settings, scr_size);
		let input = ui.input(|i| i.clone());

		let callback = egui::PaintCallback {
			rect: scr,
			callback: std::sync::Arc::new(egui_glow::CallbackFn::new(
				move |_, painter| {
					let mut raytracer = renderer.lock();

					raytracer.paint(painter.gl(), uniforms);
					raytracer.frame_index += 1;

					raytracer.camera.set_fov(fov);
					raytracer.camera.set_scr_size(scr_size.into());
					// reset frame index if moved
					// don't respond to keypresses if text is focused
					if !text_focused && raytracer.camera.update(input.clone()) {
						raytracer.frame_index = 0;
					};
				},
			)),
		};
		ui.painter().add(callback);
	}
}

pub struct Raytracer {
	program: glow::Program,
	verts: glow::VertexArray,

	camera: Camera,
	spheres: Spheres,

	frame_index: u32,
}

impl Raytracer {
	pub fn new(gl: &Context) -> Self {
		let srcs = [
			(glow::VERTEX_SHADER, include_str!("shaders/vsh.glsl")),
			(glow::FRAGMENT_SHADER, include_str!("shaders/fsh.glsl")),
		];

		let shader_version = if cfg!(target_arch = "wasm32") {
			"#version 300 es"
		} else {
			"#version 330"
		};

		unsafe {
			let program = gl.create_program().expect("failed to create program");

			// {{{ shader instantiation boilerplate
			let shaders: Vec<_> = srcs
				.iter()
				.map(|(ty, src)| {
					let shader = gl.create_shader(*ty).expect("failed to create shader");
					gl.shader_source(shader, &format!("{shader_version}\n{src}"));
					gl.compile_shader(shader);
					assert!(
						gl.get_shader_compile_status(shader),
						"failed to compile {ty}: {}",
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

			let verts = gl
				.create_vertex_array()
				.expect("failed to create vertex array");

			Self {
				program,
				verts,

				// [1.0, 1.0] is temporary. we update the actual size in paint callback
				camera: Camera::new(70.0_f32.to_radians(), [1.0, 1.0]),
				spheres: Spheres {
					radii: Box::new([0.5, 0.3]),
					pos: Box::new([0.0, -0.4, 0.0, 0.0, 0.4, 0.0]),
				},

				frame_index: 0,
			}
		}
	}

	pub fn destroy(&self, gl: &Context) {
		unsafe {
			gl.delete_program(self.program);
			gl.delete_vertex_array(self.verts);
		}
	}

	pub fn paint(&mut self, gl: &Context, uniforms: Uniforms) {
		unsafe {
			gl.use_program(Some(self.program));
			self.apply_uniforms(gl, uniforms);
			gl.bind_vertex_array(Some(self.verts));
			gl.draw_arrays(glow::TRIANGLES, 0, 3);

			// block until the rendering finishes, allowing CPU-side timing
			gl.finish();
		}
	}

	pub fn apply_uniforms(&mut self, gl: &Context, uniforms: Uniforms) {
		unsafe {
			gl.uniform_2_f32(
				gl.get_uniform_location(self.program, "scr_size").as_ref(),
				uniforms.scr_size[0],
				uniforms.scr_size[1],
			);

			gl.uniform_1_u32(
				gl.get_uniform_location(self.program, "frame_index")
					.as_ref(),
				self.frame_index,
			);

			// {{{ sky colors
			gl.uniform_3_f32(
				gl.get_uniform_location(self.program, "sky_color").as_ref(),
				uniforms.sky_color[0],
				uniforms.sky_color[1],
				uniforms.sky_color[2],
			);

			gl.uniform_3_f32(
				gl.get_uniform_location(self.program, "sun_dir").as_ref(),
				uniforms.sun_dir[0],
				uniforms.sun_dir[1],
				uniforms.sun_dir[2],
			);

			gl.uniform_1_f32(
				gl.get_uniform_location(self.program, "sun_strength")
					.as_ref(),
				uniforms.sun_strength,
			);
			// }}}

			gl.uniform_1_u32(
				gl.get_uniform_location(self.program, "bounces").as_ref(),
				uniforms.max_bounces,
			);

			// {{{ sphere
			gl.uniform_1_u32(
				gl.get_uniform_location(self.program, "sphere_count")
					.as_ref(),
				self.spheres.radii.len().try_into().unwrap(),
			);

			gl.uniform_1_f32_slice(
				gl.get_uniform_location(self.program, "sphere_radii")
					.as_ref(),
				&self.spheres.radii,
			);

			gl.uniform_3_f32_slice(
				gl.get_uniform_location(self.program, "sphere_pos").as_ref(),
				&self.spheres.pos,
			);
			// }}}

			// {{{ camera
			gl.uniform_3_f32(
				gl.get_uniform_location(self.program, "camera_pos").as_ref(),
				self.camera.pos.x,
				self.camera.pos.y,
				self.camera.pos.z,
			);

			gl.uniform_matrix_4_f32_slice(
				gl.get_uniform_location(self.program, "camera_inv_proj")
					.as_ref(),
				false, // no transpose, it's already in column-major order
				self.camera.inv_proj.as_slice(),
			);

			gl.uniform_matrix_4_f32_slice(
				gl.get_uniform_location(self.program, "camera_inv_view")
					.as_ref(),
				false, // no transpose, it's already in column-major order
				self.camera.inv_view.as_slice(),
			);
			// }}}
		}
	}
}
