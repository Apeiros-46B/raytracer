// vim:commentstring=//%s
precision mediump float;
precision mediump usampler2D;

uniform usampler2D image;
uniform vec2 scr_size;

out vec4 out_color;

void main() {
	vec2 uv = gl_FragCoord.xy / scr_size;
	uvec3 texel = texture(image, uv).rgb;

	// gamma correction
	vec3 color = pow(uintBitsToFloat(texel), vec3(1.0 / 2.2));

	// TODO: other post-processing

	out_color = vec4(color, 1.0);
}
