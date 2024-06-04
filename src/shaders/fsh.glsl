// vim:commentstring=//%s
precision mediump float;
precision mediump usampler2D;

const int MAX_SPHERES = 50;

out vec4 out_color;

// {{{ typedefs
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
// }}}

// {{{ uniforms
uniform vec2 scr_size;
uniform vec3 camera_pos;
uniform uint frame_index;

uniform uint sphere_count;
uniform float sphere_radii[MAX_SPHERES];
uniform vec3 sphere_pos[MAX_SPHERES];
uniform mat4 sphere_transform[MAX_SPHERES];

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

// {{{ intersection functions
RayHit ray_sphere_intersection(Ray ray, Sphere sphere, mat4 transform) {
	// transform ray origin based on sphere position
	// vec3 origin = ray.origin - sphere.position;
	vec3 origin = vec3(transform * vec4(ray.origin, 1.0));

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
// }}}

vec3 current_ray_dir() {
	uvec3 texel = texture(ray_directions, gl_FragCoord.xy / scr_size).rgb;
	return vec3(uintBitsToFloat(texel));
}

void main() {
	Ray primary = Ray(camera_pos, current_ray_dir());
	bool did_hit = false;

	for (int i = 0; i < MAX_SPHERES; i++) {
		if (i == int(sphere_count)) {
			break;
		}

		Sphere sp = Sphere(sphere_radii[i], sphere_pos[i]);
		RayHit hit = ray_sphere_intersection(primary, sp, sphere_transform[i]);

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
