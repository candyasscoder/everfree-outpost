precision mediump float;

const mat4 projection =
    // Map from (0,0) .. (1,1) to (-1, -1) .. (1, 1).
    mat4( 2.0,  0.0,  0.0,  0.0,
          0.0,  2.0,  0.0,  0.0,
          0.0,  0.0,  2.0,  0.0,
         -1.0, -1.0, -1.0,  1.0) *
    // Scale based on chunk size (16 * 16).
    mat4(   1.0 / 16.0, 0.0,        0.0,        0.0,
            0.0,        1.0 / 16.0, 0.0,        0.0,
            0.0,        0.0,        1.0 / 32.0, 0.0,
            0.0,        0.0,        0.0,        1.0) *
    // x' = x
    // y' = y - z
    // z' = y + z
    mat4( 1.0,  0.0,  0.0,  0.0,
          0.0,  1.0,  1.0,  0.0,
          0.0, -1.0,  1.0,  0.0,
          0.0,  0.0,  0.0,  1.0);

uniform vec2 atlasSize;

attribute vec3 position;
attribute vec2 texCoord;

varying vec2 normalizedTexCoord;

void main(void) {
    gl_Position = projection * vec4(position, 1.0);
    normalizedTexCoord = texCoord / atlasSize;
}
