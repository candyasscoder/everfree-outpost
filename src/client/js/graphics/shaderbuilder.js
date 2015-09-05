var GlObject = require('graphics/glutil').GlObject;
var uniform = require('graphics/glutil').uniform;
var attribute = require('graphics/glutil').attribute;
var buildPrograms = require('graphics/glutil').buildPrograms;
var Buffer = require('graphics/glutil').Buffer;


function copy_dict_into(d, result) {
    var ks = Object.getOwnPropertyNames(d);
    for (var i = 0; i < ks.length; ++i) {
        var k = ks[i];
        result[k] = d[k];
    }
}

function copy_dict(d) {
    var result = {};
    copy_dict_into(d, result);
    return result;
}


/** @constructor */
function ShaderBuilderContext(gl, assets, make_texture) {
    this.gl = gl;
    this.assets = assets;
    this.make_texture = make_texture;

    this.common_buffers = {};
    this.common_textures = {};
}
exports.ShaderBuilderContext = ShaderBuilderContext;

ShaderBuilderContext.prototype.start = function(vert_name, frag_name, fb_count) {
    return new ShaderBuilder(this, vert_name, frag_name, fb_count);
};

ShaderBuilderContext.prototype.makeBuffer = function(data) {
    var b = new Buffer(this.gl);
    b.loadData(data);
    return b;
};

ShaderBuilderContext.prototype.makeTexture = function(img) {
    return this.make_texture(img);
};

ShaderBuilderContext.prototype.makeAssetTexture = function(name) {
    return this.make_texture(this.assets[name]);
};


/** @constructor */
function ShaderBuilder(owner, vert_name, frag_name, fb_count) {
    this.p = owner;

    this.vert_name = vert_name;
    this.frag_name = frag_name;
    this.fb_count = fb_count || 1;
    this.shader_defs = {};
    this._uniforms = {};
    this._attributes = {};
    this._textures = {};
}

ShaderBuilder.prototype.copy = function() {
    var other = new ShaderBuilder(this.p, this.vert_name, this.frag_name, this.fb_count);
    other.shader_defs = copy_dict(this.shader_defs);
    other._uniforms = copy_dict(this._uniforms);
    other._attributes = copy_dict(this._attributes);
    other._textures = copy_dict(this._textures);
    return other;
};

ShaderBuilder.prototype.define = function(k, v) {
    this.shader_defs[k] = v;
    return this;
};

ShaderBuilder.prototype.uniforms = function(uniforms) {
    copy_dict_into(uniforms.u, this._uniforms);
    return this;
};

ShaderBuilder.prototype.uniformFloat = function() {
    var u = new Uniforms();
    u.float_.apply(u, arguments);
    this.uniforms(u);
    return this;
};

ShaderBuilder.prototype.uniformVec2 = function() {
    var u = new Uniforms();
    u.vec2.apply(u, arguments);
    this.uniforms(u);
    return this;
};

ShaderBuilder.prototype.uniformVec3 = function() {
    var u = new Uniforms();
    u.vec3.apply(u, arguments);
    this.uniforms(u);
    return this;
};

ShaderBuilder.prototype.uniformVec4 = function() {
    var u = new Uniforms();
    u.vec4.apply(u, arguments);
    this.uniforms(u);
    return this;
};

ShaderBuilder.prototype.attributes = function(attributes) {
    copy_dict_into(attributes.a, this._attributes);
    return this;
};

ShaderBuilder.prototype.textures = function(textures) {
    copy_dict_into(textures.t, this._textures);
    return this;
};

ShaderBuilder.prototype.texture = function() {
    var t = new Textures();
    t.texture.apply(t, arguments);
    this.textures(t);
    return this;
};

ShaderBuilder.prototype.finish = function() {
    var vert = this.p.assets[this.vert_name];
    var frag = this.p.assets[this.frag_name];
    var programs = buildPrograms(this.p.gl, vert, frag, this.fb_count, this.shader_defs);
    return new GlObject(this.p.gl, programs, this._uniforms, this._attributes, this._textures);
};


/** @constructor */
function Uniforms() {
    this.u = {};
}
exports.Uniforms = Uniforms;

Uniforms.prototype.copy = function() {
    var other = new Uniforms();
    other.u = copy_dict(this.u);
    return other;
};

Uniforms.prototype.float_ = function(name, value) {
    if (value === undefined) {
        value = null;
    } else if (value != null && !Array.isArray(value)) {
        value = [value];
    }
    this.u[name] = uniform('float', value);
    return this;
};

Uniforms.prototype.vec2 = function(name, value) {
    this.u[name] = uniform('vec2', value);
    return this;
};

Uniforms.prototype.vec3 = function(name, value) {
    this.u[name] = uniform('vec3', value);
    return this;
};

Uniforms.prototype.vec4 = function(name, value) {
    this.u[name] = uniform('vec4', value);
    return this;
};


/** @constructor */
function Attributes(size, buffer) {
    this.size = size;
    this.buffer = buffer || null;
    this.a = {};
}
exports.Attributes = Attributes;

Attributes.prototype.copy = function() {
    var other = new Attributes(this.size, this.buffer);
    other.a = copy_dict(this.a);
    return other;
};

Attributes.prototype.field = function(offset, type, count, name, scaled) {
    this.a[name] = attribute(this.buffer, count, type, scaled || false, this.size, offset);
    return this;
};


/** @constructor */
function Textures() {
    this.t = {};
};
exports.Textures = Textures;

Textures.prototype.copy = function() {
    var other = new Textures();
    other.t = copy_dict(this.t);
    return other;
};

Textures.prototype.texture = function(name, value) {
    this.t[name] = value || null;
    return this;
};


