// vim:commentstring=//%s
precision mediump float;
precision mediump usampler2D;

out uvec4 out_color;
uniform usampler2D ray_dirs;
uniform usampler2D image;

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
const uint RENDER_FRESNEL   = 5u;
const uint RENDER_ROUGHNESS = 6u;

const uint OBJ_TYPE_SPHERE = 0u;
const uint OBJ_TYPE_BOX    = 1u;

const uint MAT_TYPE_SOLID        = 0u;
const uint MAT_TYPE_EMISSIVE     = 1u;
const uint MAT_TYPE_TRANSMISSIVE = 2u;
// }}}

// 0x7f7f_fff = 0b0_11111110_11111111111111111111111 = 2139095039
const float MAX_FLOAT = intBitsToFloat(2139095039);

uniform vec2 scr_size;
uniform vec3 camera_pos;
uniform uint frame_index;
uniform uint accumulate;

// {{{ scene
const uint MAX_SCENE_SIZE = 50u;

// general
uniform uint scene_selected;
uniform uint scene_size;
uniform uint scene_obj_type[MAX_SCENE_SIZE];

// materials
uniform uint scene_obj_mat_type[MAX_SCENE_SIZE];
uniform vec3 scene_obj_mat_color[MAX_SCENE_SIZE];
uniform float scene_obj_mat_roughness[MAX_SCENE_SIZE];
uniform float scene_obj_mat_emissive_strength[MAX_SCENE_SIZE];
uniform float scene_obj_mat_transmissive_opacity[MAX_SCENE_SIZE];
uniform float scene_obj_mat_transmissive_ior[MAX_SCENE_SIZE];

// transforms
uniform mat4 scene_transform[MAX_SCENE_SIZE];
uniform mat4 scene_inv_transform[MAX_SCENE_SIZE];
uniform mat4 scene_normal_transform[MAX_SCENE_SIZE];
// }}}

// {{{ settings
// world
uniform vec3 sky_color;
uniform vec3 sun_color;
uniform vec3 sun_dir;
uniform float sun_strength;

// render
uniform uint render_mode;
uniform uint highlight_selected;
uniform uint max_bounces;
// }}}

// {{{ random sampling
// float hash(float seed) {
// 	return fract(sin(seed) * 43758.5453);
// }

float hash(float seed) {
	seed = fract(seed * .1031);
	seed *= seed + 33.33;
	seed *= seed + seed;
	return fract(seed);
}

vec3 random_in_hemisphere(float seed, vec3 normal) {
	float a = hash(seed);
	float b = hash(a);
	float c = hash(b);

	vec3 res = vec3(a, b, c) * 2.0 - 1.0;
	if (dot(res, normal) < 0.0) {
		res = -res;
	}
	return res;
}

// adapted from https://www.shadertoy.com/view/Xtt3Wn
vec3 cos_dir(float seed, vec3 nor) {
	vec3 tc = vec3(1.0 + nor.z - nor.xy * nor.xy, -nor.x * nor.y) / (1.0 + nor.z);
	vec3 uu = vec3(tc.x, tc.z, -nor.x);
	vec3 vv = vec3(tc.z, tc.y, -nor.y);

	float u = hash(78.233 + seed);
	float v = hash(10.873 + seed);
	float a = 6.283185 * v;

	return sqrt(u) * (cos(a) * uu + sin(a) * vv) + sqrt(1.0 - u) * nor;
}
// }}}

// {{{ transformation and ray helpers
vec3 transform(vec3 src, mat4 m) {
	return (m * vec4(src, 1.0)).xyz;
}

vec3 transform_n(vec3 src, mat4 m) {
	return normalize((m * vec4(src, 1.0)).xyz);
}

Ray transform(Ray src, mat4 m) {
	return Ray(
		transform(src.origin, m),
		// the zero here is NOT a mistake. this is needed to transform dir correctly
		// (discovered from https://iquilezles.org/articles/boxfunctions/)
		normalize((m * vec4(src.dir, 0.0)).xyz)
	);
}

vec3 pos_from_ray(Ray ray, float t) {
	return ray.origin + ray.dir * t;
}

vec3 pos_from_ray(Ray ray, float t, mat4 m) {
	return transform(ray.origin + ray.dir * t, m);
}

