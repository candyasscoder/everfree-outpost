precision mediump float;

#extension GL_EXT_draw_buffers : enable

uniform sampler2D atlasTex;

varying vec2 normalizedTexCoord;

void main(void) {
    vec4 color = texture2D(atlasTex, normalizedTexCoord);
    if (color.a == 0.0) {
        discard;
    } else {
        gl_FragData[0] = color;
        gl_FragData[1] = vec4(0.0, 0.0, 0.0, 1.0);
    }
}
