const vec2 verts[3] = vec2[3](
	vec2(-1, -1),
	vec2( 3, -1),
	vec2(-1,  3)
);
const vec4 colors[3] = vec4[3](
	vec4(1, 0, 0, 1),
	vec4(0, 1, 0, 1),
	vec4(0, 0, 1, 1)
);

out vec4 vert_color;

void main() {
	vert_color = colors[gl_VertexID];
	gl_Position = vec4(verts[gl_VertexID], 0, 1);
}
