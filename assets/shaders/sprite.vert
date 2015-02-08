attribute vec2 position;

uniform vec2 cameraPos;
uniform vec2 cameraSize;
uniform vec2 sheetSize;
uniform vec2 base;
uniform vec2 size;
uniform vec2 off;
uniform vec2 flip;

varying highp vec2 normalizedTexCoord;

void main(void) {
    vec2 flippedPos = flip + (1.0 - 2.0 * flip) * position;
    vec2 texCoord = off + flippedPos * size;
    normalizedTexCoord = texCoord / sheetSize;
    
    vec2 px = base + position * size - cameraPos;
    vec2 zeroOne = px / cameraSize;
    gl_Position = vec4(zeroOne * vec2(2.0, -2.0) + vec2(-1.0, 1.0), 0.0, 1.0);
}
