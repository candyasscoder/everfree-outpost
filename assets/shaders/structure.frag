precision mediump float;

#extension GL_EXT_frag_depth : enable

uniform sampler2D sheetTex;
uniform sampler2D depthTex;

varying vec2 normalizedTexCoord;

void main(void) {
    vec4 color = texture2D(sheetTex, normalizedTexCoord);
    if (color.a == 0.0) {
        discard;
    } else {
        gl_FragColor = color;
    }
    // Interpolated depth steps by 1/512, while color values step by 1/255.
    gl_FragDepthEXT = gl_FragCoord.z -
        (255.0 / 512.0) * texture2D(depthTex, normalizedTexCoord).r;
    //gl_FragColor = vec4(1.0, 0.0, 0.0, 1.0);
}
