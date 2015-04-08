precision mediump float;

uniform sampler2D atlasTex;

varying vec2 normalizedTexCoord;

void main(void) {
    vec4 color = texture2D(atlasTex, normalizedTexCoord);
    if (color.a == 0.0) {
        discard;
    } else {
        gl_FragColor = color;
    }
}
