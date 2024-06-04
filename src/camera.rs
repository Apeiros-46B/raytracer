use egui::Key;
use nalgebra_glm::{
	self as glm, inverse, look_at, perspective_fov, quat_angle_axis, Mat4, Vec2,
	Vec3,
};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Camera {
	vertical_fov: f32,
	near_clip: f32,
	far_clip: f32,

	pub pos: Vec3,
	forward_dir: Vec3,

	proj: Mat4,
	pub inv_proj: Mat4,
	view: Mat4,
	pub inv_view: Mat4,

	scr_size: Vec2,
	pub recalculate_ray_dirs: bool, // actual calculation is offloaded
}

const UP_DIR: Vec3 = Vec3::new(0.0, 1.0, 0.0);
const BASE_SPEED: f32 = 5.0;
const BASE_ROT_SPEED: f32 = 0.005;

const DEFAULT_POS: Vec3 = Vec3::new(0.0, 0.0, 2.0);
const DEFAULT_FORWARD_DIR: Vec3 = Vec3::new(0.0, 0.0, -1.0);

pub const DEFAULT_FOV_DEG: f32 = 70.0_f32;

impl Camera {
	pub fn new(scr_size: Vec2) -> Self {
		let vertical_fov = DEFAULT_FOV_DEG.to_radians();
		let near_clip = 0.1;
		let far_clip = 100.0;

		let pos = DEFAULT_POS;
		let forward_dir = DEFAULT_FORWARD_DIR;

		let proj = perspective_fov(
			vertical_fov,
			scr_size[0],
			scr_size[1],
			near_clip,
			far_clip,
		);
		let view = look_at(&pos, &(pos + forward_dir), &UP_DIR);

		Self {
			vertical_fov,
			near_clip,
			far_clip,

			pos,
			forward_dir,

			proj,
			inv_proj: inverse(&proj),
			view,
			inv_view: inverse(&view),

			scr_size,
			recalculate_ray_dirs: false,
		}
	}

	// return: whether the camera moved
	pub fn update(&mut self, input: egui::InputState) -> bool {
		if input.key_pressed(Key::R) {
			self.pos = DEFAULT_POS;
			self.forward_dir = DEFAULT_FORWARD_DIR;
			self.recalc_view();
			return true;
		}

		let mut moved = false;
		let dt = input.unstable_dt;
		let right_dir = glm::cross(&self.forward_dir, &UP_DIR);

		let mut speed = BASE_SPEED;
		let mut rot_speed = BASE_ROT_SPEED;

		if input.modifiers.shift {
			speed *= 0.2;
			rot_speed *= 0.2;
		}

		if input.key_down(Key::W) {
			self.pos += self.forward_dir * speed * dt;
			moved = true;
		} else if input.key_down(Key::S) {
			self.pos -= self.forward_dir * speed * dt;
			moved = true;
		}

		if input.key_down(Key::A) {
			self.pos -= right_dir * speed * dt;
			moved = true;
		} else if input.key_down(Key::D) {
			self.pos += right_dir * speed * dt;
			moved = true;
		}

		if input.key_down(Key::Q) {
			self.pos -= UP_DIR * speed * dt;
			moved = true;
		} else if input.key_down(Key::E) {
			self.pos += UP_DIR * speed * dt;
			moved = true;
		}

		if input.pointer.secondary_down() && input.pointer.is_moving() {
			let delta = input.pointer.delta() * rot_speed;

			let q = glm::quat_normalize(&glm::quat_cross(
				&quat_angle_axis(-delta.y, &right_dir),
				&quat_angle_axis(-delta.x, &UP_DIR),
			));

			self.forward_dir = glm::quat_rotate_vec3(&q, &self.forward_dir);

			moved = true;
		}

		if moved {
			self.recalc_view();
		}

		moved
	}

	pub fn set_fov(&mut self, new_fov: f32) {
		if (new_fov - self.vertical_fov).abs() <= f32::EPSILON {
			return;
		}

		self.vertical_fov = new_fov;
		self.recalc_proj();
	}

	pub fn set_scr_size(&mut self, new_scr_size: Vec2) {
		// no check with existing scr_size is needed because this is done
		// in the raytracer struct on resize (this logic is also needed there)
		self.scr_size = new_scr_size;
		self.recalc_proj();
	}

	fn recalc_proj(&mut self) {
		self.proj = perspective_fov(
			self.vertical_fov,
			self.scr_size.x,
			self.scr_size.y,
			self.near_clip,
			self.far_clip,
		);
		self.inv_proj = inverse(&self.proj);
		self.recalculate_ray_dirs = true;
	}

	fn recalc_view(&mut self) {
		self.view = look_at(&self.pos, &(self.pos + self.forward_dir), &UP_DIR);
		self.inv_view = inverse(&self.view);
		self.recalculate_ray_dirs = true;
	}
}
