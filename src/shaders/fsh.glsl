// vim:commentstring=//%s
precision mediump float;
precision mediump usampler2D;

out vec4 out_color;
uniform usampler2D ray_dirs;

// {{{ typedefs
struct Ray {
	vec3 origin;
	vec3 dir;
};

struct RayHit {
	bool hit;
	uint obj;
	vec3 pos;
	vec3 normal;
	float distance;
};

const uint RENDER_PREVIEW   = 0u;
const uint RENDER_REALISTIC = 1u;
const uint RENDER_POSITION  = 2u;
const uint RENDER_NORMAL    = 3u;
const uint RENDER_DEPTH     = 4u;

const uint OBJ_TYPE_SPHERE = 0u;
const uint OBJ_TYPE_BOX    = 1u;
// }}}

// 0x7f7f_fff = 0b0_11111110_11111111111111111111111 = 2139095039
const float MAX_FLOAT = intBitsToFloat(2139095039);
const uint MAX_SCENE_SIZE = 50u;
const RayHit NO_HIT = RayHit(false, 0u, vec3(0.0), vec3(0.0), MAX_FLOAT);

uniform vec2 scr_size;
uniform vec3 camera_pos;
uniform uint frame_index;

// {{{ scene
const uint MAX_SCENE_SIZE = 50u;

// generic
uniform uint scene_size;
uniform uint scene_obj_types[MAX_SCENE_SIZE];

// material
uniform vec3 scene_obj_mat_colors[MAX_SCENE_SIZE];
uniform float scene_obj_mat_roughness[MAX_SCENE_SIZE];

// transform
uniform mat4 scene_transforms[MAX_SCENE_SIZE];
uniform mat4 scene_inv_transforms[MAX_SCENE_SIZE];
uniform mat4 scene_normal_transforms[MAX_SCENE_SIZE];
// }}}

// {{{ settings
// render
uniform vec3 sky_color;
uniform vec3 sun_dir;
uniform float sun_strength;

// world
uniform uint render_mode;
uniform uint max_bounces;
// }}}

// {{{ randomness
// uint pcg_hash(uint input) {
// 	uint state = input * 747796405u + 2891336453u;
// 	uint word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
// 	return (word >> 22u) ^ word;
// }

float rand_float(vec2 uv) {
	return fract(sin(dot(uv, vec2(12.9898,78.233))) * 43758.5453123);
}
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
		// (discovered from https://iquilezles.org/articles/boxfunctions/)
		// (see iq's ro and rd transforms)
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
const RayHit NO_HIT = RayHit(false, vec3(0.0), vec3(0.0), -1.0);

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
	float c = dot(ray.origin, ray.origin) - 1.0; // 1 = radius^2 = 1^2 = 1

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

	return RayHit(true, i, pos, normal, tt);
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

	if (tn > tf || tf < 0.0 || tn < 0.0) return NO_HIT;

	vec3 pos = pos_from_ray(ray, tn, scene_transforms[i]);
	vec3 normal = transform_n(
		step(vec3(tn), t1) * -sign(ray.dir),
		scene_normal_transforms[i]
	);
	float t = distance(orig_origin, pos); // transformed

	return RayHit(true, i, pos, normal, t);
	// }}}
}
// }}}

RayHit intersect_obj(Ray ray, uint i) {
	switch (scene_obj_types[i]) {
		case OBJ_TYPE_SPHERE:
			return intersect_sph(ray, i);
		case OBJ_TYPE_BOX:
			return intersect_box(ray, i);
	}
}

RayHit intersect_world(Ray ray) {
	RayHit hit = NO_HIT;
	for (uint i = 0u; i < MAX_SCENE_SIZE; i++) {
		if (scene_size == i) {
			break;
		}
		RayHit new_hit = intersect_obj(ray, i);
		if (hit.distance > new_hit.distance) {
			hit = new_hit;
		}
	}
	return hit;
}

vec3 path_trace(RayHit hit) {
	// TODO
	return vec3(1);
}

vec3 get_color(RayHit hit) {
	if (hit.hit) {
		switch (render_mode) {
			case RENDER_PREVIEW:
				float light_fac = clamp(dot(hit.normal, sun_dir) * sun_strength, 0.2, 1.0);
				return scene_obj_mat_colors[hit.obj] * light_fac;
			case RENDER_REALISTIC:
				return path_trace(hit, i);
			case RENDER_POSITION:
				return hit.pos / 2.0 + 0.5;
			case RENDER_NORMAL:
				return hit.normal / 2.0 + 0.5;
			case RENDER_DEPTH:
				return vec3(hit.distance / 100.0);
		}
	} else {
		return sky_color;
	}
}

Ray primary_ray(vec2 uv) {
	uvec3 texel = texture(ray_dirs, uv).rgb;
	return Ray(camera_pos, vec3(uintBitsToFloat(texel)));
}

void main() {
	vec2 uv = gl_FragCoord.xy / scr_size;

	uint i = 0u;
	Ray primary = primary_ray(uv);
	RayHit hit = intersect_world(primary);
	vec3 color = pow(get_color(hit), vec3(1.0 / 2.2))
	out_color = vec4(color, 1.0);

	// out_color = vec4(vec3(rand_float(uv)), 1.0);
}
