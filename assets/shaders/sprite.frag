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
uniform sampler2D imageTex;

void main(void) {
    float radius = max(cameraSize.x, cameraSize.y) * sliceFrac * 0.5;
    vec2 off = (gl_FragCoord.xy - 0.5 * cameraSize) / radius;

    if (dot(off, off) <= 1.0) {
        discard;
    }

    vec4 color = texture2D(imageTex, normalizedTexCoord);
    if (color.a == 0.0) {
        discard;
    }
    emit(0, color);
    emit(1, vec4(0.0, 0.0, 0.0, 1.0));
}
