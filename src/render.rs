use eframe::{
	egui_glow,
	glow::{self, Context, HasContext, Program, VertexArray},
};
use nalgebra_glm as glm;

use crate::{
	app::RaytracingApp, camera::Camera, scene::Scene, uniform::Uniforms,
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
		let input_state = ui.input(|i| i.clone());

		let callback = egui::PaintCallback {
			rect: scr,
			callback: std::sync::Arc::new(egui_glow::CallbackFn::new(
				move |_, painter| {
					let mut raytracer = renderer.lock();
					let gl = painter.gl();

					raytracer.set_scr_size(gl, glm::vec2(scr_size.x, scr_size.y));
					raytracer.paint(gl, uniforms);
					raytracer.frame_index += 1;

					// update camera
					raytracer.camera.set_fov(fov);
					// don't respond to keypresses if text is focused
					if !text_focused && raytracer.camera.update(input_state.clone()) {
						// reset frame index if moved
						raytracer.frame_index = 0;
					};
					if raytracer.camera.recalculate_ray_dirs {
						raytracer.calculate_ray_directions(gl);
						raytracer.camera.recalculate_ray_dirs = false;
					}
				},
			)),
		};
		ui.painter().add(callback);
	}
}

pub struct Raytracer {
	// prepass calculates ray directions for each pixel when the screen size changes
	prepass_fbo: glow::Framebuffer,
	prepass_texture: glow::Texture,
	prepass_program: Program,
	prepass_verts: VertexArray,

	program: Program,
	verts: VertexArray,

	camera: Camera,
	scene: Scene,

	frame_index: u32,
	scr_size: glm::Vec2,
}

// {{{ shader compilation boilerplate
macro_rules! fragment_shader {
	($location:literal) => {
		&[
			(glow::VERTEX_SHADER, include_str!("shaders/vsh.glsl")),
			(
				glow::FRAGMENT_SHADER,
				include_str!(concat!("shaders/", $location)),
			),
		]
	};
}

#[cfg(not(target_arch = "wasm32"))]
const SHADER_VERSION: &str = "#version 330";

#[cfg(target_arch = "wasm32")]
const SHADER_VERSION: &str = "#version 300 es";

