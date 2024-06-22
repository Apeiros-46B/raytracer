// vim:commentstring=//%s
precision mediump float;
precision mediump usampler2D;

uniform usampler2D image;
uniform vec2 scr_size;
uniform uint frame_index;
uniform uint accumulate;

out vec4 out_color;

// https://knarkowicz.wordpress.com/2016/01/06/aces-filmic-tone-mapping-curve/
vec3 aces_filmic(vec3 x) {
	float a = 2.51;
	float b = 0.03;
	float c = 2.43;
	float d = 0.59;
	float e = 0.14;
	return clamp((x * (a * x + b)) / (x * (c * x + d) + e), 0.0, 1.0);
}

void main() {
	vec2 uv = gl_FragCoord.xy / scr_size;
	uvec3 texel = texture(image, uv).rgb;
	vec3 color = uintBitsToFloat(texel);
	if (accumulate == 1u) {
		color /= float(frame_index);
	}

	// HDR to LDR
	color = aces_filmic(color);

	// linear to sRGB
	color = pow(color, vec3(1.0 / 2.2));

	out_color = vec4(color, 1.0);
}
