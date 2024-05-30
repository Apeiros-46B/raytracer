use egui::Key;
use nalgebra_glm::{
	inverse, look_at, perspective_fov, quat_angle_axis, Mat4, Vec2, Vec3,
};

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
}

const UP_DIR: Vec3 = Vec3::new(0.0, 1.0, 0.0);
const BASE_SPEED: f32 = 5.0;
const BASE_ROT_SPEED: f32 = 0.005;

impl Camera {
	pub fn new(vertical_fov: f32, scr_size: [f32; 2]) -> Self {
		let near_clip = 0.1;
		let far_clip = 100.0;

		let pos: Vec3 = [0.0, 0.0, 3.0].into();
		let forward_dir: Vec3 = [0.0, 0.0, -1.0].into();

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

			scr_size: scr_size.into(),
		}
	}

	// return: whether the camera moved
	pub fn update(&mut self, input: egui::InputState) -> bool {
		let mut moved = false;
		let dt = input.unstable_dt;
		let right_dir = nalgebra_glm::cross(&self.forward_dir, &UP_DIR);

		let mut speed = BASE_SPEED;
		let mut rot_speed = BASE_ROT_SPEED;

		if input.modifiers.shift {
			speed *= 0.2;
			rot_speed *= 0.2;
		}

		if input.key_down(Key::W) {
			self.pos += self.forward_dir * speed * dt;
			log::log!(log::Level::Info, "W");
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

			let q = nalgebra_glm::quat_normalize(&nalgebra_glm::quat_cross(
				&quat_angle_axis(-delta.y, &right_dir),
				&quat_angle_axis(-delta.x, &UP_DIR),
			));

			self.forward_dir = nalgebra_glm::quat_rotate_vec3(&q, &self.forward_dir);

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

	pub fn set_scr_size(&mut self, new_scr_size: [f32; 2]) {
		let new_scr_size: Vec2 = new_scr_size.into();

		if new_scr_size == self.scr_size {
			return;
		}

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
	}

	fn recalc_view(&mut self) {
		self.view = look_at(&self.pos, &(self.pos + self.forward_dir), &UP_DIR);
		self.inv_view = inverse(&self.view);
	}

	// ray directions recalculation is done on the GPU
}
