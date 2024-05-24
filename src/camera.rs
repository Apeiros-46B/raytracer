use nalgebra_glm::{inverse, look_at, perspective_fov, Mat4, Vec2, Vec3};

pub struct Camera {
	vertical_fov: f32,
	near_clip: f32,
	far_clip: f32,

	pos: Vec3,
	dir: Vec3,

	proj: Mat4,
	inv_proj: Mat4,
	view: Mat4,
	inv_view: Mat4,

	scr_size: Vec2,

	should_recalc_rays: bool,
}

const UP: Vec3 = Vec3::new(0.0, 1.0, 0.0);

impl Camera {
	pub fn new(vertical_fov: f32, scr_size: [f32; 2]) -> Self {
		let near_clip = 0.1;
		let far_clip = 100.0;

		let pos: Vec3 = [0.0, 0.0, 3.0].into();
		let dir: Vec3 = [0.0, 0.0, -1.0].into();

		let proj = perspective_fov(
			vertical_fov,
			scr_size[0],
			scr_size[1],
			near_clip,
			far_clip,
		);
		let view = look_at(&pos, &(pos + dir), &UP);

		Self {
			vertical_fov,
			near_clip,
			far_clip,

			pos,
			dir,

			proj,
			inv_proj: inverse(&proj),
			view,
			inv_view: inverse(&view),

			scr_size: scr_size.into(),

			should_recalc_rays: false,
		}
	}

	pub fn update(&mut self, _time_step: web_time::Duration) {
		// TODO: time step is currently broken on web
		
	}

	pub fn resize(&mut self, new_scr_size: [f32; 2]) {
		let new_scr_size: Vec2 = new_scr_size.into();

		if new_scr_size == self.scr_size {
			return;
		}

		self.scr_size = new_scr_size;
		self.recalc_proj();

		// deferred to the fragment shader for the following reasons:
		// - it would be terrible to calculate a ray for every pixel on the CPU
		// - because SSBOs are not available in OpenGL 3.3/WebGL 2, data of a
		//   dynamic size can't be sent to the GPU as a uniform, so calculating
		//   it on the CPU side makes it impossible to even access from the GPU
		self.should_recalc_rays = true;
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
		self.view = look_at(&self.pos, &(self.pos + self.dir), &UP);
		self.inv_view = inverse(&self.view);
	}

	// ray directions recalculation is done on the GPU
}
