varying highp vec2 normalizedTexCoord;

uniform sampler2D sheetSampler;

void main(void) {
    gl_FragColor = texture2D(sheetSampler, normalizedTexCoord);
}
