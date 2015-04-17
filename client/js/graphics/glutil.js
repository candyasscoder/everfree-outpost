var Config = require('config').Config;


function hasExtension(gl, name) {
    if (Config.debug_block_webgl_extensions.get()[name]) {
        return false;
    }
    return gl.getExtension(name) != null;
}
exports.hasExtension = hasExtension;


function compile_shader(gl, type, src) {
    var shader = gl.createShader(type);

    gl.shaderSource(shader, src);
    gl.compileShader(shader);

    if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
        console.log('shader error', gl.getShaderInfoLog(shader));
        console.log(src);
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
    if (!gl.getProgramParameter(this.program, gl.LINK_STATUS)) {
        console.log('program error', gl.getProgramInfoLog(this.program));
        return null;
    }


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


function buildPrograms(gl, vert_src, frag_src, output_buffers) {
    if (hasExtension(gl, 'WEBGL_draw_buffers')) {
        return [new Program(gl, vert_src, frag_src)];
    } else {
        var programs = new Array(output_buffers);
        for (var i = 0; i < output_buffers; ++i) {
            var define = '#define OUTPUT_IDX ' + i + '\n'
            programs[i] = new Program(gl, define + vert_src, define + frag_src);
        }
        return programs;
    }
}
exports.buildPrograms = buildPrograms;


/** @constructor */
function Texture(gl) {
    this.gl = gl;
    this.texture = gl.createTexture();
    this.width = 0;
    this.height = 0;

    this.bind();
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    this.unbind();
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

    this.unbind();

    this.width = image.width;
    this.height = image.height;
};

Texture.prototype.loadData = function(width, height, data) {
    this.bind();

    var gl = this.gl;
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, width, height, 0, gl.RGBA,
            gl.UNSIGNED_BYTE, data);

    this.unbind();

    this.width = width;
    this.height = height;
};

Texture.prototype.initDepth = function(width, height) {
    this.bind();

    var gl = this.gl;
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.DEPTH_COMPONENT, width, height, 0,
            gl.DEPTH_COMPONENT, gl.UNSIGNED_SHORT, null);

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
function GlObject(gl, programs, uniforms, attributes, textures) {
    this.gl = gl;
    this.programs = programs;
    this.base_uniforms = uniforms;
    this.base_attributes = attributes;
    this.base_textures = textures;

    var texture_index = 0;
    for (var key in textures) {
        var image = textures[key];

        // Choose a texture slot.
        var index = texture_index;
        this.base_textures[key] = {
            index: index,
            image: image,
        };

        ++texture_index;
    }

    for (var i = 0; i < this.programs.length; ++i) {
        this.programs[i].use();

        for (var key in uniforms) {
            var data = uniforms[key];
            if (data.value != null) {
                this.programs[i].setUniform(key, data.type, data.value);
            }
        }

        for (var key in textures) {
            // Set the shader's sampler uniform.
            var index = this.base_textures[key].index;
            this.programs[i].setUniform(key, 'int', [index]);
            ++texture_index;
        }
    }
}
exports.GlObject = GlObject;

GlObject.prototype.setUniform = function(name, data) {
    this.base_uniforms[name] = data;
    if (data.value != null) {
        for (var i = 0; i < this.programs.length; ++i) {
            this.programs[i].use();
            this.programs[i].setUniform(name, data.type,data.value);
        }
    }
};

GlObject.prototype.setUniformValue = function(name, value) {
    var base = this.base_uniforms[name];
    base.value = value;
    if (value != null) {
        for (var i = 0; i < this.programs.length; ++i) {
            this.programs[i].use();
            this.programs[i].setUniform(name, base.type, base.value);
        }
    }
};

GlObject.prototype.setTexture = function(name, image) {
    this.base_textures[name].image = image;
};

GlObject.prototype.getTexture = function(name) {
    return this.base_textures[name].image;
};

GlObject.prototype.draw = function(idx, vert_base, vert_count, uniforms, attributes, textures) {
    this.drawMulti(idx, [[vert_base, vert_count]], uniforms, attributes, textures);
};

