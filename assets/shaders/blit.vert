precision mediump float;

attribute vec2 posOffset;

uniform vec2 rectPos;
uniform vec2 rectSize;
uniform vec2 cameraPos;
uniform vec2 cameraSize;

varying vec2 texCoord;

const mat4 transform = mat4(
        2.0,  0.0,  0.0,  0.0,
        0.0, -2.0,  0.0,  0.0,
        0.0,  0.0,  1.0,  0.0,
       -1.0,  1.0,  0.0,  1.0
       );

void main(void) {
    vec2 pxPos = rectPos + posOffset * rectSize - cameraPos;
    gl_Position = transform * vec4(pxPos / cameraSize, 0.0, 1.0);
    texCoord = posOffset;
    //texCoord = posOffset * vec2(1.0, -1.0) + vec2(0.0, 1.0);
}

