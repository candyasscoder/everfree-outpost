precision mediump float;

varying vec2 normalizedTexCoord;
varying vec3 pixelPosition;

uniform sampler2D atlasSampler;

uniform vec2 maskCenter;
uniform float maskRadius2;

void main(void) {
    vec2 off = pixelPosition.xy - maskCenter;
    if (dot(off, off) <= maskRadius2 && pixelPosition.z >= 8.0 - 0.1) {
        discard;
    } else {
        gl_FragColor = texture2D(atlasSampler, normalizedTexCoord);
    }
}
