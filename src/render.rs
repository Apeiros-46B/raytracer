use eframe::{
	egui_glow,
	glow::{self, Context, Framebuffer, HasContext, Program, Texture, VertexArray},
};
use nalgebra_glm as glm;

use crate::{
	app::{PersistentData, RaytracingApp},
	camera::Camera,
	util::{fill_50, flatten_matrices, Reset},
};

pub struct Raytracer {
	clear_fbo: Framebuffer,

	// prepass calculates ray directions for each pixel when the screen size changes
	ray_dirs_fbo: Framebuffer,
	ray_dirs_texture: Texture,
	ray_dirs_program: Program,
	ray_dirs_verts: VertexArray,

	// prepass calculates noise based on the noise of the previous frame
	// this prevents large seeds leading to less randomness
	noise_fbo: Framebuffer,
	noise_texture_0: Texture,
	noise_texture_1: Texture,
	noise_program: Program,
	noise_verts: VertexArray,

	accumulation_fbo: Framebuffer,
	accumulation_texture_0: Texture,
	accumulation_texture_1: Texture,
	program: Program,
	verts: VertexArray,

	final_program: Program,
	final_verts: VertexArray,

	scr_size: glm::Vec2,
	first_frame: bool,
	rendering_to_texture_0: bool,
	pub frame_index: u32,

