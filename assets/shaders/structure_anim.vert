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
uniform float now;

attribute vec3 position;
attribute vec2 texCoord;
attribute float baseZAttr;
attribute float layer;
attribute float animRate;
attribute float animLength;
attribute float animStep;

varying vec2 normalizedTexCoord;
varying float baseZ;

void main(void) {
    // Structures are always rendered vertically, so apply an adjustment to
    // each fragment depth.
    float axisAdj = -0.5;

    // Further adjust Z based on the structure's layer.
    float layerAdj = layer + 1.0;

    float adjZ = axisAdj / 512.0 + layerAdj / 16384.0;
    vec4 adj = vec4(0.0, 0.0, adjZ, 0.0);

    float frame = mod(floor(now * animRate), animLength);
    vec2 frameOffset = vec2(frame * animStep, 0.0);

    gl_Position = projection * vec4(position, 1.0) + adj;
    normalizedTexCoord = (texCoord + frameOffset) / sheetSize;
    baseZ = baseZAttr;
}
