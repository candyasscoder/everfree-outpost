
const float TILE_SIZE = 32.0;
const float CHUNK_SIZE = 16.0;
const float LOCAL_SIZE = 8.0;
const float ATLAS_SIZE = 32.0;

uniform vec2 cameraPos;
uniform vec2 cameraSize;

attribute vec2 corner;
attribute vec3 blockPos;
attribute float layer;
attribute vec2 displaySize;
attribute vec2 displayOffset;

varying vec2 texCoord;
varying float baseZ;

void main(void) {
    float posX = blockPos.x * TILE_SIZE + corner.x * displaySize.x;
    float posY = blockPos.y * TILE_SIZE;
    float posZ = blockPos.z * TILE_SIZE + (1.0 - corner.y) * displaySize.y;

    // If it's too far left/up from the camera, wrap around.
    if (blockPos.x * TILE_SIZE < cameraPos.x - CHUNK_SIZE * TILE_SIZE) {
        // Remember, posX is measured in *blocks*.
        posX += LOCAL_SIZE * CHUNK_SIZE * TILE_SIZE;
    }
    if (blockPos.y * TILE_SIZE < cameraPos.y - CHUNK_SIZE * TILE_SIZE) {
        posY += LOCAL_SIZE * CHUNK_SIZE * TILE_SIZE;
    }

    vec2 pixelPos = vec2(posX, posY - posZ);
    float depth = posZ + 1.0;

    vec2 normPos = (pixelPos - cameraPos) / cameraSize;
    float normDepth = depth / (CHUNK_SIZE * TILE_SIZE);
    vec3 glPos = vec3(normPos, normDepth) * 2.0 - 1.0;
    glPos.y = -glPos.y;
    gl_Position = vec4(glPos, 1.0);

    texCoord = (displayOffset + displaySize * corner) / (ATLAS_SIZE * TILE_SIZE);
    baseZ = blockPos.z;
}