// extremely unrealistic approximation of fresnel
float fresnel(vec3 incident, vec3 normal) {
	return pow(clamp(dot(incident, normal) + 1.0, 0.0, 1.0), 3.0) * 0.3;
	// return pow(smoothstep(0.0, 1.0, dot(incident, normal) + 1.0), 3.0) * 0.3;
}
// }}}

// {{{ intersections
const RayHit NO_HIT = RayHit(false, 0u, vec3(0.0), vec3(0.0), MAX_FLOAT);

// adapted from https://medium.com/@bromanz/another-view-on-the-classic-ray-aabb-intersection-algorithm-for-bvh-traversal-41125138b525
bool intersect_aabb(Ray ray, vec3 corner0, vec3 corner1) {
	// {{{
	vec3 inv = 1.0 / ray.dir;
	vec3 t0 = (corner0 - ray.origin) * inv;
	vec3 t1 = (corner1 - ray.origin) * inv;
	vec3 tmin = min(t0, t1);
	vec3 tmax = max(t0, t1);

	float tn = max(tmin.x, max(tmin.y, tmin.z));
	float tf = min(tmax.x, min(tmax.y, tmax.z));

	return (tn <= tf);
	// }}}
}

// adapted from The Cherno's series
RayHit intersect_sphere(Ray ray, uint i) {
	// {{{
	Ray local_ray = transform(ray, scene_inv_transform[i]);

	// quadratic formula
	// a is dot(dir, dir) which is 1 because dir is normalized
	// (dot product of two identical normalized vecs is 1)
	// b would have a factor of 2 but it cancels with qf denominator
	// c has a sub1 because radius^2 = 1^2 = 1
	float b = dot(local_ray.origin, local_ray.dir);
	float c = dot(local_ray.origin, local_ray.origin) - 1.0;

	float d = b * b - c;
	if (d < 0.0) return NO_HIT;

	float e = sqrt(d);

	float local_tn = (-b - e);
	// float local_tx = (-b + e);
	// if (local_tn > local_tx || local_tx < 0.0) return NO_HIT;
	if (local_tn < 0.0) return NO_HIT;

	// float local_t = (local_tn > 0.0) ? local_tx : local_tn

	vec3 local_pos = pos_from_ray(local_ray, local_tn);
	vec3 pos = transform(local_pos, scene_transform[i]);
	// in local space, the sphere is centered on the origin and has radius 1
	// the local position of the ray hit is automatically equal to the local normal
	vec3 normal = transform_n(local_pos, scene_normal_transform[i]);
	float distance = distance(ray.origin, pos);

	return RayHit(true, i, pos, normal, distance);
	// }}}
}

// adapted from https://iquilezles.org/articles/intersectors/
RayHit intersect_box(Ray ray, uint i) {
	// {{{
	Ray local_ray = transform(ray, scene_inv_transform[i]);

	vec3 inv = 1.0 / local_ray.dir;
	vec3 n = inv * local_ray.origin;
	vec3 k = abs(inv); // box size is (1, 1, 1); no need to multiply it
	vec3 t1 = -n - k;
	vec3 t2 = -n + k;

	// enter and exit
	float local_tn = max(max(t1.x, t1.y), t1.z);
	float local_tx = min(min(t2.x, t2.y), t2.z);

	if (local_tn > local_tx || local_tx < 0.0 || local_tn < 0.0) return NO_HIT;

	vec3 pos = pos_from_ray(local_ray, local_tn, scene_transform[i]);
	vec3 normal = transform_n(
		step(vec3(local_tn), t1) * -sign(local_ray.dir),
		scene_normal_transform[i]
	);
	float distance = distance(ray.origin, pos);

	return RayHit(true, i, pos, normal, distance);
	// }}}
}

RayHit intersect_box_back(Ray ray, uint i) {
	// {{{
	Ray local_ray = transform(ray, scene_inv_transform[i]);
	vec3 inv = 1.0 / local_ray.dir;
	vec3 t2 = -(inv * local_ray.origin) + abs(inv);

	// exit only
	float local_tx = min(min(t2.x, t2.y), t2.z);
	if (local_tx < 0.0) return NO_HIT;

	vec3 pos = pos_from_ray(local_ray, local_tx, scene_transform[i]);
	vec3 normal = transform_n(
		step(t2, vec3(local_tx)) * -sign(local_ray.dir),
		scene_normal_transform[i]
	);
	float distance = distance(ray.origin, pos);

	return RayHit(true, i, pos, normal, distance);
	// }}}
}
// }}}