	pub force_scr_size: bool,
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

#[cfg(not(target = "wasm32"))]
fn scale() -> f32 {
	1.0
}

#[cfg(target = "wasm32")]
#[wasm_bindgen]
fn scale() -> f32 {
	web_sys::window().unwrap().device_pixel_ratio() as f32
}

impl RaytracingApp {
	pub fn paint(&mut self, ui: &mut egui::Ui, ui_focused: bool) {
		let scr = ui.clip_rect();
		let scr_size = glm::vec2(scr.size().x, scr.size().y) / scale();

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

					raytracer.set_scr_size(gl, &mut data.camera, scr_size);

					raytracer.paint(gl, &data);

					if !data.settings.render.lock_camera {
						// {{{ update camera
						let fov = data.settings.render.fov;
						data.camera.set_fov(fov);
						if !ui_focused && data.camera.update(input.clone()) {
							// don't respond to keypresses if text is focused
							raytracer.frame_index = 1;
							raytracer.clear_textures(gl);
						};
						if data.camera.recalculate_ray_dirs {
							raytracer.calculate_ray_dirs(gl, &data.camera);
							data.camera.recalculate_ray_dirs = false;
						}
						// }}}
					}

					if data.settings.response.changed || data.scene.response.changed {
						raytracer.frame_index = 1;
						raytracer.clear_textures(gl);
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
			let ray_dirs_program = gl.create_program().expect("create program failed");
			let noise_program = gl.create_program().expect("create program failed");
			let program = gl.create_program().expect("create program failed");
			let final_program = gl.create_program().expect("create program failed");

			compile_shaders(gl, ray_dirs_program, fragment_shader!("ray_dirs.glsl"));
			compile_shaders(gl, noise_program, fragment_shader!("noise.glsl"));
			compile_shaders(gl, program, fragment_shader!("fsh.glsl"));
			compile_shaders(gl, final_program, fragment_shader!("final.glsl"));

			let ray_dirs_verts = gl
				.create_vertex_array()
				.expect("create vertex array failed");
			let noise_verts = gl
				.create_vertex_array()
				.expect("create vertex array failed");
			let verts = gl
				.create_vertex_array()
				.expect("create vertex array failed");
			let final_verts = gl
				.create_vertex_array()
				.expect("create vertex array failed");
			// }}}

			// {{{ create prepass (ray dirs) FBO and texture
			let ray_dirs_fbo = gl.create_framebuffer().expect("create FBO failed");
			let ray_dirs_texture = gl.create_texture().expect("create texture failed");

			gl.bind_texture(glow::TEXTURE_2D, Some(ray_dirs_texture));
			gl.bind_framebuffer(glow::FRAMEBUFFER, Some(ray_dirs_fbo));
			screen_sized_texture(gl, scr_size, true);
			framebuffer_texture(gl, ray_dirs_texture);
			gl.bind_texture(glow::TEXTURE_2D, None);
			gl.bind_framebuffer(glow::FRAMEBUFFER, None);

			let fbo_status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
			assert!(
				fbo_status == glow::FRAMEBUFFER_COMPLETE,
				"framebuffer incomplete: {fbo_status}"
			);
			// }}}

			// {{{ create prepass (noise) FBO and texture
			let noise_fbo = gl.create_framebuffer().expect("create FBO failed");
			let noise_texture_0 = gl.create_texture().expect("create texture failed");
			let noise_texture_1 = gl.create_texture().expect("create texture failed");

			gl.bind_texture(glow::TEXTURE_2D, Some(noise_texture_0));
			screen_sized_texture(gl, scr_size, true);
			gl.bind_texture(glow::TEXTURE_2D, Some(noise_texture_1));
			screen_sized_texture(gl, scr_size, true);

			gl.bind_framebuffer(glow::FRAMEBUFFER, Some(noise_fbo));
			framebuffer_texture(gl, noise_texture_0);

			gl.bind_texture(glow::TEXTURE_2D, None);
			gl.bind_framebuffer(glow::FRAMEBUFFER, None);

			let fbo_status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
			assert!(
				fbo_status == glow::FRAMEBUFFER_COMPLETE,
				"framebuffer incomplete: {fbo_status}"
			);
			// }}}

			// {{{ create accumulation FBO and texture
			let accumulation_fbo = gl.create_framebuffer().expect("create FBO failed");
			let accumulation_texture_0 =
				gl.create_texture().expect("create texture failed");
			let accumulation_texture_1 =
				gl.create_texture().expect("create texture failed");

			gl.bind_texture(glow::TEXTURE_2D, Some(accumulation_texture_0));
			screen_sized_texture(gl, scr_size, true);
			gl.bind_texture(glow::TEXTURE_2D, Some(accumulation_texture_1));
			screen_sized_texture(gl, scr_size, true);

			gl.bind_framebuffer(glow::FRAMEBUFFER, Some(accumulation_fbo));
			framebuffer_texture(gl, accumulation_texture_0);

			gl.bind_texture(glow::TEXTURE_2D, None);
			gl.bind_framebuffer(glow::FRAMEBUFFER, None);

			let fbo_status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
			assert!(
				fbo_status == glow::FRAMEBUFFER_COMPLETE,
				"framebuffer incomplete: {fbo_status}"
			);
			// }}}

			let mut this = Self {
				clear_fbo: gl.create_framebuffer().expect("create FBO failed"),

				ray_dirs_fbo,
				ray_dirs_texture,
				ray_dirs_program,
				ray_dirs_verts,

				noise_fbo,
				noise_texture_0,
				noise_texture_1,
				noise_program,
				noise_verts,

				accumulation_fbo,
				accumulation_texture_0,
				accumulation_texture_1,
				program,
				verts,

				final_program,
				final_verts,

				scr_size,
				first_frame: true,
				rendering_to_texture_0: true,

				// this starts at one to avoid division by zero
				frame_index: 1,

				force_scr_size: false,
			};
			// initial ray direction calculation
			this.calculate_ray_dirs(gl, camera);
			this
		}
	}

