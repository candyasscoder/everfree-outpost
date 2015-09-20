precision mediump float;

const float TILE_SIZE = 32.0;
const float CHUNK_SIZE = 16.0;
const float LOCAL_SIZE = 8.0;
const float ATLAS_SIZE = 32.0;

uniform vec2 cameraPos;
uniform vec2 cameraSize;

attribute vec2 corner;
attribute vec3 blockPos;
attribute float side;
attribute vec2 tileCoord;

varying vec2 texCoord;
varying float baseZ;

void main(void) {
    float posX = blockPos.x + corner.x;
    float posY = blockPos.y;
    float posZ = blockPos.z + 1.0;

    if (side == 0.0) {          // front
        posY += 1.0;
        posZ -= corner.y;
    } else if (side == 1.0) {   // back
        posZ -= corner.y;
    } else if (side == 2.0) {   // top
        posY += corner.y;
    } else if (side == 3.0) {   // bottom
        posY += corner.y;
        posZ -= 1.0;
    }

    // If it's too far left/up from the camera, wrap around.
    if (blockPos.x * TILE_SIZE < cameraPos.x - CHUNK_SIZE * TILE_SIZE) {
        // Remember, posX is measured in *blocks*.
        posX += LOCAL_SIZE * CHUNK_SIZE;
    }
    if (blockPos.y * TILE_SIZE < cameraPos.y - CHUNK_SIZE * TILE_SIZE) {
        posY += LOCAL_SIZE * CHUNK_SIZE;
    }

    vec2 pixelPos = vec2(posX, posY - posZ) * TILE_SIZE;

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
    float depth = posZ * TILE_SIZE + adjZ;

    vec2 normPos = (pixelPos - cameraPos) / cameraSize;
    float normDepth = depth / (CHUNK_SIZE * TILE_SIZE);
    vec3 glPos = vec3(normPos, normDepth) * 2.0 - 1.0;
    glPos.y = -glPos.y;
    gl_Position = vec4(glPos, 1.0);

    texCoord = (tileCoord + corner) / ATLAS_SIZE;
    baseZ = blockPos.z;
}