RayHit intersect_obj(Ray ray, uint i) {
	switch (scene_obj_type[i]) {
		case OBJ_TYPE_SPHERE:
			return intersect_sphere(ray, i);
		case OBJ_TYPE_BOX:
			return intersect_box(ray, i);
	}
}

RayHit intersect_world(Ray ray) {
	RayHit hit = NO_HIT;
	for (uint i = 0u; i < scene_size; i++) {
		RayHit new_hit = intersect_obj(ray, i);
		if (hit.distance > new_hit.distance) {
			hit = new_hit;
		}
	}
	return hit;
}

// vec3 path_trace(Ray ray, uint seed) {
vec3 path_trace(Ray ray, float seed) {
	vec3 light = vec3(0.0);
	vec3 contribution = vec3(1.0);

	for (uint i = 0u; i <= max_bounces; i++) {
		RayHit hit = intersect_world(ray);

		if (!hit.hit) {
			light += contribution * sky_color;
			break;
		}

		uint j = hit.obj;
		uint m = scene_obj_mat_type[j];

		if (m == MAT_TYPE_SOLID) {
			contribution *= scene_obj_mat_color[j];
		} else if (m == MAT_TYPE_EMISSIVE) {
			light += contribution
			       * scene_obj_mat_color[j]
			       * scene_obj_mat_emissive_strength[j];
			break;
		} else if (m == MAT_TYPE_TRANSMISSIVE) {
			// TODO
		}

		vec3 diffuse = random_in_hemisphere(seed, hit.normal);
		// vec3 diffuse = cos_dir(seed, hit.normal);
		vec3 specular = reflect(ray.dir, hit.normal);
		float r = scene_obj_mat_roughness[hit.obj];
		r = max(r - fresnel(ray.dir, hit.normal), 0.0);

		ray.origin = hit.pos;
		ray.dir = normalize((r * diffuse + (1.0 - r) * specular) * 0.5);
	}

	return light;
}

// vec3 get_color(Ray primary, uint seed) {
vec3 get_color(Ray primary, float seed) {
	if (render_mode == RENDER_REALISTIC) {
		return path_trace(primary, seed);
	}

	RayHit hit = intersect_world(primary);

	if (!hit.hit) {
		return sky_color;
	}
	
	switch (render_mode) {
		case RENDER_PREVIEW:
			vec3 light_fac = dot(hit.normal, sun_dir) * sun_strength * sun_color;
			vec3 addend = (highlight_selected == 1u) && (hit.obj == scene_selected)
						? vec3(0.4, 0.2, 0.1)
						: vec3(0.0);
			return (scene_obj_mat_color[hit.obj] * light_fac) + addend;
		case RENDER_POSITION:
			return hit.pos / 2.0 + 0.5;
		case RENDER_NORMAL:
			return hit.normal / 2.0 + 0.5;
		case RENDER_DEPTH:
			return vec3(hit.distance / 100.0);
		case RENDER_FRESNEL:
			return vec3(fresnel(primary.dir, hit.normal));
		case RENDER_ROUGHNESS:
			float r = scene_obj_mat_roughness[hit.obj];
			// return vec3(max(r - fresnel(primary.dir, hit.normal), 0.0));
			return vec3(smoothstep(0.0, fresnel(primary.dir, hit.normal), r));
	}
}

Ray primary_ray(vec2 uv) {
	uvec3 texel = texture(ray_dirs, uv).rgb;
	return Ray(camera_pos, vec3(uintBitsToFloat(texel)));
}

void main() {
	vec2 uv = gl_FragCoord.xy / scr_size;
	float sample = float(frame_index);
	float rng_seed = (uv.x + uv.y) * sample;

	// TODO: randomly skew a tiny bit for free "anti aliasing"
	Ray primary = primary_ray(uv);
	vec3 color = get_color(primary, rng_seed);
	if (frame_index > 1u && accumulate == 1u) {
		color += uintBitsToFloat(texture(image, uv).rgb);
	}
	out_color = floatBitsToUint(vec4(color, 1.0));
}
