precision mediump float;

#extension GL_EXT_draw_buffers : enable

#ifdef GL_EXT_draw_buffers
# define emit(idx, val)   gl_FragData[(idx)] = (val)
#else
# define emit(idx, val)   if (idx == OUTPUT_IDX) gl_FragData[0] = (val)
#endif

varying highp vec2 normalizedTexCoord;

uniform vec2 cameraSize;
uniform float sliceFrac;
uniform sampler2D sheetSampler[8];
uniform vec4 color[8];

void main(void) {
    float radius = max(cameraSize.x, cameraSize.y) * sliceFrac * 0.5;
    vec2 off = (gl_FragCoord.xy - 0.5 * cameraSize) / radius;

    if (dot(off, off) <= 1.0) {
        discard;
    }

    vec4 result = vec4(0.0);
    for (int idx = 0; idx < 8; ++idx) {
        vec4 tex_color = texture2D(sheetSampler[idx], normalizedTexCoord);
        vec4 next = tex_color * color[idx];
        result = mix(result, next, next.a);
    }
    if (result.a == 0.0) {
        discard;
    }

    emit(0, result);
    emit(1, vec4(0.0, 0.0, 0.0, 1.0));
}
