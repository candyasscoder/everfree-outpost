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
    // Terrain
    //

    shaders.terrain = ctx.start('terrain2.vert', 'terrain2.frag', 2)
        .uniformVec2('cameraPos')
        .uniformVec2('cameraSize')
        .uniformFloat('sliceRadius')
        .uniformFloat('sliceZ')
        .attributes(new Attributes(SIZEOF.TerrainVertex)
                .field(0, gl.UNSIGNED_BYTE, 2, 'corner')
                .field(2, gl.UNSIGNED_BYTE, 3, 'blockPos')
                .field(5, gl.UNSIGNED_BYTE, 1, 'side')
                .field(6, gl.UNSIGNED_BYTE, 2, 'tileCoord'))
        .texture('atlasTex', ctx.makeAssetTexture('tiles'))
        .finish();


    //
    // Light
    //

    var light_base = ctx.start('light2.vert', 'light2.frag', 1)
        .uniformVec2('cameraPos')
        .uniformVec2('cameraSize')
        .texture('depthTex');

    shaders.light_static = light_base.copy()
        .define('LIGHT_INPUT', 'attribute')
        .attributes(new Attributes(SIZEOF.LightVertex)
                .field( 0, gl.UNSIGNED_BYTE,  2, 'corner')
                .field( 2, gl.UNSIGNED_SHORT, 3, 'center')
                .field( 8, gl.UNSIGNED_BYTE,  3, 'colorIn', true)
                .field(12, gl.UNSIGNED_SHORT, 1, 'radiusIn'))
        .finish();

    shaders.light_dynamic = light_base.copy()
        .define('LIGHT_INPUT', 'uniform')
        .uniformVec3('center')
        .uniformVec3('colorIn')
        .uniformFloat('radiusIn')
        .attributes(new Attributes(2, square01_buf)
                .field( 0, gl.UNSIGNED_BYTE,  2, 'corner'))
        .finish();


    //
    // Structure
    //

    var structure_uniforms = new Uniforms()
        .vec2('cameraPos')
        .vec2('cameraSize')
        .float_('sliceRadius')
        .float_('sliceZ');

    var structure_attributes = new Attributes(SIZEOF.StructureBaseVertex)
        .field( 0, gl.UNSIGNED_SHORT, 3, 'vertOffset')
        .field( 8, gl.UNSIGNED_BYTE,  3, 'blockPos')
        .field(11, gl.UNSIGNED_BYTE,  1, 'layer')
        .field(12, gl.UNSIGNED_SHORT, 2, 'displayOffset');

    var structure_textures = new Textures()
        .texture('sheetTex', ctx.makeAssetTexture('structures0'));

    shaders.structure = ctx.start('structure2.vert', 'structure2.frag', 2)
        .uniforms(structure_uniforms)
        .attributes(structure_attributes)
        .textures(structure_textures)
        .finish();

    shaders.structure_shadow = ctx.start('structure2.vert', 'structure2.frag', 1)
        .define('OUTPOST_SHADOW', '1')
        .uniforms(structure_uniforms)
        .attributes(structure_attributes)
        .textures(structure_textures)
        .finish();

    shaders.structure_anim = ctx.start('structure2.vert', 'structure2.frag', 2)
        .define('OUTPOST_ANIM', '1')
        .uniforms(structure_uniforms)
        .uniformFloat('now')
        .attributes(new Attributes(SIZEOF.StructureAnimVertex)
                .field( 0, gl.UNSIGNED_SHORT, 3, 'vertOffset')
                .field( 6, gl.BYTE,           1, 'animLength')
                .field( 7, gl.UNSIGNED_BYTE,  1, 'animRate')
                .field( 8, gl.UNSIGNED_BYTE,  3, 'blockPos')
                .field(11, gl.UNSIGNED_BYTE,  1, 'layer')
                .field(12, gl.SHORT,          2, 'displayOffset')
                .field(16, gl.UNSIGNED_SHORT, 1, 'animOneshotStart')
                .field(18, gl.UNSIGNED_SHORT, 1, 'animStep'))
        .textures(structure_textures)
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
        .texture('shadowTex')
        .texture('shadowDepthTex')
        .finish();
}
exports.makeShaders = makeShaders;
