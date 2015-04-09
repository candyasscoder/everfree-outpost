precision mediump float;

varying highp vec2 normalizedTexCoord;

uniform sampler2D imageTex;

void main(void) {
    vec4 color = texture2D(imageTex, normalizedTexCoord);
    if (color.a == 0.0) {
        discard;
    }
    gl_FragColor = color;
}
