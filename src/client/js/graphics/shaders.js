var SIZEOF = require('asmlibs').SIZEOF;
var CHUNK_SIZE = require('data/chunk').CHUNK_SIZE;
var TILE_SIZE = require('data/chunk').TILE_SIZE;

var sb = require('graphics/shaderbuilder');
var Uniforms = sb.Uniforms;
var Attributes = sb.Attributes;
var Textures = sb.Textures;



function makeShaders(shaders, gl, assets, make_texture) {
    var ctx = new sb.ShaderBuilderContext(gl, assets, make_texture);


    var square_buf = ctx.makeBuffer(new Uint8Array([
        -1, -1,
        -1,  1,
         1,  1,

        -1, -1,
         1,  1,
         1, -1,
    ]));

    var square01_buf = ctx.makeBuffer(new Uint8Array([
        0, 0,
        0, 1,
        1, 1,

        0, 0,
        1, 1,
        1, 0,
    ]));


    //
    // Terrain2
    //

    shaders.terrain2 = ctx.start('terrain2.vert', 'terrain2.frag', 2)
        .uniformVec2('cameraPos')
        .uniformVec2('cameraSize')
        .attributes(new Attributes(SIZEOF.Terrain2Vertex)
                .field(0, gl.UNSIGNED_BYTE, 2, 'corner')
                .field(2, gl.UNSIGNED_BYTE, 3, 'blockPos')
                .field(5, gl.UNSIGNED_BYTE, 1, 'side')
                .field(6, gl.UNSIGNED_BYTE, 2, 'tileCoord'))
        .texture('atlasTex', ctx.makeAssetTexture('tiles'))
        .finish();


    //
    // Blits
    //

    var blit_attributes = new Attributes(2, square01_buf)
        .field(0, gl.UNSIGNED_BYTE, 2, 'posOffset');
    var blit_textures = new Textures()
        .texture('image0Tex')
        .texture('image1Tex')
        .texture('depthTex');

    shaders.blit_full = ctx.start('blit_fullscreen.vert', 'blit_output.frag', 1)
        .attributes(blit_attributes)
        .texture('imageTex')
        .finish();

    shaders.post_filter = ctx.start('blit_fullscreen.vert', 'blit_post.frag', 1)
        .uniformVec2('screenSize')
        .attributes(blit_attributes)
        .textures(blit_textures)
        .texture('lightTex')
        .finish();


    //
    // Light2
    //

    var light_base = ctx.start('light2.vert', 'light2.frag', 1)
        .uniformVec2('cameraPos')
        .uniformVec2('cameraSize')
        .texture('depthTex');

    shaders.light2_static = light_base.copy()
        .define('LIGHT_INPUT', 'attribute')
        .attributes(new Attributes(SIZEOF.Light2Vertex)
                .field( 0, gl.UNSIGNED_BYTE,  2, 'corner')
                .field( 2, gl.UNSIGNED_SHORT, 3, 'center')
                .field( 8, gl.UNSIGNED_BYTE,  3, 'colorIn', true)
                .field(12, gl.UNSIGNED_SHORT, 1, 'radiusIn'))
        .finish();

    shaders.light2_dynamic = light_base.copy()
        .define('LIGHT_INPUT', 'uniform')
        .uniformVec3('center')
        .uniformVec3('colorIn')
        .uniformFloat('radiusIn')
        .attributes(new Attributes(2, square01_buf)
                .field( 0, gl.UNSIGNED_BYTE,  2, 'corner'))
        .finish();


    //
    // Structure2
    //

    var struct_uniforms = new Uniforms()

    shaders.structure2 = ctx.start('structure2.vert', 'structure2.frag', 2)
        .uniformVec2('cameraPos')
        .uniformVec2('cameraSize')
        .attributes(new Attributes(SIZEOF.Structure2BaseVertex)
                .field( 0, gl.UNSIGNED_BYTE,  2, 'corner')
                .field( 2, gl.UNSIGNED_BYTE,  3, 'blockPos')
                .field( 5, gl.UNSIGNED_BYTE,  1, 'layer')
                .field( 8, gl.UNSIGNED_SHORT, 2, 'displaySize')
                .field(12, gl.UNSIGNED_SHORT, 2, 'displayOffset'))
        .texture('sheetTex', ctx.makeAssetTexture('structures0'))
        .texture('depthTex', ctx.makeAssetTexture('structdepth0'))
        .finish();

    shaders.structure2_anim = ctx.start('structure2.vert', 'structure2.frag', 2)
        .define('OUTPOST_ANIM', '1')
        .uniformVec2('cameraPos')
        .uniformVec2('cameraSize')
        .uniformFloat('now')
        .attributes(new Attributes(SIZEOF.Structure2AnimVertex)
                .field( 0, gl.UNSIGNED_BYTE,  2, 'corner')
                .field( 2, gl.UNSIGNED_BYTE,  3, 'blockPos')
                .field( 5, gl.UNSIGNED_BYTE,  1, 'layer')
                .field( 8, gl.UNSIGNED_SHORT, 2, 'displaySize')
                .field(12, gl.UNSIGNED_SHORT, 2, 'displayOffset')
                .field(16, gl.UNSIGNED_SHORT, 2, 'animPos')
                .field(20, gl.BYTE,           1, 'animLength')
                .field(21, gl.UNSIGNED_BYTE,  1, 'animRate')
                .field(22, gl.UNSIGNED_SHORT, 1, 'animOneshotStart'))
        .texture('sheetTex', ctx.makeAssetTexture('staticanim0'))
        .texture('depthTex', ctx.makeAssetTexture('staticanimdepth0'))
        .finish();
}
exports.makeShaders = makeShaders;
