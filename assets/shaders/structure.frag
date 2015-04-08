precision mediump float;

uniform sampler2D sheetTex;

varying vec2 normalizedTexCoord;

void main(void) {
    vec4 color = texture2D(sheetTex, normalizedTexCoord);
    if (color.a == 0.0) {
        discard;
    } else {
        gl_FragColor = color;
    }
    //gl_FragColor = vec4(1.0, 0.0, 0.0, 1.0);
}