	// {{{ clean up GL objects
	pub fn destroy(&self, gl: &Context) {
		unsafe {
			gl.delete_framebuffer(self.clear_fbo);

			gl.delete_framebuffer(self.ray_dirs_fbo);
			gl.delete_texture(self.ray_dirs_texture);
			gl.delete_program(self.ray_dirs_program);
			gl.delete_vertex_array(self.ray_dirs_verts);

			gl.delete_framebuffer(self.accumulation_fbo);
			gl.delete_texture(self.accumulation_texture_0);
			gl.delete_texture(self.accumulation_texture_1);
			gl.delete_program(self.program);
			gl.delete_vertex_array(self.verts);

			gl.delete_program(self.final_program);
			gl.delete_vertex_array(self.final_verts);
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
		if self.force_scr_size {
			self.force_scr_size = false;
		} else if self.scr_size == new_scr_size {
			return;
		}

		self.scr_size = new_scr_size;
		camera.set_scr_size(new_scr_size);

		self.frame_index = 1;
		self.realloc_textures(gl, new_scr_size);
	}
	// }}}

	// {{{ reset textures
	fn realloc_textures(&self, gl: &Context, scr_size: glm::Vec2) {
		unsafe {
			gl.bind_texture(glow::TEXTURE_2D, Some(self.ray_dirs_texture));
			screen_sized_texture(gl, scr_size, false);
			gl.bind_texture(glow::TEXTURE_2D, Some(self.noise_texture_0));
			screen_sized_texture(gl, scr_size, true);
			gl.bind_texture(glow::TEXTURE_2D, Some(self.noise_texture_1));
			screen_sized_texture(gl, scr_size, true);
			gl.bind_texture(glow::TEXTURE_2D, Some(self.accumulation_texture_0));
			screen_sized_texture(gl, scr_size, true);
			gl.bind_texture(glow::TEXTURE_2D, Some(self.accumulation_texture_1));
			screen_sized_texture(gl, scr_size, true);
		}
	}

	fn clear_textures(&self, gl: &Context) {
		unsafe {
			gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.clear_fbo));

			framebuffer_texture(gl, self.noise_texture_0);
			gl.draw_buffers(&[glow::COLOR_ATTACHMENT0]);
			gl.clear_buffer_u32_slice(glow::COLOR, 0, &[0, 0, 0, 0]);
			framebuffer_texture(gl, self.noise_texture_1);
			gl.draw_buffers(&[glow::COLOR_ATTACHMENT0]);
			gl.clear_buffer_u32_slice(glow::COLOR, 0, &[0, 0, 0, 0]);
			framebuffer_texture(gl, self.accumulation_texture_0);
			gl.draw_buffers(&[glow::COLOR_ATTACHMENT0]);
			gl.clear_buffer_u32_slice(glow::COLOR, 0, &[0, 0, 0, 0]);
			framebuffer_texture(gl, self.accumulation_texture_1);
			gl.draw_buffers(&[glow::COLOR_ATTACHMENT0]);
			gl.clear_buffer_u32_slice(glow::COLOR, 0, &[0, 0, 0, 0]);
			gl.bind_framebuffer(glow::FRAMEBUFFER, None);
		}
	}
	// }}}

	// {{{ calculate ray directions
	fn calculate_ray_dirs(&mut self, gl: &Context, camera: &Camera) {
		unsafe {
			gl.use_program(Some(self.ray_dirs_program));

			// {{{ bind uniforms for ray direction calculation
			self.apply_uniforms_common(gl, self.ray_dirs_program);

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
			// }}}

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

	// {{{ call on every frame to render
	pub fn paint(&mut self, gl: &Context, data: &PersistentData) {
		unsafe {
			// {{{ calculate noise texture
			gl.use_program(Some(self.noise_program));
			gl.active_texture(glow::TEXTURE0);
			gl.bind_texture(
				glow::TEXTURE_2D,
				Some(if self.rendering_to_texture_0 {
					self.noise_texture_1
				} else {
					self.noise_texture_0
				}),
			);

			// {{{ uniforms
			self.apply_uniforms_common(gl, self.noise_program);

			// texture sampler
			gl.uniform_1_i32(
				gl.get_uniform_location(self.noise_program, "noise")
					.as_ref(),
				0,
			);
			// }}}

			gl.bind_vertex_array(Some(self.noise_verts));
			gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.noise_fbo));

			// unbind the other texture (the one that is being sampled)
			framebuffer_texture(
				gl,
				if self.rendering_to_texture_0 {
					self.noise_texture_0
				} else {
					self.noise_texture_1
				},
			);

			gl.draw_arrays(glow::TRIANGLES, 0, 3);

			// unbind
			gl.bind_vertex_array(None);
			gl.bind_framebuffer(glow::FRAMEBUFFER, None);
			gl.bind_texture(glow::TEXTURE_2D, None);
			// }}}

			// {{{ draw ray traced image into accumulation buffer
			gl.use_program(Some(self.program));

			self.apply_uniforms(gl, data);

			// {{{ bind textures
			if self.first_frame {
				gl.uniform_1_i32(
					gl.get_uniform_location(self.program, "ray_dirs").as_ref(),
					0, // ray directions texture
				);
				gl.uniform_1_i32(
					gl.get_uniform_location(self.program, "noise").as_ref(),
					1, // noise texture, one of two buffers
				);
				gl.uniform_1_i32(
					gl.get_uniform_location(self.program, "image").as_ref(),
					2, // accumulation texture, one of two buffers
				);
			}
			gl.active_texture(glow::TEXTURE0);
			gl.bind_texture(glow::TEXTURE_2D, Some(self.ray_dirs_texture));

			// sample from the noise that just got generated
			gl.active_texture(glow::TEXTURE1);
			gl.bind_texture(
				glow::TEXTURE_2D,
				Some(if self.rendering_to_texture_0 {
					self.noise_texture_0
				} else {
					self.noise_texture_1
				}),
			);

			// sample from the one that isn't being rendered to
			gl.active_texture(glow::TEXTURE2);
			gl.bind_texture(
				glow::TEXTURE_2D,
				Some(if self.rendering_to_texture_0 {
					self.accumulation_texture_1
				} else {
					self.accumulation_texture_0
				}),
			);
			// }}}

			// draw into accumulation buffer
			gl.bind_vertex_array(Some(self.verts));
			gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.accumulation_fbo));

