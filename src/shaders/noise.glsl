// vim:commentstring=//%s
precision mediump float;
precision mediump usampler2D;

uniform usampler2D noise;
uniform uint frame_index;
uniform vec2 scr_size;

out uvec4 out_color;

uint pcg_hash(uint p) {
	uint state = p * 747796405u + 2891336453u;
	uint word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
	return (word >> 22u) ^ word;
}

void main() {
	vec2 uv = gl_FragCoord.xy / scr_size;
	uint value;

	if (frame_index == 1u) {
		value = uint(gl_FragCoord.y * scr_size.x + gl_FragCoord.x);
	} else {
		value = texture(noise, uv).r;
	}
	value = pcg_hash(value);

	out_color = uvec4(value, 0u, 0u, 0u);
}
