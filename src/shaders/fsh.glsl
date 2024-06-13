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
uniform mat4 scene_trans_transforms[MAX_SCENE_SIZE];
uniform mat4 scene_rot_transforms[MAX_SCENE_SIZE];
uniform mat4 scene_inv_rot_transforms[MAX_SCENE_SIZE];

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
vec3 ltrans(vec3 src, mat4 m) {
	return vec3(m * vec4(src, 1.0));
}

// {{{ intersection functions
float ray_sphere_intersection(Ray ray, mat4 im) {
	vec3 origin = ltrans(ray.origin, im);
	vec3 dir = normalize(ltrans(ray.dir, im));

	float a = dot(dir, dir);
	float b = 2.0 * dot(origin, dir);
	float c = dot(origin, origin) - 1; // 1 = radius^2 = 1^2 = 1
	float discriminant = b * b - 4.0 * a * c;

	if (discriminant >= 0.0) {
		float t = (-b - sqrt(discriminant)) / (2.0 * a);
		if (t >= 0.0) {
			return t;
		} else {
			return -1.0;
		}
	} else {
		return -1.0;
	}
}
// }}}

// {{{ new intersection functions
// adapted from https://medium.com/@bromanz/another-view-on-the-classic-ray-aabb-intersection-algorithm-for-bvh-traversal-41125138b525
bool intersect_aabb(Ray ray, vec3 corner0, vec3 corner1) {
	vec3 inv = 1.0 / ray.dir;
	vec3 t0 = (corner0 - ray.origin) * inv;
	vec3 t1 = (corner1 - ray.origin) * inv;
	vec3 tmin = min(t0, t1);
	vec3 tmax = max(t0, t1);

	float min_component = max(tmin.x, max(tmin.y, tmin.z));
	float max_component = min(tmax.x, min(tmax.y, tmax.z));

	return (min_component <= max_component);
}

// adapted from The Cherno's series
RayHit intersect_sph(Ray ray, mat4 m, mat4 im, mat4 tm) {
	vec3 origin = ltrans(ray.origin, im);
	vec3 dir = normalize(ltrans(ray.dir, im));

	// quadratic formula coefficients
	// a is dot(dir, dir) which is 1
	float b = 2.0 * dot(origin, dir);
	float c = dot(origin, origin) - 1; // 1 = radius^2 = 1^2 = 1
	float discrim = b * b - 4.0 * c;

	if (discrim < 0.0) return NO_HIT;

	float t = (-b - sqrt(discrim)) / 2.0;

	return RayHit(t > 0.0, TODO, TODO, t);
}

// adapted from https://iquilezles.org/articles/intersectors/
RayHit intersect_box(Ray ray, uint i) {
	mat4 m = scene_transforms[i];
	mat4 im = scene_inv_transforms[i];
	mat4 rm = scene_rot_transforms[i];

	vec3 origin = ltrans(ray.origin, im);
	vec3 dir = normalize(ltrans(ray.dir, im));
	vec3 inv = 1.0 / dir;

	vec3 n = inv * origin;
	vec3 k = abs(inv); // box size is (1, 1, 1); no need to multiply it
	vec3 t1 = -n - k;
	vec3 t2 = -n + k;

	float t_near = max(max(t1.x, t1.y), t1.z);
	float t_far = min(min(t2.x, t2.y), t2.z);

	if (t_near > t_far || t_far < 0.0) return NO_HIT;

	vec3 pos = ltrans(origin + dir * t_near, m);
	vec3 normal = step(vec3(t_near), t1) * -sign(dir);
	normal = ltrans(normal, rm);

	// TODO: transform t_near
	return RayHit(t_near > 0.0, pos, normal, t_near);
}
// }}}

Ray primary_ray_for_cur_pixel() {
	uvec3 texel = texture(ray_dirs, gl_FragCoord.xy / scr_size).rgb;
	return Ray(camera_pos, vec3(uintBitsToFloat(texel)));
}

void main() {
	Ray primary = primary_ray_for_cur_pixel();
	RayHit hit = intersect_box(primary, 0u);
	if (hit.hit) {
		// out_color = vec4(hit.normal, 1);
		float light_fac = max(dot(hit.normal, sun_dir), 0.0);
		light_fac *= sun_strength;
		out_color = vec4(vec3(light_fac), 1);
	} else {
		out_color = vec4(sky_color, 1);
	}

	// bool did_hit = false;

	// for (uint i = 0u; i < MAX_SCENE_SIZE; i++) {
	// 	if (i == scene_size) {
	// 		break;
	// 	}

	// 	mat4 im = scene_inv_transforms[i];
	// 	float t = ray_sphere_intersection(primary, im);

	// 	if (t != -1.0) {
	// 		mat4 m = scene_transforms[i];
	// 		mat4 tm = scene_trans_transforms[i];

	// 		// TODO: transforms for normals not working
	// 		vec3 hit_pos = ltrans(primary.origin + primary.dir * t, tm);

	// 		// color
	// 		// float light_fac = max(dot(normalize(hit_pos), ltrans(sun_dir, im)), 0.0);
	// 		// light_fac *= sun_strength;
	// 		// out_color = vec4(vec3(light_fac), 1);

	// 		// normal
	// 		out_color = vec4(normalize(hit_pos), 1);

	// 		// white
	// 		// out_color = vec4(1);

	// 		did_hit = true;
	// 		break;
	// 	}
	// }

	// if (!did_hit) {
	// 	out_color = vec4(sky_color, 1);
	// }
}