unsafe fn compile_shaders(
	gl: &Context,
	program: Program,
	srcs: &[(u32, &'static str)],
) {
	let shaders: Vec<_> = srcs
		.iter()
		.map(|(ty, src)| {
			let shader = gl.create_shader(*ty).expect("create shader failed");
			gl.shader_source(shader, &format!("{SHADER_VERSION}\n{src}"));
			gl.compile_shader(shader);
			assert!(
				gl.get_shader_compile_status(shader),
				"compile {ty} shader failed: {}",
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
}
// }}}

impl Raytracer {
	pub fn new(
		gl: &Context,
		camera: Camera,
		scene: Scene,
		scr_size: glm::Vec2,
	) -> Self {
		unsafe {
			// {{{ create shader programs
			let prepass_program = gl.create_program().expect("create program failed");
			let program = gl.create_program().expect("create program failed");

			compile_shaders(
				gl,
				prepass_program,
				fragment_shader!("prepass_fsh.glsl"),
			);
			compile_shaders(gl, program, fragment_shader!("fsh.glsl"));

			let prepass_verts = gl
				.create_vertex_array()
				.expect("create vertex array failed");
			let verts = gl
				.create_vertex_array()
				.expect("create vertex array failed");
			// }}}

			// {{{ create prepass FBO and texture
			let prepass_fbo = gl.create_framebuffer().expect("create FBO failed");
			let prepass_texture = gl.create_texture().expect("create texture failed");

			gl.bind_texture(glow::TEXTURE_2D, Some(prepass_texture));
			gl.bind_framebuffer(glow::FRAMEBUFFER, Some(prepass_fbo));
			gl.tex_image_2d(
				glow::TEXTURE_2D,
				0,
				glow::RGBA32UI as i32,
				scr_size.x as i32,
				scr_size.y as i32,
				0,
				glow::RGBA_INTEGER,
				glow::UNSIGNED_INT,
				None,
			);
			gl.tex_parameter_i32(
				glow::TEXTURE_2D,
				glow::TEXTURE_MIN_FILTER,
				glow::NEAREST as i32,
			);
			gl.tex_parameter_i32(
				glow::TEXTURE_2D,
				glow::TEXTURE_MAG_FILTER,
				glow::NEAREST as i32,
			);
			gl.framebuffer_texture_2d(
				glow::FRAMEBUFFER,
				glow::COLOR_ATTACHMENT0,
				glow::TEXTURE_2D,
				Some(prepass_texture),
				0,
			);
			gl.bind_texture(glow::TEXTURE_2D, None);
			gl.bind_framebuffer(glow::FRAMEBUFFER, None);

			let fbo_status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
			assert!(
				fbo_status == glow::FRAMEBUFFER_COMPLETE,
				"framebuffer incomplete: {fbo_status}"
			);
			// }}}

			let mut this = Self {
				prepass_fbo,
				prepass_texture,
				prepass_program,
				prepass_verts,

				program,
				verts,

				camera,
				scene,

				frame_index: 0,
				scr_size,
			};
			this.calculate_ray_directions(gl);
			this
		}
	}

	pub fn destroy(&self, gl: &Context) {
		unsafe {
			gl.delete_framebuffer(self.prepass_fbo);
			gl.delete_texture(self.prepass_texture);
			gl.delete_program(self.prepass_program);
			gl.delete_vertex_array(self.prepass_verts);

			gl.delete_program(self.program);
			gl.delete_vertex_array(self.verts);
		}
	}

	pub fn paint(&mut self, gl: &Context, uniforms: Uniforms) {
		unsafe {
			gl.bind_framebuffer(glow::FRAMEBUFFER, None);
			gl.use_program(Some(self.program));
			self.apply_uniforms(gl, uniforms);
			gl.bind_texture(glow::TEXTURE_2D, Some(self.prepass_texture));
			gl.bind_vertex_array(Some(self.verts));
			gl.draw_arrays(glow::TRIANGLES, 0, 3);
			gl.bind_texture(glow::TEXTURE_2D, None);
		}
	}

	// {{{ set screen size
	fn set_scr_size(&mut self, gl: &Context, new_scr_size: glm::Vec2) {
		if self.scr_size == new_scr_size {
			return;
		}

		self.scr_size = new_scr_size;
		self.camera.set_scr_size(new_scr_size);

		unsafe {
			// resize ray directions texture
			gl.bind_texture(glow::TEXTURE_2D, Some(self.prepass_texture));
			gl.tex_image_2d(
				glow::TEXTURE_2D,
				0,
				glow::RGBA32UI as i32,
				new_scr_size.x as i32,
				new_scr_size.y as i32,
				0,
				glow::RGBA_INTEGER,
				glow::UNSIGNED_INT,
				None,
			);
			gl.bind_texture(glow::TEXTURE_2D, None);
		}
	}
	// }}}

	// {{{ calculate ray directions
	fn calculate_ray_directions(&mut self, gl: &Context) {
		unsafe {
			gl.use_program(Some(self.prepass_program));

			gl.uniform_2_f32(
				gl.get_uniform_location(self.prepass_program, "scr_size")
					.as_ref(),
				self.scr_size.x,
				self.scr_size.y,
			);
			gl.uniform_matrix_4_f32_slice(
				gl.get_uniform_location(self.prepass_program, "inv_proj")
					.as_ref(),
				false, // no transpose, it's already in column-major order
				self.camera.inv_proj.as_slice(),
			);
			gl.uniform_matrix_4_f32_slice(
				gl.get_uniform_location(self.prepass_program, "inv_view")
					.as_ref(),
				false, // no transpose, it's already in column-major order
				self.camera.inv_view.as_slice(),
			);

			gl.bind_vertex_array(Some(self.prepass_verts));
			gl.bind_texture(glow::TEXTURE_2D, Some(self.prepass_texture));
			gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.prepass_fbo));
			gl.draw_buffers(&[glow::COLOR_ATTACHMENT0]);
			gl.clear_buffer_u32_slice(glow::COLOR, 0, &[0, 0, 0, 0]);
			gl.draw_arrays(glow::TRIANGLES, 0, 3);

			gl.bind_framebuffer(glow::FRAMEBUFFER, None);
			gl.bind_texture(glow::TEXTURE_2D, None);
			gl.use_program(Some(self.program));
		}
	}
	// }}}

	fn apply_uniforms(&mut self, gl: &Context, uniforms: Uniforms) {
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
				self.scene.radii.len().try_into().unwrap(),
			);

			gl.uniform_1_f32_slice(
				gl.get_uniform_location(self.program, "sphere_radii")
					.as_ref(),
				&self.scene.radii,
			);

			gl.uniform_3_f32_slice(
				gl.get_uniform_location(self.program, "sphere_pos").as_ref(),
				&self.scene.pos,
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
