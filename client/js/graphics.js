var OffscreenContext = require('canvas').OffscreenContext;
var TileDef = require('chunk').TileDef;
var CHUNK_SIZE = require('chunk').CHUNK_SIZE;
var TILE_SIZE = require('chunk').TILE_SIZE;
var LOCAL_SIZE = require('chunk').LOCAL_SIZE;


function compile_shader(gl, type, src) {
    var shader = gl.createShader(type);

    gl.shaderSource(shader, src);
    gl.compileShader(shader);

    if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
        console.log('shader error', gl.getShaderInfoLog(shader));
        return null;
    }

    return shader;
}

/** @constructor */
function Program(gl, vert_src, frag_src) {
    var vert = compile_shader(gl, gl.VERTEX_SHADER, vert_src);
    var frag = compile_shader(gl, gl.FRAGMENT_SHADER, frag_src);

    this.gl = gl;

    this.program = gl.createProgram();
    gl.attachShader(this.program, vert);
    gl.attachShader(this.program, frag);
    gl.linkProgram(this.program);

    this._locations = {};
}

Program.prototype.use = function() {
    this.gl.useProgram(this.program);
};

Program.prototype.getUniformLocation = function(name) {
    if (!(name in this._locations)) {
        this._locations[name] = this.gl.getUniformLocation(this.program, name);
    }
    return this._locations[name];
};

Program.prototype.getAttributeLocation = function(name) {
    if (!(name in this._locations)) {
        this._locations[name] = this.gl.getAttribLocation(this.program, name);
    }
    return this._locations[name];
};

Program.prototype.setUniform1i = function(name, v0) {
    var loc = this.getUniformLocation(name);
    if (loc == null) {
        return;
    }
    this.use();
    this.gl.uniform1i(loc, v0);
};

Program.prototype.setUniform2i = function(name, v0, v1) {
    var loc = this.getUniformLocation(name);
    if (loc == null) {
        return;
    }
    this.use();
    this.gl.uniform2i(loc, v0, v1);
};

Program.prototype.setUniform3i = function(name, v0, v1, v2) {
    var loc = this.getUniformLocation(name);
    if (loc == null) {
        return;
    }
    this.use();
    this.gl.uniform3i(loc, v0, v1, v2);
};

Program.prototype.setUniform4i = function(name, v0, v1, v2, v3) {
    var loc = this.getUniformLocation(name);
    if (loc == null) {
        return;
    }
    this.use();
    this.gl.uniform4i(loc, v0, v1, v2, v3);
};

Program.prototype.setUniform1f = function(name, v0) {
    var loc = this.getUniformLocation(name);
    if (loc == null) {
        return;
    }
    this.use();
    this.gl.uniform1f(loc, v0);
};

Program.prototype.setUniform2f = function(name, v0, v1) {
    var loc = this.getUniformLocation(name);
    if (loc == null) {
        return;
    }
    this.use();
    this.gl.uniform2f(loc, v0, v1);
};

Program.prototype.setUniform3f = function(name, v0, v1, v2) {
    var loc = this.getUniformLocation(name);
    if (loc == null) {
        return;
    }
    this.use();
    this.gl.uniform3f(loc, v0, v1, v2);
};

Program.prototype.setUniform4f = function(name, v0, v1, v2, v3) {
    var loc = this.getUniformLocation(name);
    if (loc == null) {
        return;
    }
    this.use();
    this.gl.uniform4f(loc, v0, v1, v2, v3);
};


/** @constructor */
function Texture(gl) {
    this.gl = gl;
    this.texture = gl.createTexture();
}

Texture.prototype.bind = function() {
    this.gl.bindTexture(this.gl.TEXTURE_2D, this.texture);
};

Texture.prototype.unbind = function() {
    this.gl.bindTexture(this.gl.TEXTURE_2D, null);
};

Texture.prototype.loadImage = function(image) {
    this.bind();

    var gl = this.gl;
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, image);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);

    this.unbind();
};


