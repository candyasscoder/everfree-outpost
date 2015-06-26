precision mediump float;

#extension GL_EXT_frag_depth : enable

uniform sampler2D sheetTex;
uniform sampler2D depthTex;

varying vec2 normalizedTexCoord;
varying float baseZ;

void main(void) {
    vec4 color = texture2D(sheetTex, normalizedTexCoord);
    if (color.a == 0.0) {
        discard;
    } else if (color.a == 1.0) {
        gl_FragColor = vec4(0.0);
    } else {
        gl_FragColor = color;
    }

    // Same logic as in structure.frag.
    gl_FragDepthEXT = gl_FragCoord.z -
        (255.0 / 512.0) * texture2D(depthTex, normalizedTexCoord).r;
}
