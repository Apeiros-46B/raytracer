// vim:commentstring=//%s
precision mediump float;
precision mediump usampler2D;

uniform usampler2D image;
uniform vec2 scr_size;
uniform uint frame_index;
uniform uint accumulate;

out vec4 out_color;

void main() {
	vec2 uv = gl_FragCoord.xy / scr_size;
	uvec3 texel = texture(image, uv).rgb;
	vec3 color = uintBitsToFloat(texel);
	if (accumulate == 1u) {
		color /= float(frame_index);
	}

	// gamma correction
	color = pow(color, vec3(1.0 / 2.2));

	out_color = vec4(color, 1.0);
}
