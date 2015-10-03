precision highp float;

const float TILE_SIZE = 32.0;
const float CHUNK_SIZE = 16.0;

#extension GL_EXT_draw_buffers : enable

#ifdef GL_EXT_draw_buffers
# define emit(idx, val)   gl_FragData[(idx)] = (val)
#else
# define emit(idx, val)   if (idx == OUTPUT_IDX) gl_FragData[0] = (val)
#endif

uniform sampler2D sheetTex;
uniform sampler2D depthTex;
uniform vec2 cameraSize;
uniform float sliceRadius;
uniform float sliceZ;

varying vec2 texCoord;
varying float baseZ;
#ifdef OUTPOST_ANIM
varying vec2 renderMin;
varying vec2 renderMax;
#endif

void main(void) {
    if (sliceRadius > 0.0 && baseZ >= sliceZ) {
        vec2 pixelPos = gl_FragCoord.xy - cameraSize * 0.5;
        if (dot(pixelPos, pixelPos) < sliceRadius * sliceRadius) {
            discard;
        }
    }

#ifdef OUTPOST_ANIM
    if (texCoord.x < renderMin.x || texCoord.x >= renderMax.x || 
            texCoord.y < renderMin.y || texCoord.y >= renderMax.y) {
        discard;
    }
#endif

    vec4 color = texture2D(sheetTex, texCoord);
#ifndef OUTPOST_SHADOW
    if (color.a < 1.0) {
        discard;
    } else {
        emit(0, color);
        float tileZ = baseZ;
        emit(1, vec4(tileZ * 8.0 / 255.0, 0.0, 1.0, 1.0));
    }
#else
    if (color.a == 0.0) {
        discard;
    } else if (color.a == 1.0) {
        emit(0, vec4(1.0));
    } else {
        emit(0, color);
    }
#endif
}
