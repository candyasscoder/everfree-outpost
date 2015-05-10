precision mediump float;

uniform float cursorRadius;

varying highp vec2 pixelOffset;

void main(void) {
    float dist = max(abs(pixelOffset.x), abs(pixelOffset.y));
    if (dist >= cursorRadius - 2.0) {
        gl_FragColor = vec4(0.0, 0.0, 0.0, 1.0);
    } else {
        discard;
    }
}
