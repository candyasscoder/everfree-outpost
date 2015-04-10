precision mediump float;

#extension GL_EXT_frag_depth : enable

varying vec2 texCoord;

uniform sampler2D imageTex;
uniform sampler2D depthTex;

void main(void) {
    gl_FragColor = texture2D(imageTex, texCoord);
    gl_FragDepthEXT = texture2D(depthTex, texCoord).r;
}
