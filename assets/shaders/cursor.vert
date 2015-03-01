precision mediump float;

attribute vec2 position;

uniform vec2 cameraPos;
uniform vec2 cameraSize;
uniform vec2 cursorPos;
uniform float cursorRadius;

varying highp vec2 pixelOffset;

void main(void) {
    vec2 signedOffset = position * 2.0 - 1.0;
    pixelOffset = signedOffset * cursorRadius;

    vec2 worldPos = cursorPos + pixelOffset;
    vec2 zeroOne = (worldPos - cameraPos) / cameraSize;
    // OpenGL normally has the Y axis point upward, but we have it point
    // downward instead.
    gl_Position = vec4(zeroOne * vec2(2.0, -2.0) + vec2(-1.0, 1.0), 0.0, 1.0);
}
