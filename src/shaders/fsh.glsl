// vim:commentstring=//%s
precision mediump float;
precision mediump usampler2D;

out vec4 out_color;

// {{{ typedefs
struct Ray {
	vec3 origin;
	vec3 direction;
};

struct RayHit {
	bool hit;
	vec3 pos;
	vec3 norm;
	float dist;
};
// }}}

const vec3 TODO = vec3(0);
const uint MAX_SPHERES = 50u;
const RayHit NO_HIT = RayHit(false, vec3(0.0), vec3(0.0), 0.0);

// {{{ uniforms
uniform vec2 scr_size;
uniform vec3 camera_pos;
uniform uint frame_index;

uniform uint scene_size;
uniform mat4 scene_transforms[MAX_SPHERES];
uniform mat4 scene_inv_transforms[MAX_SPHERES];
uniform mat4 scene_trans_transforms[MAX_SPHERES];

uniform vec3 sky_color;
uniform vec3 sun_dir;
uniform float sun_strength;

uniform uint max_bounces;

// passed as a texture from our prepass shader
uniform usampler2D ray_directions;
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

vec3 left_transform(vec3 src, mat4 m) {
	return vec3(m * vec4(src, 1.0));
}

vec3 right_transform(vec3 src, mat4 m) {
	return vec3(vec4(src, 1.0) * m);
}

// {{{ intersection functions
float ray_sphere_intersection(Ray ray, mat4 im) {
	vec3 origin = left_transform(ray.origin, im);
	vec3 direction = normalize(left_transform(ray.direction, im));

	float a = dot(direction, direction);
	float b = 2.0 * dot(origin, direction);
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
RayHit intersect_sphere(Ray ray, mat4 m, mat4 im, mat4 tm) {
	vec3 origin = left_transform(ray.origin, im);
	vec3 direction = normalize(left_transform(ray.direction, im));

	float a = dot(direction, direction);
	float b = 2.0 * dot(origin, direction);
	float c = dot(origin, origin) - 1; // 1 = radius^2 = 1^2 = 1
	float discriminant = b * b - 4.0 * a * c;

	if (discriminant >= 0.0) {
		float t = (-b - sqrt(discriminant)) / (2.0 * a);
		return RayHit(t >= 0.0, TODO, TODO, t);
	} else {
		return NO_HIT;
	}
}
// }}}

vec3 current_ray_dir() {
	uvec3 texel = texture(ray_directions, gl_FragCoord.xy / scr_size).rgb;
	return vec3(uintBitsToFloat(texel));
}

void main() {
	Ray primary = Ray(camera_pos, current_ray_dir());
	bool did_hit = false;

	for (uint i = 0u; i < MAX_SPHERES; i++) {
		if (i == scene_size) {
			break;
		}

		mat4 im = scene_inv_transforms[i];
		float t = ray_sphere_intersection(primary, im);

		if (t != -1.0) {
			mat4 m = scene_transforms[i];
			mat4 tm = scene_trans_transforms[i];

			// float hit_distance = length(left_transform(primary.direction * t, im));
			// vec3 hit_pos = left_transform(primary.origin, im) + left_transform(primary.direction, im) * hit_distance;

			// vec3 hit_pos = left_transform(primary.origin + primary.direction * t, im);

			// vec3 hit_pos = left_transform(primary.origin, im) + left_transform(primary.direction, im) * t;

			vec3 hit_pos = left_transform(primary.origin + primary.direction * t, tm);

			// color
			// float light_fac = max(dot(normalize(hit_pos), left_transform(sun_dir, im)), 0.0);
			// light_fac *= sun_strength;
			// out_color = vec4(vec3(light_fac), 1);

			// normal
			out_color = vec4(normalize(hit_pos), 1);

			// white
			// out_color = vec4(1);

			did_hit = true;
			break;
		}
	}

	if (!did_hit) {
		out_color = vec4(0, 0, 0, 1);
	}
}
