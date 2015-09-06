precision mediump float;

#extension GL_EXT_draw_buffers : enable

#ifdef GL_EXT_draw_buffers
# define emit(idx, val)   gl_FragData[(idx)] = (val)
#else
# define emit(idx, val)   if (idx == OUTPUT_IDX) gl_FragData[0] = (val)
#endif

uniform sampler2D atlasTex;

varying vec2 texCoord;

void main(void) {
    vec4 color = texture2D(atlasTex, texCoord);
    if (color.a == 0.0) {
        discard;
    } else {
        emit(0, color);
        emit(1, vec4(0.0, 0.0, 0.0, 1.0));
    }
    //gl_FragData[0] = vec4(1.0, 0.0, 0.0, 1.0);
}
