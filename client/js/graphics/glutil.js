
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
exports.Program = Program;

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

Program.prototype.setUniformNi = function(name, width, vs) {
    var loc = this.getUniformLocation(name);
    if (loc == null) {
        return;
    }
    switch (width) {
        case 1: this.gl.uniform1iv(loc, vs); break;
        case 2: this.gl.uniform2iv(loc, vs); break;
        case 3: this.gl.uniform3iv(loc, vs); break;
        case 4: this.gl.uniform4iv(loc, vs); break;
        default:
            console.assert(false, 'expected width of 1-4, but got', vs.length);
            throw 'bad width for uniform';
    }
};

Program.prototype.setUniformNf = function(name, width, vs) {
    var loc = this.getUniformLocation(name);
    if (loc == null) {
        return;
    }
    switch (width) {
        case 1: this.gl.uniform1fv(loc, vs); break;
        case 2: this.gl.uniform2fv(loc, vs); break;
        case 3: this.gl.uniform3fv(loc, vs); break;
        case 4: this.gl.uniform4fv(loc, vs); break;
        default:
            console.assert(false, 'expected width of 1-4, but got', width);
            throw 'bad width for uniform';
    }
};

Program.prototype.setUniform = function(name, type, vs) {
    switch (type) {
        case 'int': this.setUniformNi(name, 1, vs); break;
        case 'float': this.setUniformNf(name, 1, vs); break;
        case 'vec2': this.setUniformNf(name, 2, vs); break;
        case 'vec3': this.setUniformNf(name, 3, vs); break;
        case 'vec4': this.setUniformNf(name, 4, vs); break;
        default:
            console.assert(false, 'bad uniform type', type);
            throw 'bad uniform type';
    }
};


/** @constructor */
function Texture(gl) {
    this.gl = gl;
    this.texture = gl.createTexture();
    this.width = 0;
    this.height = 0;
}

exports.Texture = Texture;

Texture.prototype.getName = function() {
    return this.texture;
};

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

    this.width = image.width;
    this.height = image.height;
};

Texture.prototype.loadData = function(width, height, data) {
    this.bind();

    var gl = this.gl;
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, width, height, 0, gl.RGBA,
            gl.UNSIGNED_BYTE, data);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);

    this.unbind();

    this.width = width;
    this.height = height;
};


/** @constructor */
function Buffer(gl) {
    this.gl = gl;
    this.buffer = gl.createBuffer();
}
exports.Buffer = Buffer;

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
function GlObject(gl, program, uniforms, attributes, textures) {
    this.gl = gl;
    this.program = program;
    this.base_uniforms = uniforms;
    this.base_attributes = attributes;
    this.base_textures = textures;

    this.program.use();
    for (var key in uniforms) {
        var data = uniforms[key];
        if (data.value != null) {
            this.program.setUniform(key, data.type, data.value);
        }
    }

    var texture_index = 0;
    for (var key in textures) {
        var image = textures[key];

        // Choose a texture slot and set the shader's sampler uniform.
        var index = texture_index;
        this.program.setUniform(key, 'int', [index]);
        this.base_textures[key] = {
            index: index,
            image: image,
        };

        ++texture_index;
    }
}
exports.GlObject = GlObject;

GlObject.prototype.setUniform = function(name, data) {
    this.base_uniforms[name] = data;
    if (data.value != null) {
        this.program.use();
        this.program.setUniform(name, data.type,data.value);
    }
};

GlObject.prototype.setUniformValue = function(name, value) {
    var base = this.base_uniforms[name];
    base.value = value;
    if (value != null) {
        this.program.use();
        this.program.setUniform(name, base.type, base.value);
    }
};

GlObject.prototype.setTexture = function(name, image) {
    this.base_textures[name].image = image;
};

GlObject.prototype.getTexture = function(name) {
    return this.base_textures[name].image;
};

GlObject.prototype.draw = function(vert_base, vert_count, uniforms, attributes, textures) {
    this.drawMulti([[vert_base, vert_count]], uniforms, attributes, textures);
};

