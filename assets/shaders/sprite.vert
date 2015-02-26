attribute vec2 position;

uniform vec2 cameraPos;
uniform vec2 cameraSize;
uniform vec2 sheetSize;
uniform vec2 srcPos;
uniform vec2 srcSize;
uniform vec2 destPos;
uniform vec2 destSize;

varying highp vec2 normalizedTexCoord;

void main(void) {
    vec2 texCoord = srcPos + position * srcSize;
    normalizedTexCoord = texCoord / sheetSize;

    vec2 worldPos = destPos + position * destSize;
    vec2 zeroOne = (worldPos - cameraPos) / cameraSize;
    // OpenGL normally has the Y axis point upward, but we have it point
    // downward instead.
    gl_Position = vec4(zeroOne * vec2(2.0, -2.0) + vec2(-1.0, 1.0), 0.0, 1.0);
}
