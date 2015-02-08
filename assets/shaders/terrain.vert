attribute vec2 position;
attribute vec2 texCoord;

uniform vec2 atlasSize;
uniform vec2 cameraPos;
uniform vec2 cameraSize;
uniform vec2 chunkPos;

varying highp vec2 normalizedTexCoord;

void main(void) {
    normalizedTexCoord = texCoord / atlasSize;
    
    vec2 px = (chunkPos * 16.0 + position) * 32.0 - cameraPos;
    vec2 zeroOne = px / cameraSize;
    gl_Position = vec4(zeroOne * vec2(2.0, -2.0) + vec2(-1.0, 1.0), 0.0, 1.0);
}
