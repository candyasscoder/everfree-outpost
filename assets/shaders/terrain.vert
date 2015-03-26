precision mediump float;

attribute vec3 position;
attribute vec2 texCoord;

uniform vec2 atlasSize;
uniform vec2 cameraPos;
uniform vec2 cameraSize;
uniform vec2 chunkPos;

varying vec2 normalizedTexCoord;
varying vec3 pixelPosition;

void main(void) {
    normalizedTexCoord = texCoord / atlasSize;
    pixelPosition = vec3((chunkPos * 16.0 + position.xy) * 32.0, position.z);
    
    vec2 px = pixelPosition.xy - cameraPos;
    vec2 zeroOne = px / cameraSize;
    gl_Position = vec4(zeroOne * vec2(2.0, -2.0) + vec2(-1.0, 1.0), 0.0, 1.0);
}
