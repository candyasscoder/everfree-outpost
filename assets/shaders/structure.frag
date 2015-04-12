precision mediump float;

#extension GL_EXT_frag_depth : enable
#extension GL_EXT_draw_buffers : enable

uniform sampler2D sheetTex;
uniform sampler2D depthTex;

varying vec2 normalizedTexCoord;
varying float baseZ;

void main(void) {
    vec4 color = texture2D(sheetTex, normalizedTexCoord);
    if (color.a == 0.0) {
        discard;
    } else {
        gl_FragData[0] = color;
        float tileZ = baseZ / 32.0;
        gl_FragData[1] = vec4(tileZ * 8.0 / 255.0, 0.0, 1.0, 1.0);
    }
    // gl_FragCoord.z steps by 1/512, while color values step by 1/255.  Note
    // that gl_FragCoord varies in the range 0..1, not -1..+1
    gl_FragDepthEXT = gl_FragCoord.z -
        (255.0 / 512.0) * texture2D(depthTex, normalizedTexCoord).r;
}