GlObject.prototype.drawMulti = function(vert_indexes, uniforms, attributes, textures) {
    var gl = this.gl;

    this.program.use();

    // Set values for uniforms.  The `uniforms` argument has only the new
    // values - the types are taken from the corresponding element of
    // `base_uniforms`.
    for (var key in uniforms) {
        console.assert(this.base_uniforms.hasOwnProperty(key),
                'tried to override undefined uniform', key);
        var base = this.base_uniforms[key];
        var value = uniforms[key];
        this.program.setUniform(key, base.type, value);
    }

    // Enable and bind each vertex attribute.  For these, we *do* need to set
    // the ones in `base_attributes` as well.
    for (var key in this.base_attributes) {
        var base = this.base_attributes[key];
        var buffer = attributes[key] || base.buffer;
        if (buffer == null) {
            continue;
        }

        var attr = this.program.getAttributeLocation(key);
        if (attr == -1) {
            continue;
        }

        gl.enableVertexAttribArray(attr);
        buffer.bind();
        gl.vertexAttribPointer(attr,
                base.count, base.type, base.normalize, base.stride, base.offset);
        buffer.unbind();
    }

    // Bind textures to the appropriate slots.
    for (var key in this.base_textures) {
        var base = this.base_textures[key];
        var image = textures[key] || base.image;
        if (image == null) {
            continue;
        }

        gl.activeTexture(gl.TEXTURE0 + base.index);
        image.bind();
    }

    for (var i = 0; i < vert_indexes.length; ++i) {
        gl.drawArrays(gl.TRIANGLES, vert_indexes[i][0], vert_indexes[i][1]);
    }

    // Unbind all textures.
    for (var key in this.base_textures) {
        var base = this.base_textures[key];
        gl.activeTexture(gl.TEXTURE0 + base.index);
        gl.bindTexture(gl.TEXTURE_2D, null);
    }

    // Disable vertex attributes.
    for (var key in this.base_attributes) {
        var attr = this.program.getAttributeLocation(key);
        if (attr == -1) {
            continue;
        }
        gl.disableVertexAttribArray(attr);
    }

    // Uniforms that have a base value should be restored.  This lets us avoid
    // setting "mostly static" uniforms on every draw call.
    for (var key in uniforms) {
        var base = this.base_uniforms[key];
        if (base.value != null) {
            this.program.setUniform(key, base.type, base.value);
        }
    }
};


/** @constructor */
function Framebuffer(gl, width, height) {
    this.gl = gl;
    this.width = width;
    this.height = height;

    this.texture = new Texture(gl);
    this.texture.loadData(width, height, null);

    var depth_buf = gl.createRenderbuffer();
    gl.bindRenderbuffer(gl.RENDERBUFFER, depth_buf);
    gl.renderbufferStorage(gl.RENDERBUFFER, gl.DEPTH_COMPONENT16, width, height);

    this.fb = gl.createFramebuffer();
    gl.bindFramebuffer(gl.FRAMEBUFFER, this.fb);
    gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D,
            this.texture.getName(), 0);
    gl.framebufferRenderbuffer(gl.FRAMEBUFFER, gl.DEPTH_ATTACHMENT, gl.RENDERBUFFER,
            depth_buf);

    var fb_status = gl.checkFramebufferStatus(gl.FRAMEBUFFER);
    if (fb_status != gl.FRAMEBUFFER_COMPLETE) {
        throw 'framebuffer is not complete: ' + fb_status;
    }

    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    gl.bindRenderbuffer(gl.RENDERBUFFER, null);
    this.texture.unbind();
}
exports.Framebuffer = Framebuffer;

Framebuffer.prototype.getName = function() {
    return this.fb;
};

Framebuffer.prototype.bind = function() {
    this.gl.bindFramebuffer(this.gl.FRAMEBUFFER, this.fb);
};

Framebuffer.prototype.unbind = function() {
    this.gl.bindFramebuffer(this.gl.FRAMEBUFFER, null);
};


exports.uniform = function(type, value) {
    return ({
        type: type,
        value: value,
    });
};

exports.attribute = function(buffer, count, type, normalize, stride, offset) {
    return ({
        buffer: buffer,
        count: count,
        type: type,
        normalize: normalize,
        stride: stride,
        offset: offset,
    });
};
