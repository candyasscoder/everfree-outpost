precision mediump float;

#extension GL_EXT_frag_depth : enable

varying vec2 texCoord;

uniform sampler2D upperImageTex;
uniform sampler2D upperDepthTex;
uniform sampler2D lowerImageTex;
uniform sampler2D lowerDepthTex;

uniform vec2 cameraSize;
uniform float sliceFrac;

void main(void) {
    float radius = max(cameraSize.x, cameraSize.y) * sliceFrac * 0.5;
    vec2 off = (gl_FragCoord.xy - 0.5 * cameraSize) / radius;

    if (dot(off, off) <= 1.0) {
        gl_FragColor = texture2D(lowerImageTex, texCoord);
        gl_FragDepthEXT = texture2D(lowerDepthTex, texCoord).r;
    } else {
        gl_FragColor = texture2D(upperImageTex, texCoord);
        gl_FragDepthEXT = texture2D(upperDepthTex, texCoord).r;
    }
}