			// unbind the other texture (the one that is being sampled)
			framebuffer_texture(
				gl,
				if self.rendering_to_texture_0 {
					self.accumulation_texture_0
				} else {
					self.accumulation_texture_1
				},
			);

			gl.draw_arrays(glow::TRIANGLES, 0, 3);

			// unbind
			gl.bind_vertex_array(None);
			gl.bind_framebuffer(glow::FRAMEBUFFER, None);
			gl.bind_texture(glow::TEXTURE_2D, None);
			// }}}

			gl.bind_framebuffer(glow::FRAMEBUFFER, None);

			// {{{ render accumulation buffer with post-process effects
			gl.use_program(Some(self.final_program));

			// {{{ uniforms
			self.apply_uniforms_common(gl, self.final_program);

			// texture sampler
			gl.uniform_1_i32(
				gl.get_uniform_location(self.final_program, "image")
					.as_ref(),
				0,
			);

			gl.uniform_1_u32(
				gl.get_uniform_location(self.final_program, "accumulate")
					.as_ref(),
				data.settings.render.accumulate as u32,
			);
			// }}}

			// sample from the one that just got rendered to
			gl.active_texture(glow::TEXTURE0);
			gl.bind_texture(
				glow::TEXTURE_2D,
				Some(if self.rendering_to_texture_0 {
					self.accumulation_texture_0
				} else {
					self.accumulation_texture_1
				}),
			);
			gl.bind_vertex_array(Some(self.final_verts));
			gl.draw_arrays(glow::TRIANGLES, 0, 3);

			gl.bind_texture(glow::TEXTURE_2D, None);
			gl.bind_vertex_array(None);
			gl.use_program(Some(self.program));

