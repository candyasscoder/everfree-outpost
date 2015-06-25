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
            0.0,        0.0,        1.0 / 16.0, 0.0,
            0.0,        0.0,        0.0,        1.0) *
    // x' = x
    // y' = y - z
    // z' = z
    mat4( 1.0,  0.0,  0.0,  0.0,
          0.0,  1.0,  0.0,  0.0,
          0.0, -1.0,  1.0,  0.0,
          0.0,  0.0,  0.0,  1.0);

uniform vec2 atlasSize;

attribute vec3 position;
attribute vec2 texCoord;
attribute float side;

varying vec2 normalizedTexCoord;

void main(void) {
    // Adjustment to make horizontal and vertical fragments get the same Z.
    // Vertical fragments get Z set to the fragment midpoint, which will be
    // +0.5 from the base Z.
    float axisAdj = side < 2.0 ? -0.5 : 0.0;

    // Adjustment based on side.  This occupies the same bits as the adjustment
    // based on structure layer.
    //
    // Ordering:
    //   -1: Top (of the block below)
    //    0: Bottom (of the current block)
    //    1..12: Structures (by layer)
    //   13: Front (of the current block)
    //   14: Back (of the block in front)
    float sideAdj = 0.0;
    if (side == 0.0) {  // Front
        sideAdj = 13.0;
    } else if (side == 1.0) { // Back
        sideAdj = 14.0;
    } else if (side == 2.0) { // Top
        // -1 == 15 (mod 16).  So we don't use numbers >= 15 in any other
        // cases, to avoid collisions.
        sideAdj = -1.0;
    } else {    // Bottom
        sideAdj = 0.0;
    }

    float adjZ = axisAdj / 512.0 + sideAdj / 16384.0;
    vec4 adj = vec4(0.0, 0.0, adjZ, 0.0);

    gl_Position = projection * vec4(position, 1.0) + adj;
    normalizedTexCoord = texCoord / atlasSize;
}
