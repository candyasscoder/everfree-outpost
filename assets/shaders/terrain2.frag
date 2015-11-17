precision mediump float;

#extension GL_EXT_draw_buffers : enable

#ifdef GL_EXT_draw_buffers
# define emit(idx, val)   gl_FragData[(idx)] = (val)
#else
# define emit(idx, val)   if (idx == OUTPUT_IDX) gl_FragData[0] = (val)
#endif

uniform sampler2D atlasTex;
uniform vec2 cameraSize;
uniform float sliceRadius;
uniform float sliceZ;

varying vec2 texCoord;
varying float baseZ;

void main(void) {
    if (sliceRadius > 0.0 && baseZ >= sliceZ) {
        vec2 pixelPos = gl_FragCoord.xy - cameraSize * 0.5;
        if (dot(pixelPos, pixelPos) < sliceRadius * sliceRadius) {
            discard;
        }
    }

    vec4 color = texture2D(atlasTex, texCoord);
    if (color.a == 0.0) {
        discard;
    } else {
        emit(0, color);
        emit(1, vec4(0.0, 0.0, 0.0, 1.0));
    }
    //gl_FragData[0] = vec4(1.0, 0.0, 0.0, 1.0);
}
