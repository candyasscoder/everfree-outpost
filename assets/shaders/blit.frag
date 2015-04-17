precision mediump float;

#extension GL_EXT_frag_depth : enable
#extension GL_EXT_draw_buffers : enable

#ifdef GL_EXT_draw_buffers
# define emit(idx, val)   gl_FragData[(idx)] = (val)
#else
# define emit(idx, val)   if (idx == OUTPUT_IDX) gl_FragData[0] = (val)
#endif

varying vec2 texCoord;

uniform sampler2D image0Tex;
uniform sampler2D image1Tex;
uniform sampler2D depthTex;

void main(void) {
    emit(0, texture2D(image0Tex, texCoord));
    emit(1, texture2D(image1Tex, texCoord));
    gl_FragDepthEXT = texture2D(depthTex, texCoord).r;
}
