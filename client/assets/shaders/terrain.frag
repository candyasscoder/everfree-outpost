varying highp vec2 normalizedTexCoord;

uniform sampler2D atlasSampler;

void main(void) {
    gl_FragColor = texture2D(atlasSampler, normalizedTexCoord);
}
