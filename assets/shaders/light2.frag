precision mediump float;

uniform sampler2D depthTex;
uniform vec2 cameraSize;

varying float radius;
varying vec3 color;
varying vec2 localPos;
varying vec3 localCenter;

void main(void) {
    vec2 texCoord = localPos / cameraSize;
    texCoord.y = 1.0 - texCoord.y;

    float depth = texture2D(depthTex, texCoord).r;
    float z = depth * 512.0;
    vec3 localPos3 = vec3(localPos.x, localPos.y + z, z);
    vec3 off = localPos3 - localCenter;
    float dist = length(off);

    float ratio = 1.0 - (dist * dist) / (radius * radius);
    gl_FragColor = vec4(color * ratio, ratio);
}
