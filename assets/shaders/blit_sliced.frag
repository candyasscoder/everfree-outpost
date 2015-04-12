precision mediump float;

#extension GL_EXT_frag_depth : enable
#extension GL_EXT_draw_buffers : enable

varying vec2 texCoord;

uniform sampler2D upperImage0Tex;
uniform sampler2D upperImage1Tex;
uniform sampler2D upperDepthTex;
uniform sampler2D lowerImage0Tex;
uniform sampler2D lowerImage1Tex;
uniform sampler2D lowerDepthTex;

uniform vec2 cameraSize;
uniform float sliceFrac;

void main(void) {
    float radius = max(cameraSize.x, cameraSize.y) * sliceFrac * 0.5;
    vec2 off = (gl_FragCoord.xy - 0.5 * cameraSize) / radius;

    if (dot(off, off) <= 1.0) {
        gl_FragData[0] = texture2D(lowerImage0Tex, texCoord);
        gl_FragData[1] = texture2D(lowerImage1Tex, texCoord);
        gl_FragDepthEXT = texture2D(lowerDepthTex, texCoord).r;
    } else {
        gl_FragData[0] = texture2D(upperImage0Tex, texCoord);
        gl_FragData[1] = texture2D(upperImage1Tex, texCoord);
        gl_FragDepthEXT = texture2D(upperDepthTex, texCoord).r;
    }
}
