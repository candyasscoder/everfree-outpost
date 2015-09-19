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
    float depth = posZ * TILE_SIZE + 1.0;

    vec2 normPos = (pixelPos - cameraPos) / cameraSize;
    float normDepth = depth / (CHUNK_SIZE * TILE_SIZE);
    vec3 glPos = vec3(normPos, normDepth) * 2.0 - 1.0;
    glPos.y = -glPos.y;
    gl_Position = vec4(glPos, 1.0);

    texCoord = (tileCoord + corner) / ATLAS_SIZE;
    baseZ = blockPos.z;
}
