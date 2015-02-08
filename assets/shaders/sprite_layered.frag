precision mediump float;

varying highp vec2 normalizedTexCoord;

uniform sampler2D sheetSampler[8];
uniform vec4 color[8];

void main(void) {
    vec4 result = vec4(0.0);
    for (int idx = 0; idx < 8; ++idx) {
        vec4 tex_color = texture2D(sheetSampler[idx], normalizedTexCoord);
        vec4 next = tex_color * color[idx];
        result = mix(result, next, next.a);
    }

    gl_FragColor = result;
}
