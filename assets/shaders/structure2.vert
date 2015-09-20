
const float TILE_SIZE = 32.0;
const float CHUNK_SIZE = 16.0;
const float LOCAL_SIZE = 8.0;
const float ATLAS_SIZE = 32.0;
const float ANIM_MODULUS_MS = 55440.0;

uniform vec2 cameraPos;
uniform vec2 cameraSize;
#ifdef OUTPOST_ANIM
uniform float now;  // Seconds
#endif

attribute vec2 corner;
attribute vec3 blockPos;
attribute float layer;
attribute vec2 displayOffset;
attribute vec2 displaySize;
#ifdef OUTPOST_ANIM
attribute vec2 animPos;
attribute float animLength;
attribute float animRate;
attribute float animOneshotStart;
#endif

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

#ifdef OUTPOST_ANIM
    posX += animPos.x;
    posZ += animPos.y;
#endif

    vec2 pixelPos = vec2(posX, posY - posZ);

    // Structures are always rendered vertically, so apply an adjustment to
    // each fragment depth.
    float axisAdj = -0.5;
    // Further adjust Z based on the structure's layer.
    float layerAdj = layer + 1.0;
    float adjZ = axisAdj / 512.0 + layerAdj / 16384.0;
    float depth = posZ + adjZ;

    vec2 normPos = (pixelPos - cameraPos) / cameraSize;
    float normDepth = depth / (CHUNK_SIZE * TILE_SIZE);
    vec3 glPos = vec3(normPos, normDepth) * 2.0 - 1.0;
    glPos.y = -glPos.y;
    gl_Position = vec4(glPos, 1.0);

    vec2 texPx = displayOffset + displaySize * corner;
#ifdef OUTPOST_ANIM
    float frame;
    if (animLength >= 0.0) {
        frame = mod(floor(now * animRate), animLength);
    } else {
        // Compute the delta in milliseconds between `now` and
        // `animOneshotStart`, in the range -MODULUS/2 .. MODULUS / 2.
        const float HALF_MOD = ANIM_MODULUS_MS / 2.0;
        float now_ms = mod(now * 1000.0, ANIM_MODULUS_MS);
        float delta = mod(now_ms - animOneshotStart + HALF_MOD, ANIM_MODULUS_MS) - HALF_MOD;
        frame = clamp(floor(delta / 1000.0 * animRate), 0.0, -animLength - 1.0);
    }

    texPx.x += frame * displaySize.x;
#endif
    texCoord = texPx / (ATLAS_SIZE * TILE_SIZE);
    baseZ = blockPos.z;
}
