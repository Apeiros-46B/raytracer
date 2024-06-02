// vim:commentstring=//%s
precision mediump float;
precision mediump usampler2D;

const int MAX_SPHERES = 50;

struct Ray {
	vec3 origin;
	vec3 direction;
};

struct RayHit {
	bool hit;
	float distance;
};

struct Sphere {
	float radius;
	vec3 position;
};

out vec4 out_color;

uniform uint frame_index;
uniform vec2 scr_size;

uniform vec3 sky_color;
uniform vec3 sun_dir;
uniform float sun_strength;

uniform uint bounces;

uniform uint sphere_count;
uniform float sphere_radii[MAX_SPHERES];
uniform vec3 sphere_positions[MAX_SPHERES];

uniform vec3 camera_pos;
uniform mat4 camera_inv_proj;
uniform mat4 camera_inv_view;

uniform usampler2D ray_directions;

// 0x7f7f_fff = 0b0_11111110_11111111111111111111111 = 2139095039
const float max_float = intBitsToFloat(2139095039);

float rand_float(inout uint seed) {
	// PCG hash
	uint state = seed * 747796405u + 2891336453u;
	uint word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
	seed = (word >> 22u) ^ word;

	return float(seed) / max_float;
}

vec3 rand_in_unit_sphere(inout uint seed) {
	return normalize(vec3(
		rand_float(seed) * 2.0 - 1.0,
		rand_float(seed) * 2.0 - 1.0,
		rand_float(seed) * 2.0 - 1.0
	));
}

vec3 current_ray_dir() {
	uvec3 texel = texture(ray_directions, gl_FragCoord.xy / scr_size).rgb;
	return vec3(uintBitsToFloat(texel));
}

RayHit ray_sphere_intersection(Ray ray, Sphere sphere) {
	// transform ray origin based on sphere position
	vec3 origin = ray.origin - sphere.position;

	float a = dot(ray.direction, ray.direction);
	float b = 2.0 * dot(origin, ray.direction);
	float c = dot(origin, origin) - sphere.radius * sphere.radius;
	float discriminant = b * b - 4.0 * a * c;

	if (discriminant >= 0.0) {
		float t = (-b - sqrt(discriminant)) / (2.0 * a);
		return RayHit(t >= 0.0, t);
	} else {
		return RayHit(false, 0.0);
	}
}

void main() {
	// uint seed = floatBitsToUint(gl_FragCoord.x + gl_FragCoord.y * scr_size.x);
	// seed *= frame_index;
	//
	// Ray primary = Ray(vec3(0, 0, 2), vec3(current_pixel(), -1));
	// Sphere sp = Sphere(0.5, vec3(0));
	// RayHit hit = ray_sphere_intersection(primary, sp);
	//
	// if (hit.hit) {
	// 	vec3 hit_pos = (primary.origin - sp.position) + primary.direction * hit.distance;
	// 	vec3 normal = normalize(hit_pos);
	//
	// 	float light_fac = max(dot(normal, sun_dir), 0.0);
	// 	light_fac *= sun_strength;
	//
	// 	out_color = vec4(vec3(light_fac), 1);
	// } else {
	// 	out_color = vec4(sky_color, 1);
	// }

	// Ray primary = Ray(vec3(0, 0, 2), vec3(current_pixel(), -1));
	Ray primary = Ray(camera_pos, current_ray_dir());
	bool did_hit = false;

	for (int i = 0; i < MAX_SPHERES; i++) {
		if (i == int(sphere_count)) {
			break;
		}

		Sphere sp = Sphere(sphere_radii[i], sphere_positions[i]);
		RayHit hit = ray_sphere_intersection(primary, sp);
		if (hit.hit) {
		 	vec3 hit_pos = (primary.origin - sp.position) + primary.direction * hit.distance;
		 	float light_fac = max(dot(normalize(hit_pos), sun_dir), 0.0);
		 	light_fac *= sun_strength;

			out_color = vec4(vec3(light_fac), 1);
			did_hit = true;

			break;
		}
	}

	if (!did_hit) {
		out_color = vec4(0, 0, 0, 1);
	}
}
