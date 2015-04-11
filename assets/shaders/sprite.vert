precision mediump float;

attribute vec2 posOffset;

const mat4 projection =
    mat4( 1.0,  0.0,  0.0,  0.0,
          0.0,  1.0,  0.0,  0.0,
          0.0, -1.0,  1.0,  0.0,
          0.0,  0.0,  0.0,  1.0);

const mat4 scaling =
    mat4( 2.0,  0.0,  0.0,  0.0,
          0.0, -2.0,  0.0,  0.0,
          0.0,  0.0,  2.0,  0.0,
         -1.0,  1.0, -1.0,  1.0);

uniform vec2 cameraPos;
uniform vec2 cameraSize;
uniform vec2 sheetSize;

uniform vec3 pos;
uniform vec2 base;
uniform vec2 size;
uniform vec2 anchor;

varying vec2 normalizedTexCoord;
varying vec2 extra;

void main(void) {
    vec2 texCoord = base + posOffset * size;
    normalizedTexCoord = texCoord / sheetSize;

    vec4 worldPos4 = projection * vec4(
            pos.x - anchor.x + abs(size.x) * posOffset.x,
            pos.y,
            pos.z + anchor.y - abs(size.y) * posOffset.y,
            1.0);

    vec4 pos = (worldPos4 - vec4(cameraPos, 0.0, 0.0)) / vec4(cameraSize, 512.0, 1.0);

    gl_Position = scaling * pos;
}
