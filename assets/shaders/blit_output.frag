precision mediump float;

varying vec2 texCoord;

uniform sampler2D imageTex;

void main(void) {
    gl_FragColor = texture2D(imageTex, texCoord);
}
