precision mediump float;

#extension GL_EXT_frag_depth : enable

varying vec2 texCoord;

uniform sampler2D depthTex;

void main(void) {
    gl_FragColor = vec4(0.0);
    gl_FragDepthEXT = texture2D(depthTex, texCoord).r;
}
