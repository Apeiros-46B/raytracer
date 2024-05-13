precision mediump float;

in vec4 vert_color;
out vec4 out_color;

uniform vec2 u_scr_size;
uniform vec3 u_sky_color;
uniform uint u_max_bounces;

const vec3 color = vec3(1);
const vec3 pos = vec3(0);
const float radius = 0.5;
const vec3 light_dir = normalize(vec3(-1));

void main() {
	// screen space position of current pixel
	vec2 pixel_pos = gl_FragCoord.xy / u_scr_size * 2.0 - 1.0;
	float aspect_ratio = u_scr_size.x / u_scr_size.y;
	pixel_pos.x *= aspect_ratio;

	// camera/world space
	vec3 ray_orig = vec3(0, 0, 1);
	vec3 ray_dir = vec3(pixel_pos, -1);

	// quadratic formula
	float a = dot(ray_dir, ray_dir);
	float b = 2.0 * dot(ray_orig, ray_dir);
	float c = dot(ray_orig, ray_orig) - radius * radius;
	float discriminant = b * b - 4.0 * a * c;

	if (discriminant >= 0.0) {
		float t = (-b - sqrt(discriminant)) / (2.0 * a);
		vec3 hit = ray_orig + ray_dir * t;
		vec3 normal = normalize(hit);

		float light_fac = max(dot(normal, -light_dir), 0.0);

		// out_color = vec4(normal * 0.5 + 0.5, 1);
		out_color = vec4(color * light_fac, 1);
	} else {
		out_color = vec4(u_sky_color, 1);
	}
}
