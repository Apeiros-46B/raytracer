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
const uint RENDER_RAY_DIR   = 7u;

const uint OBJ_TYPE_SPHERE = 0u;
const uint OBJ_TYPE_BOX    = 1u;

const uint MAT_TYPE_SOLID        = 0u;
const uint MAT_TYPE_EMISSIVE     = 1u;
const uint MAT_TYPE_TRANSMISSIVE = 2u;
// }}}

// 0x7f7f_fff = 0b0_11111110_11111111111111111111111 = 2139095039
const float MAX_FLOAT = intBitsToFloat(2139095039);
const float PI = 3.14159;
const vec3 CAMERA_UP = vec3(0.0, 1.0, 0.0);

uniform vec2 scr_size;
uniform vec3 camera_pos;
uniform vec3 camera_dir;
uniform uint frame_index;

// {{{ scene
const uint MAX_SCENE_SIZE = 50u;

// general
uniform uint scene_selected;
uniform uint scene_size;
uniform uint scene_obj_type[MAX_SCENE_SIZE];

// materials
uniform uint scene_mat_type[MAX_SCENE_SIZE];
uniform vec3 scene_mat_color[MAX_SCENE_SIZE];
uniform float scene_mat_roughness[MAX_SCENE_SIZE];
uniform float scene_mat_emissive_strength[MAX_SCENE_SIZE];
uniform float scene_mat_transmissive_opacity[MAX_SCENE_SIZE];
uniform float scene_mat_transmissive_ior[MAX_SCENE_SIZE];

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
uniform uint accumulate;
uniform uint samples_per_frame;
uniform uint highlight_selected;
uniform uint max_bounces;
// }}}

// {{{ random sampling
// adapted from https://github.com/patriciogonzalezvivo/lygia/blob/main/generative/random.glsl
const vec4 RAND_SCALE = vec4(443.897, 441.423, .0973, .1099);

// for larger ranges
const vec4 RAND_SCALE_HI = vec4(.1031, .1030, .0973, .1099);

float rand1f(float p) {
	p = fract(p * RAND_SCALE.x);
	p *= p + 33.33;
	p *= p + p;
	return fract(p);
}

float rand1f(vec2 p) {
	// this (.1031) is better than using lygia's low range RAND_SCALE
	// (when used for camera ray skewing)
	vec3 p3 = fract(p.xyx * RAND_SCALE_HI.x);
	p3 += dot(p3, p3.yzx + 33.33);
	return fract((p3.x + p3.y) * p3.z);
}

vec2 rand2f(float p) {
	vec3 p3 = fract(p * RAND_SCALE.xyz);
	p3 += dot(p3, p3.yzx + 19.19);
	return fract((p3.xx + p3.yz) * p3.zy);
}

vec3 rand3f(float p) {
	vec3 p3 = fract(p * RAND_SCALE.xyz);
	p3 += dot(p3, p3.yzx + 19.19);
	return fract((p3.xxy + p3.yzz) * p3.zyx);
}

