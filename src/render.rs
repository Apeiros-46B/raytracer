use eframe::{
	egui_glow,
	glow::{self, Context, HasContext, Program, VertexArray},
};
use nalgebra_glm as glm;

use crate::{
	app::{PersistentData, RaytracingApp},
	camera::Camera,
	util::{fill_50, flatten_mats, Reset},
};

pub struct Raytracer {
	// prepass calculates ray directions for each pixel when the screen size changes
	ray_dirs_fbo: glow::Framebuffer,
	ray_dirs_texture: glow::Texture,
	ray_dirs_program: Program,
	ray_dirs_verts: VertexArray,
	program: Program,
	verts: VertexArray,

	scr_size: glm::Vec2,
	first_frame: bool,
	pub frame_index: u32,
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

impl RaytracingApp {
	pub fn paint(&mut self, ui: &mut egui::Ui, ui_focused: bool) {
		let scr = ui.clip_rect();
		let scr_size = scr.size();
		let raytracer_mutex = self.renderer.clone();
		let data_mutex = self.data.clone();
		let input = ui.input(|i| i.clone());

		// {{{ paint callback
		let callback = egui::PaintCallback {
			rect: scr,
			callback: std::sync::Arc::new(egui_glow::CallbackFn::new(
				move |_, painter| {
					let mut raytracer = raytracer_mutex.lock();
					let mut data = data_mutex.lock();

					let gl = painter.gl();

					raytracer.set_scr_size(
						gl,
						&mut data.camera,
						glm::vec2(scr_size.x, scr_size.y),
					);

					raytracer.paint(gl, &data);
					raytracer.frame_index += 1;

					if !data.settings.render.lock_camera {
						// {{{ update camera
						let fov = data.settings.render.fov;
						data.camera.set_fov(fov);
						if !ui_focused && data.camera.update(input.clone()) {
							// don't respond to keypresses if text is focused
							// reset frame index if moved
							raytracer.frame_index = 0;
						};
						if data.camera.recalculate_ray_dirs {
							raytracer.calculate_ray_dirs(gl, &data.camera);
							data.camera.recalculate_ray_dirs = false;
						}
						// }}}
					}

					data.settings.response.reset();
					data.scene.response.reset();
				},
			)),
		};
		ui.painter().add(callback);
		// }}}
	}
}

impl Raytracer {
	pub fn new(gl: &Context, camera: &Camera, scr_size: glm::Vec2) -> Self {
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
				ray_dirs_fbo: prepass_fbo,
				ray_dirs_texture: prepass_texture,
				ray_dirs_program: prepass_program,
				ray_dirs_verts: prepass_verts,
				program,
				verts,

				scr_size,
				first_frame: true,
				frame_index: 0,
			};
			// initial ray direction calculation
			this.calculate_ray_dirs(gl, camera);
			this
		}
	}

	// {{{ clean up GL objects
	pub fn destroy(&self, gl: &Context) {
		unsafe {
			gl.delete_framebuffer(self.ray_dirs_fbo);
			gl.delete_texture(self.ray_dirs_texture);
			gl.delete_program(self.ray_dirs_program);
			gl.delete_vertex_array(self.ray_dirs_verts);

			gl.delete_program(self.program);
			gl.delete_vertex_array(self.verts);
		}
	}
	// }}}

	// {{{ call on every frame to render
	pub fn paint(&mut self, gl: &Context, data: &PersistentData) {
		unsafe {
			gl.bind_framebuffer(glow::FRAMEBUFFER, None);
			gl.use_program(Some(self.program));
			self.apply_uniforms(gl, data);
			gl.bind_texture(glow::TEXTURE_2D, Some(self.ray_dirs_texture));
			gl.bind_vertex_array(Some(self.verts));
			gl.draw_arrays(glow::TRIANGLES, 0, 3);
			gl.bind_texture(glow::TEXTURE_2D, None);

			self.first_frame = false;
		}
	}
	// }}}

	// {{{ set screen size
	fn set_scr_size(
		&mut self,
		gl: &Context,
		camera: &mut Camera,
		new_scr_size: glm::Vec2,
	) {
		if self.scr_size == new_scr_size {
			return;
		}

		self.scr_size = new_scr_size;
		camera.set_scr_size(new_scr_size);

		unsafe {
			// resize ray directions texture
			gl.bind_texture(glow::TEXTURE_2D, Some(self.ray_dirs_texture));
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
	fn calculate_ray_dirs(&mut self, gl: &Context, camera: &Camera) {
		unsafe {
			gl.use_program(Some(self.ray_dirs_program));

			// bind uniforms for ray direction calculation
			gl.uniform_2_f32(
				gl.get_uniform_location(self.ray_dirs_program, "scr_size")
					.as_ref(),
				self.scr_size.x,
				self.scr_size.y,
			);
			gl.uniform_matrix_4_f32_slice(
				gl.get_uniform_location(self.ray_dirs_program, "inv_proj")
					.as_ref(),
				false, // no transpose, it's already in column-major order
				camera.inv_proj.as_slice(),
			);
			gl.uniform_matrix_4_f32_slice(
				gl.get_uniform_location(self.ray_dirs_program, "inv_view")
					.as_ref(),
				false, // no transpose, it's already in column-major order
				camera.inv_view.as_slice(),
			);

			// draw into framebuffer
			gl.bind_vertex_array(Some(self.ray_dirs_verts));
			gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.ray_dirs_fbo));
			gl.bind_texture(glow::TEXTURE_2D, Some(self.ray_dirs_texture));
			gl.draw_buffers(&[glow::COLOR_ATTACHMENT0]);
			gl.clear_buffer_u32_slice(glow::COLOR, 0, &[0, 0, 0, 0]);
			gl.draw_arrays(glow::TRIANGLES, 0, 3);

			// unbind
			gl.bind_vertex_array(None);
			gl.bind_framebuffer(glow::FRAMEBUFFER, None);
			gl.bind_texture(glow::TEXTURE_2D, None);
			gl.use_program(Some(self.program));
		}
	}
	// }}}

	// apply uniforms to main program
	fn apply_uniforms(&mut self, gl: &Context, data: &PersistentData) {
		unsafe {
			// {{{ state
			gl.uniform_2_f32(
				gl.get_uniform_location(self.program, "scr_size").as_ref(),
				self.scr_size.x,
				self.scr_size.y,
			);

			gl.uniform_3_f32(
				gl.get_uniform_location(self.program, "camera_pos").as_ref(),
				data.camera.pos.x,
				data.camera.pos.y,
				data.camera.pos.z,
			);

			gl.uniform_1_u32(
				gl.get_uniform_location(self.program, "frame_index")
					.as_ref(),
				self.frame_index,
			);
			// }}}

			if self.first_frame || data.scene.response.changed {
				// {{{ scene
				// TODO: these aren't always 50 long which causes an error
				gl.uniform_1_u32(
					gl.get_uniform_location(self.program, "scene_size").as_ref(),
					data.scene.len().try_into().unwrap(),
				);

				gl.uniform_1_u32_slice(
					gl.get_uniform_location(self.program, "scene_obj_types")
						.as_ref(),
					&fill_50(bytemuck::cast_slice(&data.scene.types)),
				);

				gl.uniform_3_f32_slice(
					gl.get_uniform_location(self.program, "scene_obj_mat_colors")
					.as_ref(),
					bytemuck::cast_slice(&fill_50(&data.scene.mat_colors)),
				);

				gl.uniform_3_f32_slice(
					gl.get_uniform_location(self.program, "scene_obj_mat_roughness")
					.as_ref(),
					&data.scene.mat_roughness,
				);

				gl.uniform_matrix_4_f32_slice(
					gl.get_uniform_location(self.program, "scene_transforms")
						.as_ref(),
					false, // no transpose, it's already in column-major order
					flatten_mats(&fill_50(&data.scene.transforms)),
				);

				gl.uniform_matrix_4_f32_slice(
					gl.get_uniform_location(self.program, "scene_inv_transforms")
						.as_ref(),
					false, // no transpose, it's already in column-major order
					flatten_mats(&fill_50(&data.scene.inv_transforms)),
				);

				gl.uniform_matrix_4_f32_slice(
					gl.get_uniform_location(self.program, "scene_normal_transforms")
						.as_ref(),
					false, // no transpose, it's already in column-major order
					flatten_mats(&fill_50(&data.scene.normal_transforms)),
				);
				// }}}
			}

			if self.first_frame || data.settings.response.changed {
				// {{{ world settings
				// sky color
				gl.uniform_3_f32(
					gl.get_uniform_location(self.program, "sky_color").as_ref(),
					data.settings.world.sky_color[0],
					data.settings.world.sky_color[1],
					data.settings.world.sky_color[2],
				);

				// sun direction
				let beta_cos = data.settings.world.sun_elevation.cos();
				let x = data.settings.world.sun_rotation.cos() * beta_cos;
				let y = data.settings.world.sun_elevation.sin();
				let z = data.settings.world.sun_rotation.sin() * beta_cos;
				let mag = (x.powi(2) + y.powi(2) + z.powi(2)).sqrt();
				gl.uniform_3_f32(
					gl.get_uniform_location(self.program, "sun_dir").as_ref(),
					x / mag,
					y / mag,
					z / mag,
				);

				// sun strength
				gl.uniform_1_f32(
					gl.get_uniform_location(self.program, "sun_strength")
						.as_ref(),
					data.settings.world.sun_strength,
				);
				// }}}

				// {{{ render settings
				// render mode
				gl.uniform_1_u32(
					gl.get_uniform_location(self.program, "render_mode")
						.as_ref(),
					data.settings.render.mode as u32,
				);

				// maximum light bounces
				gl.uniform_1_u32(
					gl.get_uniform_location(self.program, "max_bounces")
						.as_ref(),
					data.settings.render.max_bounces,
				);
				// }}}
			}
		}
	}
}
