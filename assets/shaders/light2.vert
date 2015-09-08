precision mediump float;

const float TILE_SIZE = 32.0;
const float CHUNK_SIZE = 16.0;
const float LOCAL_SIZE = 8.0;

uniform vec2 cameraPos;
uniform vec2 cameraSize;

attribute vec2 corner;
LIGHT_INPUT vec3 center;
LIGHT_INPUT float radiusIn;
LIGHT_INPUT vec3 colorIn;

varying float radius;
varying vec3 color;
// The position of the fragment and of the center, relative to the camera
// position.  These are measured in pixels.
varying vec2 localPos;
varying vec3 localCenter;

void main(void) {
    radius = radiusIn;
    color = colorIn;

    // The quad needs to be taller than it is wide by at least sqrt(2).
    // Consider the point X = 0, Y = -Z = radius / sqrt(2).  Its position on
    // the screen is Y - Z = radius * sqrt(2) below the center.
    vec2 cornerOffset = (corner * 2.0 - 1.0) * radius * vec2(1.0, 1.5);

    localCenter = center - vec3(cameraPos, 0.0);

    // If it's too far left/up from the camera, wrap around.
    if (localCenter.x < -CHUNK_SIZE * TILE_SIZE) {
        localCenter.x += LOCAL_SIZE * CHUNK_SIZE * TILE_SIZE;
    }
    if (localCenter.y < -CHUNK_SIZE * TILE_SIZE) {
        localCenter.y += LOCAL_SIZE * CHUNK_SIZE * TILE_SIZE;
    }

    localPos = vec2(localCenter.x, localCenter.y - localCenter.z) + cornerOffset;
    vec2 glPos = localPos / cameraSize * 2.0 - 1.0;
    glPos.y = -glPos.y;
    gl_Position = vec4(glPos, 0.0, 1.0);
}
