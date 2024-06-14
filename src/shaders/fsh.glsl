// vim:commentstring=//%s
precision mediump float;
precision mediump usampler2D;

out vec4 out_color;

// {{{ typedefs
struct Ray {
	vec3 origin;
	vec3 dir;
};

struct RayHit {
	bool hit;
	vec3 pos;
	vec3 normal;
	float distance;
};

// new ver
// struct RayHit {
// 	bool hit;
// 	vec3 normal;
// 	float dist_near;
// 	float dist_far;
// };
// }}}

const mat4 ID = mat4(1.0);
const vec3 TODO = vec3(0);
const uint MAX_SCENE_SIZE = 50u;
const RayHit NO_HIT = RayHit(false, vec3(0.0), vec3(0.0), -1.0);
// const RayHit NO_HIT = RayHit(false, vec3(0.0), -1.0, -1.0);

// {{{ uniforms
uniform vec2 scr_size;
uniform vec3 camera_pos;
uniform uint frame_index;

uniform uint scene_size;
uniform mat4 scene_transforms[MAX_SCENE_SIZE];
uniform mat4 scene_inv_transforms[MAX_SCENE_SIZE];
uniform mat4 scene_normal_transforms[MAX_SCENE_SIZE];

uniform vec3 sky_color;
uniform vec3 sun_dir;
uniform float sun_strength;

uniform uint max_bounces;

// passed as a texture from our prepass shader
uniform usampler2D ray_dirs;
// }}}

// {{{ random number generation, use later for diffuse scattering
// 0x7f7f_fff = 0b0_11111110_11111111111111111111111 = 2139095039
// const float max_float = intBitsToFloat(2139095039);

// float rand_float(inout uint seed) {
// 	// PCG hash
// 	uint state = seed * 747796405u + 2891336453u;
// 	uint word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
// 	seed = (word >> 22u) ^ word;

// 	return float(seed) / max_float;
// }

// vec3 rand_in_unit_sphere(inout uint seed) {
// 	return normalize(vec3(
// 		rand_float(seed) * 2.0 - 1.0,
// 		rand_float(seed) * 2.0 - 1.0,
// 		rand_float(seed) * 2.0 - 1.0
// 	));
// }
// }}}

// translate a vec3 by a mat4, mat multiplied on the left
// {{{ transformation speech
vec3 transform(vec3 src, mat4 m) {
	return vec3(m * vec4(src, 1.0));
}

vec3 transform_n(vec3 src, mat4 m) {
	return normalize(vec3(m * vec4(src, 1.0)));
}

Ray transform(Ray src, mat4 m) {
	return Ray(
		(m * vec4(src.origin, 1.0)).xyz,
		// the zero here is NOT a mistake. this is needed to transform dir correctly
		// see https://iquilezles.org/articles/boxfunctions/
		normalize((m * vec4(src.dir, 0.0)).xyz)
	);
}
// }}}

vec3 pos_from_transform(mat4 m) {
	return m[3].xyz;
}

vec3 pos_from_ray(Ray ray, float t, mat4 m) {
	return transform(ray.origin + ray.dir * t, m);
}

// {{{ intersection functions
// adapted from https://medium.com/@bromanz/another-view-on-the-classic-ray-aabb-intersection-algorithm-for-bvh-traversal-41125138b525
bool intersect_aabb(Ray ray, vec3 corner0, vec3 corner1) {
	// {{{
	vec3 inv = 1.0 / ray.dir;
	vec3 t0 = (corner0 - ray.origin) * inv;
	vec3 t1 = (corner1 - ray.origin) * inv;
	vec3 tmin = min(t0, t1);
	vec3 tmax = max(t0, t1);

	float min_component = max(tmin.x, max(tmin.y, tmin.z));
	float max_component = min(tmax.x, min(tmax.y, tmax.z));

	return (min_component <= max_component);
	// }}}
}

// adapted from The Cherno's series
RayHit intersect_sph(Ray ray, uint i) {
	// {{{
	vec3 orig_origin = ray.origin;
	vec3 orig_dir = ray.dir;
	ray = transform(ray, scene_inv_transforms[i]);

	// quadratic formula
	// a is dot(dir, dir) which is 1 because dir is normalized
	// (dot product of two identical normalized vecs is 1)
	// b would have a factor of 2 but it cancels with qf denominator
	float b = dot(ray.origin, ray.dir);
	float c = dot(ray.origin, ray.origin) - 1; // 1 = radius^2 = 1^2 = 1

	float d = b * b - c;
	if (d < 0.0) return NO_HIT;

	float t = (-b - sqrt(d));
	if (t < 0.0) return NO_HIT;

	vec3 pos = pos_from_ray(ray, t, scene_transforms[i]);
	float tt = distance(orig_origin, pos); // transformed
	vec3 normal = transform(
		orig_origin - pos_from_transform(scene_transforms[i]),
		scene_normal_transforms[i]
	);

	return RayHit(t > 0.0, pos, normal, tt);
	// }}}
}

// adapted from https://iquilezles.org/articles/intersectors/
RayHit intersect_box(Ray ray, uint i) {
	// {{{
	vec3 orig_origin = ray.origin;
	ray = transform(ray, scene_inv_transforms[i]);

	vec3 inv = 1.0 / ray.dir;
	vec3 n = inv * ray.origin;
	vec3 k = abs(inv); // box size is (1, 1, 1); no need to multiply it
	vec3 t1 = -n - k;
	vec3 t2 = -n + k;

	// near and far
	float tn = max(max(t1.x, t1.y), t1.z);
	float tf = min(min(t2.x, t2.y), t2.z);

	if (tn > tf || tf < 0.0) return NO_HIT;

	vec3 pos = pos_from_ray(ray, tn, scene_transforms[i]);
	vec3 normal = transform_n(
		step(vec3(tn), t1) * -sign(ray.dir),
		scene_normal_transforms[i]
	);
	float t = distance(orig_origin, pos); // transformed

	return RayHit(tn > 0.0, pos, normal, t);
	// }}}
}
// }}}

Ray primary_ray_for_cur_pixel() {
	uvec3 texel = texture(ray_dirs, gl_FragCoord.xy / scr_size).rgb;
	return Ray(camera_pos, vec3(uintBitsToFloat(texel)));
}

vec3 get_cur_color() {
	Ray primary = primary_ray_for_cur_pixel();
	RayHit hit = intersect_sph(primary, 0u);
	if (hit.hit) {
		// return hit.normal / 2.0 + 0.5;
		return hit.normal / 2.0 + 0.5;
	} else {
		return sky_color;
	}
}

void main() {
	out_color = vec4(get_cur_color(), 1.0);
}
