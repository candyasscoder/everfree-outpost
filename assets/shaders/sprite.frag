precision mediump float;

varying highp vec2 normalizedTexCoord;

uniform vec2 cameraSize;
uniform float sliceFrac;
uniform sampler2D imageTex;

void main(void) {
    float radius = max(cameraSize.x, cameraSize.y) * sliceFrac * 0.5;
    vec2 off = (gl_FragCoord.xy - 0.5 * cameraSize) / radius;

    if (dot(off, off) <= 1.0) {
        discard;
    }

    vec4 color = texture2D(imageTex, normalizedTexCoord);
    if (color.a == 0.0) {
        discard;
    }
    gl_FragColor = color;
}
