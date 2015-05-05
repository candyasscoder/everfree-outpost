precision mediump float;

attribute vec2 posOffset;
attribute vec3 center;
attribute float radiusAttr;
attribute vec3 colorAttr;

uniform vec2 cameraPos;
uniform vec2 cameraSize;

varying vec3 localCenter;
varying float radius;
varying vec3 color;
varying vec2 localPos;

const mat4 transform = mat4(
        2.0,  0.0,  0.0,  0.0,
        0.0, -2.0,  0.0,  0.0,
        0.0,  0.0,  1.0,  0.0,
       -1.0,  1.0,  0.0,  1.0
       );

void main(void) {
    radius = radiusAttr;
    color = colorAttr;

    localCenter = center - vec3(cameraPos, 0.0);
    localPos = vec2(localCenter.x, localCenter.y - localCenter.z) +
        posOffset * radius * vec2(1.0, 2.0);
    vec2 relPos = localPos / cameraSize;
    gl_Position = transform * vec4(relPos, 0.0, 1.0);
}
