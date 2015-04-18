var Program = require('graphics/glutil').Program;
var Buffer = require('graphics/glutil').Buffer;

var GlObject = require('graphics/glutil').GlObject;
var uniform = require('graphics/glutil').uniform;
var attribute = require('graphics/glutil').attribute;

var TILE_SIZE = require('data/chunk').TILE_SIZE;


/** @constructor */
function Cursor(gl, assets, radius) {
    this.gl = gl;

    var vert = assets['cursor.vert'];
    var frag = assets['cursor.frag'];
    var program = new Program(gl, vert, frag);

    var buffer = new Buffer(gl);
    buffer.loadData(new Uint8Array([
            0, 0,
            0, 1,
            1, 1,

            0, 0,
            1, 1,
            1, 0,
    ]));

    var uniforms = {
        'cameraPos': uniform('vec2', null),
        'cameraSize': uniform('vec2', null),
        'cursorPos': uniform('vec2', null),
        'cursorRadius': uniform('float', [radius]),
    };
    this._obj = new GlObject(gl, [program],
            uniforms,
            {'position': attribute(buffer, 2, gl.UNSIGNED_BYTE, false, 0, 0)},
            {});
}
exports.Cursor = Cursor;

Cursor.prototype.draw = function(cameraPos, cameraSize, pos) {
    var adjusted_pos = pos.mulScalar(TILE_SIZE).addScalar(TILE_SIZE / 2);

    var uniforms = {
        'cameraPos':    [cameraPos.x,
                         cameraPos.y],
        'cameraSize':   [cameraSize.x,
                         cameraSize.y],
        'cursorPos':    [adjusted_pos.x,
                         adjusted_pos.y],
    };
    this._obj.draw(0, 0, 6, uniforms, {}, {});
}
