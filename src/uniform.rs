#[derive(Clone, Copy)]
pub struct Uniforms {
	pub scr_size: [f32; 2],

	pub sky_color: [f32; 3],
	pub sun_dir: [f32; 3],
	// pub sun_color: [f32; 3],
	pub sun_strength: f32,

	pub max_bounces: u32,
}

impl Uniforms {
	pub fn new(
		settings: &crate::settings::Settings,
		scr_size: egui::Vec2,
	) -> Self {
		Uniforms {
			scr_size: scr_size.into(),

			sky_color: settings.sky.background_color,
			sun_dir: {
				// TODO: don't recompute this every frame
				let alpha = settings.sky.sun_rotation;
				let beta = settings.sky.sun_elevation;

				let x = alpha.cos() * beta.cos();
				let y = beta.sin();
				let z = alpha.sin() * beta.cos();

				let mag = (x.powi(2) + y.powi(2) + z.powi(2)).sqrt();

				[x / mag, y / mag, z / mag]
			},
			// sun_color: crate::math::blackbody::blackbody(settings.sky.sun_warmth),
			sun_strength: settings.sky.sun_strength,

			max_bounces: settings.render.max_bounces,
		}
	}
}
