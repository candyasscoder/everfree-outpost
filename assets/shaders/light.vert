precision mediump float;

attribute vec2 posOffset;

uniform vec3 center;
uniform float radius;
uniform vec2 cameraSize;

varying vec2 texCoord;
varying vec2 pos;

const mat4 transform = mat4(
        2.0,  0.0,  0.0,  0.0,
        0.0,  2.0,  0.0,  0.0,
        0.0,  0.0,  1.0,  0.0,
       -1.0, -1.0,  0.0,  1.0
       );

void main(void) {
    pos = vec2(center.x, center.y - center.z) + posOffset * radius;
    vec2 relPos = pos / cameraSize;
    gl_Position = transform * vec4(relPos, 0.0, 1.0);
    texCoord = relPos;
}
