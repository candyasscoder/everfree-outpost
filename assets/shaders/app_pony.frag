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

uniform sampler2D sheetBase;
uniform sampler2D sheetMane;
uniform sampler2D sheetTail;
uniform sampler2D sheetEyes;
uniform sampler2D sheetEquip[3];
uniform bool hasEquip[3];

uniform vec3 colorBody;
uniform vec3 colorHair;

void layer(inout vec4 cur, sampler2D tex) {
    vec4 samp = texture2D(tex, normalizedTexCoord);
    if (samp.a > cur.a) {
        cur = samp;
    }
}

void layerTinted(inout vec4 cur, sampler2D tex, vec3 tint) {
    vec4 samp = texture2D(tex, normalizedTexCoord);
    if (samp.a > cur.a) {
        cur = vec4(samp.rgb * tint, samp.a);
    }
}

void main(void) {
    float radius = max(cameraSize.x, cameraSize.y) * sliceFrac * 0.5;
    vec2 off = (gl_FragCoord.xy - 0.5 * cameraSize) / radius;

    if (dot(off, off) <= 1.0) {
        discard;
    }

    vec4 result = vec4(0.0);
    layerTinted(result, sheetBase, colorBody);
    layer(result, sheetEyes);
    layerTinted(result, sheetMane, colorHair);
    layerTinted(result, sheetTail, colorHair);
    for (int i = 0; i < 3; ++i) {
        if (hasEquip[i]) {
            layer(result, sheetEquip[i]);
        }
    }

    if (result.a == 0.0) {
        discard;
    }

    emit(0, vec4(result.rgb, 1.0));
    emit(1, vec4(0.0, 0.0, 0.0, 1.0));
}