/** @constructor */
function Buffer(gl) {
    this.gl = gl;
    this.buffer = gl.createBuffer();
}

Buffer.prototype.bind = function() {
    this.gl.bindBuffer(this.gl.ARRAY_BUFFER, this.buffer);
};

Buffer.prototype.unbind = function() {
    this.gl.bindBuffer(this.gl.ARRAY_BUFFER, null);
};

Buffer.prototype.loadData = function(data) {
    var gl = this.gl;
    this.bind();
    gl.bufferData(gl.ARRAY_BUFFER, data, gl.STATIC_DRAW);
    this.unbind();
};


/** @constructor */
function Renderer(gl) {
    this.gl = gl;
    this._asm = new Asm(Asm.getRendererHeapSize());

    this._chunk_buffer = new Array(LOCAL_SIZE * LOCAL_SIZE);
    this._chunk_points = new Array(LOCAL_SIZE * LOCAL_SIZE);
    for (var i = 0; i < LOCAL_SIZE * LOCAL_SIZE; ++i) {
        this._chunk_buffer[i] = new Buffer(gl);
        this._chunk_points[i] = 0;
    }
}
exports.Renderer = Renderer;

Renderer.prototype.initGl = function(assets) {
    var gl = this.gl;

    gl.clearColor(0, 0, 0, 1);
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);

    var terrain_vert = assets['terrain.vert'];
    var terrain_frag = assets['terrain.frag'];
    this.terrain_program = new Program(gl, terrain_vert, terrain_frag);

    var sprite_vert = assets['sprite.vert'];
    var sprite_frag = assets['sprite.frag'];
    this.sprite_program = new Program(gl, sprite_vert, sprite_frag);
    this.sprite_program.setUniform1i('sheetSampler', 0);
    this.sprite_buffer = new Buffer(gl);
    this.sprite_buffer.loadData(new Uint8Array([
            0, 0,
            0, 1,
            1, 1,

            0, 0,
            1, 1,
            1, 0,
    ]));

    var atlas = assets['tiles'];
    this.atlas_texture = new Texture(gl);
    this.atlas_texture.loadImage(atlas);

    this.terrain_program.setUniform2f('atlasSize',
            (atlas.width / TILE_SIZE)|0,
            (atlas.height / TILE_SIZE)|0);
    console.log('atlas size = ',
            (atlas.width / TILE_SIZE)|0,
            (atlas.height / TILE_SIZE)|0);
    this.terrain_program.setUniform1i('atlasSampler', 0);

    this.sprite_texture = new Texture(gl);
};

Renderer.prototype.setSpriteSheet = function(sheet) {
    var img = sheet.image;
    this.sprite_texture.loadImage(img);
    this.sprite_program.setUniform2f('sheetSize', img.width, img.height);
    console.log('loaded sheet', img, img.width, img.height);
};

Renderer.prototype.loadBlockData = function(blocks) {
    var view = this._asm.blockDataView();
    for (var i = 0; i < blocks.length; ++i) {
        var block = blocks[i];
        var base = i * 4;
        view[base + 0] = block.front;
        view[base + 1] = block.back;
        view[base + 2] = block.top;
        view[base + 3] = block.bottom;
    }
};

Renderer.prototype.loadChunk = function(i, j, chunk) {
    var idx = i * LOCAL_SIZE + j;

    this._asm.chunkDataView().set(chunk._tiles);
    i = (idx / LOCAL_SIZE)|0;
    j = (idx % LOCAL_SIZE);
    this._asm.updateXvData(i, j);

    var geom = this._asm.generateGeometry(i, j);
    this._chunk_buffer[idx].loadData(geom);
    this._chunk_points[idx] = (geom.length / 4)|0;
};