GlObject.prototype.drawMulti = function(prog_idx, vert_indexes, uniforms, attributes, textures) {
    var gl = this.gl;
    if (prog_idx >= this.programs.length) {
        return;
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

    var program = this.programs[prog_idx];
    program.use();

    // Set values for uniforms.  The `uniforms` argument has only the new
    // values - the types are taken from the corresponding element of
    // `base_uniforms`.
    for (var key in uniforms) {
        console.assert(this.base_uniforms.hasOwnProperty(key),
                'tried to override undefined uniform', key);
        var base = this.base_uniforms[key];
        var value = uniforms[key];
        program.setUniform(key, base.type, value);
    }

    // Enable and bind each vertex attribute.  For these, we *do* need to
    // set the ones in `base_attributes` as well.
    for (var key in this.base_attributes) {
        var base = this.base_attributes[key];
        var buffer = attributes[key] || base.buffer;
        if (buffer == null) {
            continue;
        }

        var attr = program.getAttributeLocation(key);
        if (attr == -1) {
            console.log('no attr', key);
            continue;
        }

        gl.enableVertexAttribArray(attr);
        buffer.bind();
        gl.vertexAttribPointer(attr,
                base.count, base.type, base.normalize, base.stride, base.offset);
        buffer.unbind();
    }

    for (var j = 0; j < vert_indexes.length; ++j) {
        gl.drawArrays(gl.TRIANGLES, vert_indexes[j][0], vert_indexes[j][1]);
    }

    // Disable vertex attributes.
    for (var key in this.base_attributes) {
        var attr = program.getAttributeLocation(key);
        if (attr == -1) {
            continue;
        }
        gl.disableVertexAttribArray(attr);
    }

    // Uniforms that have a base value should be restored.  This lets us
    // avoid setting "mostly static" uniforms on every draw call.
    for (var key in uniforms) {
        var base = this.base_uniforms[key];
        if (base.value != null) {
            program.setUniform(key, base.type, base.value);
        }
    }

    // Unbind all textures.
    for (var key in this.base_textures) {
        var base = this.base_textures[key];
        gl.activeTexture(gl.TEXTURE0 + base.index);
        gl.bindTexture(gl.TEXTURE_2D, null);
    }
};


/** @constructor */
function Framebuffer(gl, width, height, planes) {
    this.gl = gl;
    this.width = width;
    this.height = height;
    planes = planes || 1;

    this.textures = new Array(planes);
    for (var i = 0; i < planes; ++i) {
        this.textures[i] = new Texture(gl);
        this.textures[i].loadData(width, height, null);
    }
    this.texture = this.textures[0];

    this.depth_texture = new Texture(gl);
    this.depth_texture.initDepth(width, height);

    if (hasExtension(gl, 'WEBGL_draw_buffers')) {
        this.fb = gl.createFramebuffer();
        gl.bindFramebuffer(gl.FRAMEBUFFER, this.fb);
        for (var i = 0; i < planes; ++i) {
            gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0 + i, gl.TEXTURE_2D,
                    this.textures[i].getName(), 0);
        }
        gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.DEPTH_ATTACHMENT, gl.TEXTURE_2D,
                this.depth_texture.getName(), 0);

        var fb_status = gl.checkFramebufferStatus(gl.FRAMEBUFFER);
        if (fb_status != gl.FRAMEBUFFER_COMPLETE) {
            throw 'framebuffer is not complete: ' + fb_status;
        }

        var attachments = new Array(planes);
        for (var i = 0; i < planes; ++i) {
            attachments[i] = gl.COLOR_ATTACHMENT0 + i;
        }
        gl.getExtension('WEBGL_draw_buffers').drawBuffersWEBGL(attachments);

        this.fbs = [this.fb];
    } else {
        this.fbs = new Array(planes);
        for (var i = 0; i < planes; ++i) {
            this.fbs[i] = gl.createFramebuffer();
            gl.bindFramebuffer(gl.FRAMEBUFFER, this.fbs[i]);
            gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D,
                    this.textures[i].getName(), 0);
            if (i == 0) {
                gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.DEPTH_ATTACHMENT, gl.TEXTURE_2D,
                        this.depth_texture.getName(), 0);
            } else {
                var depth_buf = gl.createRenderbuffer();
                gl.bindRenderbuffer(gl.RENDERBUFFER, depth_buf);
                gl.renderbufferStorage(gl.RENDERBUFFER, gl.DEPTH_COMPONENT16, width, height);
                gl.framebufferRenderbuffer(gl.FRAMEBUFFER, gl.DEPTH_ATTACHMENT, gl.RENDERBUFFER,
                        depth_buf);
            }

            var fb_status = gl.checkFramebufferStatus(gl.FRAMEBUFFER);
            if (fb_status != gl.FRAMEBUFFER_COMPLETE) {
                throw 'framebuffer is not complete: ' + fb_status;
            }
        }
    }

    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    gl.bindRenderbuffer(gl.RENDERBUFFER, null);
    this.texture.unbind();
}
exports.Framebuffer = Framebuffer;

Framebuffer.prototype.getName = function() {
    return this.fb;
};

Framebuffer.prototype.use = function(callback) {
    var gl = this.gl;
    for (var i = 0; i < this.fbs.length; ++i) {
        gl.bindFramebuffer(gl.FRAMEBUFFER, this.fbs[i]);
        callback(i);
    }
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
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
