// vim:commentstring=//%s
precision mediump float;
precision mediump usampler2D;

uniform usampler2D seeds;
uniform uint frame_index;

out uvec4 out_color;

float rand1f(float p) {
	p = fract(p * 443.897);
	p *= p + 33.33;
	p *= p + p;
	return fract(p);
}

float rand1f(vec2 p) {
	vec3 p3 = fract(p.xyx * .1031);
	p3 += dot(p3, p3.yzx + 33.33);
	return fract((p3.x + p3.y) * p3.z);
}

void main() {
	vec2 uv = gl_FragCoord.xy / scr_size;

	float value;
	if (frame_index == 1u) {
		value = rand1f(uv)
	} else {
		value = rand1f(uintBitsToFloat(texture(seeds).r));
	}

	out_color = uvec4(floatBitsToUint(value), 0u, 0u, 0u);
}