Renderer.prototype.render = function(ctx, sx, sy, sw, sh, sprites) {
    var gl = this.gl;

    this.terrain_program.setUniform2f('cameraPos', sx, sy);
    this.terrain_program.setUniform2f('cameraSize', sw, sh);

    this.sprite_program.setUniform2f('cameraPos', sx, sy);
    this.sprite_program.setUniform2f('cameraSize', sw, sh);

    var log = [];

    var this_ = this;

    function draw_terrain(cx, cy, begin, end) {
        log.push(['terrain', cx, cy, begin, end]);
        var posAttr = this_.terrain_program.getAttributeLocation('position');
        var texAttr = this_.terrain_program.getAttributeLocation('texCoord');

        gl.enableVertexAttribArray(posAttr);
        gl.enableVertexAttribArray(texAttr);
        this_.atlas_texture.bind();

        this_.terrain_program.use();
        this_.terrain_program.setUniform2f('chunkPos', cx, cy);

        var i = cy % LOCAL_SIZE;
        var j = cx % LOCAL_SIZE;
        var idx = i * LOCAL_SIZE + j;

        this_._chunk_buffer[idx].bind();
        gl.vertexAttribPointer(posAttr, 2, gl.UNSIGNED_BYTE, false, 4, 0);
        gl.vertexAttribPointer(texAttr, 2, gl.UNSIGNED_BYTE, false, 4, 2);
        gl.drawArrays(gl.TRIANGLES, 0, this_._chunk_points[idx]);
        this_._chunk_buffer[idx].unbind();

        this_.atlas_texture.unbind();
        gl.disableVertexAttribArray(posAttr);
        gl.disableVertexAttribArray(texAttr);
    }

    function draw_sprite(id, x, y) {
        log.push(['sprite', id, x, y]);
        var posAttr = this_.sprite_program.getAttributeLocation('position');

        gl.enableVertexAttribArray(posAttr);
        this_.sprite_texture.bind();

        var sprite = sprites[id];
        var x = sprite.ref_x - sprite.anchor_x;
        var y = sprite.ref_y - sprite.ref_z - sprite.anchor_y;
        this_.sprite_program.use();
        this_.sprite_program.setUniform2f('base', x, y);
        this_.sprite_program.setUniform2f('off', sprite.offset_x, sprite.offset_y);
        this_.sprite_program.setUniform2f('size', sprite.width, sprite.height);
        this_.sprite_program.setUniform2f('flip', sprite.flip, 0);

        this_.sprite_buffer.bind();
        gl.vertexAttribPointer(posAttr, 2, gl.UNSIGNED_BYTE, false, 0, 0);
        gl.drawArrays(gl.TRIANGLES, 0, 6);
        this_.sprite_buffer.unbind();

        this_.sprite_texture.unbind();
        gl.disableVertexAttribArray(posAttr);
    }

    this._asm.render(sx, sy, sw, sh, sprites, draw_terrain, draw_sprite);

    //console.log(log.join(' ; '));
};


/** @constructor */
function Sprite() {
    this.image = null;
    this.offset_x = 0;
    this.offset_y = 0;
    this.width = 0;
    this.height = 0;
    this.flip = false;

    this.ref_x = 0;
    this.ref_y = 0;
    this.ref_z = 0;
    this.anchor_x = 0;
    this.anchor_y = 0;
}
exports.Sprite = Sprite;

Sprite.prototype.refPosition = function() {
    return new Vec(this.ref_x, this.ref_y, this.ref_z);
};

Sprite.prototype.setSource = function(image, offset_x, offset_y, width, height) {
    this.image = image;
    this.offset_x = offset_x;
    this.offset_y = offset_y;
    this.width = width;
    this.height = height;
};

Sprite.prototype.setFlip = function(flip) {
    this.flip = flip;
};

Sprite.prototype.setDestination = function(ref_pos, anchor_x, anchor_y) {
    this.ref_x = ref_pos.x;
    this.ref_y = ref_pos.y;
    this.ref_z = ref_pos.z;
    this.anchor_x = anchor_x;
    this.anchor_y = anchor_y;
};
