precision mediump float;

varying vec2 texCoord;
varying vec2 pos;

uniform vec3 center;
uniform float radius;
uniform vec3 color;
uniform sampler2D depthTex;

void main(void) {
    float depth = texture2D(depthTex, texCoord).r;
    float z = depth * 512.0;
    vec3 pos3 = vec3(pos.x, pos.y - z, z);
    vec3 off = pos3 - center;
    float dist = length(off);

    float ratio = 1.0 - (dist * dist) / (radius * radius);
    gl_FragColor = vec4(color * ratio, ratio);
}
