precision mediump float;

#extension GL_EXT_frag_depth : enable
#extension GL_EXT_draw_buffers : enable

varying vec2 texCoord;

uniform sampler2D image0Tex;
uniform sampler2D image1Tex;
uniform sampler2D depthTex;

void main(void) {
    gl_FragData[0] = texture2D(image0Tex, texCoord);
    gl_FragData[1] = texture2D(image1Tex, texCoord);
    gl_FragDepthEXT = texture2D(depthTex, texCoord).r;
}
