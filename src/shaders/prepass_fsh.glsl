// vim:commentstring=//%s
precision mediump float;

// screen
uniform vec2 scr_size;

// camera
uniform mat4 inv_proj;
uniform mat4 inv_view;

// not actually a color, the RGB components are float bits encoded as uints
// which represent a vec3 that represents the ray direction for each pixel
// the alpha channel is not used; RGBA32UI is chosen because RGB32UI isn't
// considered "color-renderable" on web targets
out uvec4 out_color;

void main() {
	// adapted from The Cherno's series
	vec2 uv = gl_FragCoord.xy / scr_size * 2.0 - 1.0;
	vec4 target = inv_proj * vec4(uv, 1, 1);
	vec3 dir = vec3(inv_view * vec4(normalize(vec3(target) / target.w), 0));

	out_color = uvec4(floatBitsToUint(dir), 0u);
}