vec3 cos_dist_in_hemi(float seed, vec3 normal) {
	return normalize(normal + (rand3f(seed) * 2.0 - 1.0));
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

// extremely unrealistic approximation of fresnel
float fresnel(vec3 incident, vec3 normal) {
	return pow(clamp(dot(incident, normal) + 1.0, 0.0, 1.0), 3.0) * 0.3;
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

	vec3 pos = transform(pos_from_ray(local_ray, local_tn), scene_transform[i]);
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

	vec3 pos = transform(pos_from_ray(local_ray, local_tx), scene_transform[i]);
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

vec3 path_trace(Ray ray, float seed) {
	vec3 light = vec3(0.0);
	vec3 contribution = vec3(1.0);

	for (uint ray_n = 0u; ray_n <= max_bounces; ray_n++) {
		RayHit hit = intersect_world(ray);

		if (!hit.hit) {
			light += contribution * sky_color;
			light += contribution
			       * sun_color
						 * sun_strength * 100.0
						 * step(0.005, max(dot(ray.dir, sun_dir) - 0.99, 0.0));
			break;
		}

		uint i = hit.obj;
		uint m = scene_mat_type[i];

		if (highlight_selected == 1u && i == scene_selected) {
			light += contribution * vec3(0.4, 0.2, 0.1);
		} else if (m == MAT_TYPE_SOLID) {
			contribution *= scene_mat_color[i];
		} else if (m == MAT_TYPE_EMISSIVE) {
			light += contribution
			       * scene_mat_color[i]
			       * scene_mat_emissive_strength[i];
			break;
		} else if (m == MAT_TYPE_TRANSMISSIVE) {
			// TODO: implement glass
		}

		// TODO: smoother fresnel
		float r = max(scene_mat_roughness[i] - fresnel(ray.dir, hit.normal), 0.0);
		r *= r; // square roughness, makes it feel more linear perceptually

		vec3 diffuse = cos_dist_in_hemi(seed, hit.normal);
		vec3 specular = reflect(ray.dir, hit.normal);

		ray.origin = hit.pos + hit.normal * 0.00001;
		ray.dir = normalize(mix(specular, diffuse, r));
	}

	return (render_mode == RENDER_RAY_DIR) ? (ray.dir * 0.5 + 0.5) : light;
}

// vec3 get_color(Ray primary, uint seed) {
vec3 get_color(Ray primary, float seed) {
	if (render_mode == RENDER_REALISTIC || render_mode == RENDER_RAY_DIR) {
		vec3 color = vec3(0.0);

		// average multiple samples in one frame
		for (uint i = 0u; i < samples_per_frame; i++) {
			Ray ray = primary;

			// "antialias" by skewing the ray direction by a small random offset
			vec2 ofs = (rand2f(seed) * 2.0 - 1.0) / 1000.0;
			ray.dir += (cross(camera_dir, CAMERA_UP) * ofs.x);
			ray.dir += (CAMERA_UP * ofs.y);

			color += path_trace(ray, seed);
			seed = rand1f(seed);
		}
		color /= float(samples_per_frame);

		return color;
	}

	RayHit hit = intersect_world(primary);

	if (!hit.hit) {
		return sky_color;
	}
	
	switch (render_mode) {
		case RENDER_PREVIEW:
			vec3 color = vec3(0.3 - fresnel(primary.dir, hit.normal));
			if (highlight_selected == 1u && hit.obj == scene_selected) {
				color += vec3(0.2, 0.1, 0.05);
			} else {
				color *= scene_mat_color[hit.obj];
			}
			return color;
		case RENDER_POSITION:
			return hit.pos / 2.0 + 0.5;
		case RENDER_NORMAL:
			return hit.normal / 2.0 + 0.5;
		case RENDER_DEPTH:
			return vec3(hit.distance / 100.0);
		case RENDER_FRESNEL:
			return vec3(fresnel(primary.dir, hit.normal));
		case RENDER_ROUGHNESS:
			float r = scene_mat_roughness[hit.obj];
			return vec3(max(r - fresnel(primary.dir, hit.normal), 0.0));
	}
}

Ray get_primary_ray(vec2 uv) {
	uvec3 texel = texture(ray_dirs, uv).rgb;
	return Ray(camera_pos, vec3(uintBitsToFloat(texel)));
}

void main() {
	float seed = rand1f(rand1f(gl_FragCoord.xy) * float(frame_index));
	vec2 uv = gl_FragCoord.xy / scr_size;

	Ray primary = get_primary_ray(uv);
	vec3 color = get_color(primary, seed);
	if (frame_index > 1u && accumulate == 1u) {
		color += uintBitsToFloat(texture(image, uv).rgb);
	}

	// encode into framebuffer (float format isn't color-renderable);
	out_color = floatBitsToUint(vec4(color, 1.0));
}
