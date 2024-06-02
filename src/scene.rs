// #[repr(u32)]
// pub enum IntersectionType {
// 	Spheroid,
// 	Box,
// }

// #[repr(C)]
// pub struct Object {
// 	ty: IntersectionType,
// 	hole: bool,
// 	scale: [f32; 3],
// 	rotation: [f32; 3],
// 	pos: [f32; 3],
// }

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Scene {
	pub radii: Box<[f32]>,
	pub pos: Box<[f32]>, // 3 times the length
}
