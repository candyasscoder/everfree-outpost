precision mediump float;

varying vec2 texCoord;

uniform sampler2D image0Tex;
uniform sampler2D image1Tex;
uniform sampler2D lightTex;
uniform sampler2D depthTex;
uniform vec2 screenSize;


// Highlight if the pixel to the left
//  1) is part of a structure
//  2) is above (higher depth) the current pixel
//  3) has z-offset 0
//  4) is not continuous with the pixel below it
float check_horiz(vec2 off, float centerDepth) {
    vec2 pos = texCoord + off / screenSize;
    float depth = texture2D(depthTex, pos).r;
    // #2
    if (depth < centerDepth + 8.0 / 512.0) {
        return 0.0;
    }

    vec4 color1 = texture2D(image1Tex, pos);
    if (color1.b != 1.0) {
        // #1
        return 0.0;
    }

    float baseZ = color1.r * (255.0 / 8.0 * 32.0);
    float pixelZ = depth * 512.0;

    if (pixelZ - baseZ > 0.75) {
        // #3
        return 0.0;
    }

    float neighborDepth = texture2D(depthTex, pos + vec2(0.0, -1.0) / screenSize).r;
    float neighborDelta = (depth - neighborDepth) * 512.0;
    if (0.5 < neighborDelta && neighborDelta < 1.5) {
        // #4
        return 0.0;
    }

    float delta = depth - (centerDepth + 8.0 / 512.0);
    return clamp(delta * 512.0 / 16.0, 0.0, 1.0);
}

// Highlight if the pixel above
//  1) is part of a structure
//  2) is above (higher depth) the current pixel
//  3) has z-offset 0
float check_vert(vec2 off, float centerDepth) {
    vec2 pos = texCoord + off / screenSize;
    float depth = texture2D(depthTex, pos).r;
    // #2
    if (depth < centerDepth + 8.0 / 512.0) {
        return 0.0;
    }

    vec4 color1 = texture2D(image1Tex, pos);
    if (color1.b != 1.0) {
        // #1
        return 0.0;
    }

    float baseZ = color1.r * (255.0 / 8.0 * 32.0);
    float pixelZ = depth * 512.0;

    if (pixelZ - baseZ > 0.75) {
        // #3
        return 0.0;
    }

    float delta = depth - (centerDepth + 8.0 / 512.0);
    return clamp(delta * 512.0 / 16.0, 0.0, 1.0);
}

float get_highlight() {
    float centerDepth = texture2D(depthTex, texCoord).r;
    float n = check_vert(vec2(0.0, -1.0), centerDepth);
    float s = check_vert(vec2(0.0, 1.0), centerDepth);
    float w = check_horiz(vec2(-1.0, 0.0), centerDepth);
    float e = check_horiz(vec2(1.0, 0.0), centerDepth);

    return max(max(n, s), max(w, e));
}

void main(void) {
    vec4 lightColor = texture2D(lightTex, texCoord);
    vec4 mainColor = texture2D(image0Tex, texCoord) * lightColor;
    vec4 highlightColor = vec4(0.0, 0.75, 1.0, 1.0) * lightColor.a;
    gl_FragColor = mix(mainColor, highlightColor, get_highlight());
    gl_FragColor.a = 1.0;
}
