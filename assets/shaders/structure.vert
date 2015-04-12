precision mediump float;

const mat4 projection =
    // Map from (0,0) .. (1,1) to (-1, -1) .. (1, 1).
    mat4( 2.0,  0.0,  0.0,  0.0,
          0.0,  2.0,  0.0,  0.0,
          0.0,  0.0,  2.0,  0.0,
         -1.0, -1.0, -1.0,  1.0) *
    // Scale based on chunk size in pixels (512 * 512).
    mat4(   1.0 / 512.0, 0.0,         0.0,          0.0,
            0.0,         1.0 / 512.0, 0.0,          0.0,
            0.0,         0.0,         1.0 / 512.0,  0.0,
            0.0,         0.0,         0.0,          1.0) *
    // x' = x
    // y' = y - z
    // z' = y + z
    mat4( 1.0,  0.0,  0.0,  0.0,
          0.0,  1.0,  0.0,  0.0,
          0.0, -1.0,  1.0,  0.0,
          0.0,  0.0,  0.0,  1.0);

uniform vec2 sheetSize;

attribute vec3 position;
attribute vec2 texCoord;
attribute float baseZAttr;

varying vec2 normalizedTexCoord;
varying float baseZ;

void main(void) {
    gl_Position = projection * vec4(position, 1.0);
    normalizedTexCoord = texCoord / sheetSize;
    baseZ = baseZAttr;
}