			self.first_frame = false;
			self.frame_index += 1;
			self.rendering_to_texture_0 = !self.rendering_to_texture_0;
			// }}}
		}
	}
	// }}}

	// apply uniforms to main program
	fn apply_uniforms(&mut self, gl: &Context, data: &PersistentData) {
		unsafe {
			self.apply_uniforms_common(gl, self.program);

			// {{{ camera
			gl.uniform_3_f32(
				gl.get_uniform_location(self.program, "camera_pos").as_ref(),
				data.camera.pos.x,
				data.camera.pos.y,
				data.camera.pos.z,
			);

			gl.uniform_3_f32(
				gl.get_uniform_location(self.program, "camera_dir").as_ref(),
				data.camera.forward_dir.x,
				data.camera.forward_dir.y,
				data.camera.forward_dir.z,
			);
			// }}}

			if self.first_frame || data.scene.response.changed {
				// {{{ scene
				// general
				gl.uniform_1_u32(
					gl.get_uniform_location(self.program, "scene_selected")
						.as_ref(),
					data.scene.selected.try_into().unwrap(),
				);

				gl.uniform_1_u32(
					gl.get_uniform_location(self.program, "scene_size").as_ref(),
					data.scene.len().try_into().unwrap(),
				);

				gl.uniform_1_u32_slice(
					gl.get_uniform_location(self.program, "scene_obj_type")
						.as_ref(),
					&fill_50(bytemuck::cast_slice(&data.scene.ty)),
				);

				// materials
				gl.uniform_1_u32_slice(
					gl.get_uniform_location(self.program, "scene_mat_type")
						.as_ref(),
					&fill_50(bytemuck::cast_slice(&data.scene.mat_ty)),
				);

				gl.uniform_3_f32_slice(
					gl.get_uniform_location(self.program, "scene_mat_color")
						.as_ref(),
					bytemuck::cast_slice(&fill_50(&data.scene.mat_color)),
				);

				gl.uniform_1_f32_slice(
					gl.get_uniform_location(self.program, "scene_mat_ior")
						.as_ref(),
					&fill_50(&data.scene.mat_ior),
				);

				gl.uniform_1_f32_slice(
					gl.get_uniform_location(self.program, "scene_mat_specular")
						.as_ref(),
					&fill_50(&data.scene.mat_specular),
				);

				gl.uniform_1_f32_slice(
					gl.get_uniform_location(self.program, "scene_mat_roughness")
						.as_ref(),
					&fill_50(&data.scene.mat_roughness),
				);

				gl.uniform_1_f32_slice(
					gl.get_uniform_location(self.program, "scene_mat_emissive_strength")
						.as_ref(),
					&fill_50(&data.scene.mat_emissive_strength),
				);

				gl.uniform_1_f32_slice(
					gl.get_uniform_location(self.program, "scene_mat_transmissive_opacity")
						.as_ref(),
					&fill_50(&data.scene.mat_transmissive_opacity),
				);

				// transforms
				gl.uniform_matrix_4_f32_slice(
					gl.get_uniform_location(self.program, "scene_transform")
						.as_ref(),
					false, // no transpose, it's already in column-major order
					flatten_matrices(&fill_50(&data.scene.transform)),
				);

				gl.uniform_matrix_4_f32_slice(
					gl.get_uniform_location(self.program, "scene_inv_transform")
						.as_ref(),
					false, // no transpose, it's already in column-major order
					flatten_matrices(&fill_50(&data.scene.inv_transform)),
				);

				gl.uniform_matrix_4_f32_slice(
					gl.get_uniform_location(self.program, "scene_normal_transform")
						.as_ref(),
					false, // no transpose, it's already in column-major order
					flatten_matrices(&fill_50(&data.scene.normal_transform)),
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

				// sun color
				gl.uniform_3_f32(
					gl.get_uniform_location(self.program, "sun_color").as_ref(),
					data.settings.world.sun_color[0],
					data.settings.world.sun_color[1],
					data.settings.world.sun_color[2],
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

				gl.uniform_1_u32(
					gl.get_uniform_location(self.program, "accumulate").as_ref(),
					data.settings.render.accumulate as u32,
				);

				gl.uniform_1_u32(
					gl.get_uniform_location(self.program, "samples_per_frame")
						.as_ref(),
					data.settings.render.samples_per_frame,
				);

				// highlight selected object
				gl.uniform_1_u32(
					gl.get_uniform_location(self.program, "highlight_selected")
						.as_ref(),
					data.settings.render.highlight as u32,
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

	fn apply_uniforms_common(&self, gl: &Context, program: Program) {
		unsafe {
			gl.uniform_2_f32(
				gl.get_uniform_location(program, "scr_size").as_ref(),
				self.scr_size.x,
				self.scr_size.y,
			);

			gl.uniform_1_u32(
				gl.get_uniform_location(program, "frame_index").as_ref(),
				self.frame_index,
			);
		}
	}
}

// {{{ gl helpers
unsafe fn screen_sized_texture(gl: &Context, scr_size: glm::Vec2, params: bool) {
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

	if params {
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
	}
}

unsafe fn framebuffer_texture(gl: &Context, texture: Texture) {
	gl.framebuffer_texture_2d(
		glow::FRAMEBUFFER,
		glow::COLOR_ATTACHMENT0,
		glow::TEXTURE_2D,
		Some(texture),
		0,
	);
}
// }}}
