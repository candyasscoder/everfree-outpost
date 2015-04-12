precision mediump float;

attribute vec2 posOffset;

varying vec2 texCoord;

const mat4 transform = mat4(
        2.0,  0.0,  0.0,  0.0,
        0.0,  2.0,  0.0,  0.0,
        0.0,  0.0,  1.0,  0.0,
       -1.0, -1.0,  0.0,  1.0
       );

void main(void) {
    gl_Position = transform * vec4(posOffset, 0.0, 1.0);
    texCoord = posOffset;
}
