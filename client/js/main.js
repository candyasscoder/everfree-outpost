(function() {

var $ = document.getElementById.bind(document);

function fstr1(x) {
    var y = Math.round(x * 10) / 10;
    if (y % 1 == 0) {
        return y + '.0';
    } else {
        return y + '';
    }
}

function gcd(a, b) {
    while (b != 0) {
        var t = b;
        b = a % b;
        a = t;
    }
    return a;
}

function lcm(a, b) {
    return (a * b / gcd(a, b))|0;
}

var INT_MAX = 0x7fffffff;
var INT_MIN = -INT_MAX - 1;


var Vec = require('vec').Vec;
var Deque = require('util').Deque;
var TimeSeries = require('timeseries').TimeSeries;
var AnimCanvas = require('canvas').AnimCanvas;
var OffscreenContext = require('canvas').OffscreenContext;
var Asm = require('asmlibs').Asm;


function DebugMonitor() {
    this.container = document.createElement('table');
    this.container.setAttribute('class', 'debug-monitor');

    this.pos = this._addRow('Pos');
    this.fps = this._addRow('FPS');
    this.load = this._addRow('Load');
    this.jobs = this._addRow('Jobs');
    //this.plan = this._addRow('Plan');
    this.gfxDebug = this._addRow('Gfx');

    this.gfxDebug.innerHTML = '<canvas width="128" height="128" style="border: solid 1px black">';
    this.gfxCanvas = this.gfxDebug.getElementsByTagName('canvas')[0];
    this.gfxCtx = this.gfxCanvas.getContext('2d');

    this._frames = new TimeSeries(5000);
    this._frame_start = 0;
}

DebugMonitor.prototype._addRow = function(label) {
    var row = document.createElement('tr');
    this.container.appendChild(row);

    var left = document.createElement('td');
    row.appendChild(left);
    left.textContent = label;

    var right = document.createElement('td');
    row.appendChild(right);
    return right;
};

DebugMonitor.prototype.frameStart = function() {
    this._frame_start = Date.now();
};

DebugMonitor.prototype.frameEnd = function() {
    var now = Date.now();
    this._frames.record(now, now - this._frame_start);

    var frames = this._frames.count;
    var dur = this._frames.duration() / 1000;
    var fps = frames / dur;
    this.fps.textContent =
        fstr1(fps) + ' fps (' + frames + ' in ' + fstr1(dur) + 's)';

    var work = this._frames.sum;
    var frame_work = work / frames;
    var frame_target = 16.6;
    var load = frame_work / frame_target * 100;
    this.load.textContent =
        fstr1(load) + '% (' + fstr1(frame_work) + ' / ' + fstr1(frame_target) + ')';
};

DebugMonitor.prototype.updateJobs = function(runner) {
    var counts = runner.count();
    var total = counts[0] + counts[1];
    this.jobs.textContent = total + ' (' + counts[0] + ' + ' + counts[1] + ')';
};

DebugMonitor.prototype.updatePlan = function(plan) {
    //this.plan.innerHTML = plan.map(describe_render_step).join('<br>');
};

DebugMonitor.prototype.updatePos = function(pos) {
    this.pos.innerHTML = pos.x + ', ' + pos.y + ', ' + pos.z;
};


function Sheet(image, item_width, item_height) {
    this.image = image;
    this.item_width = item_width;
    this.item_height = item_height;
}

Sheet.prototype.drawInto = function(ctx, i, j, x, y) {
    ctx.drawImage(this.image,
            j * this.item_width,
            i * this.item_height,
            this.item_width,
            this.item_height,
            x,
            y,
            this.item_width,
            this.item_height);
};


function LayeredSheet(images, item_width, item_height) {
    this.images = images;
    this.item_width = item_width;
    this.item_height = item_height;
}

LayeredSheet.prototype.drawInto = function(ctx, i, j, x, y) {
    for (var idx = 0; idx < this.images.length; ++idx) {
        ctx.drawImage(this.images[idx],
                j * this.item_width,
                i * this.item_height,
                this.item_width,
                this.item_height,
                x,
                y,
                this.item_width,
                this.item_height);
    }
};


function AssetLoader() {
    this.assets = {}
    this.pending = 0;
    this.loaded = 0;
}

AssetLoader.prototype.addImage = function(name, url, callback) {
    var img = new Image();

    var this_ = this;
    img.onload = function() {
        if (callback != null) {
            callback(img);
        }
        this_._handleAssetLoad();
    };

    img.src = url;
    this._addPendingAsset(name, img);
};

AssetLoader.prototype.addJson = function(name, url, callback) {
    var xhr = new XMLHttpRequest();
    xhr.open('GET', url, true);

    xhr.responseType = 'json';

    var this_ = this;
    xhr.onreadystatechange = function() {
        if (this.readyState == XMLHttpRequest.DONE) {
            if (callback != null) {
                callback(this.response);
            }
            this_._handleAssetLoad();
        }
    };

    xhr.send();
    this._addPendingAsset(name, xhr);
};

AssetLoader.prototype._addPendingAsset = function(name, asset) {
    if (name != null) {
        this.assets[name] = asset;
    }
    this.pending += 1;
    this._handleProgress();
};

AssetLoader.prototype._handleAssetLoad = function() {
    this.pending -= 1;
    this.loaded += 1;
    this._handleProgress();
    if (this.pending == 0 && typeof this.onload == 'function') {
        this.onload();
    }
};

AssetLoader.prototype._handleProgress = function() {
    if (typeof this.onprogress == 'function') {
        this.onprogress(this.loaded, this.pending + this.loaded);
    }
};


function BackgroundJobRunner() {
    // (jobs_cur, jobs_new) form a standard "queue from two stacks" data
    // structure.  New items are pushed into `jobs_new`; old items are popped
    // from `jobs_cur`.
    this.jobs_cur = [];
    this.jobs_new = [];
    // `subjobs` is a list of subjobs that were created by running the current
    // job.  When the current job finishes, `subjobs` will be reversed and
    // appended to `jobs_cur` (meaning subjobs automatically cut to the front
    // of the queue).
    this.subjobs = [];
    this.current_job_name = null;
    this.subjob_count = 0;
}

BackgroundJobRunner.prototype.job = function(name, cb) {
    var args = Array.prototype.slice.call(arguments, 2);
    this.jobs_new.push({ name: name, cb: cb, args: args });
};

BackgroundJobRunner.prototype.subjob = function(name, cb) {
    console.assert(this.current_job_name != null);
    var args = Array.prototype.slice.call(arguments, 2);
    var full_name = this.current_job_name + '/' + name;
    this.subjobs.push({ name: full_name, cb: cb, args: args });
};

BackgroundJobRunner.prototype.run = function(start, max_dur) {
    var end = start + max_dur;
    var count = 0;
    do {
        var had_job = this.run_one();
        if (had_job) {
            ++count;
        }
    } while (had_job && Date.now() < end);
    if (count > 0) {
        console.log('ran', count, 'jobs in', Date.now() - start);
    }
};

BackgroundJobRunner.prototype.run_one = function() {
    if (this.jobs_cur.length == 0) {
        while (this.jobs_new.length > 0) {
            this.jobs_cur.push(this.jobs_new.pop());
        }
        if (this.jobs_cur.length == 0) {
            return false;
        }
    }

    var job = this.jobs_cur.pop();
    if (this.subjob_count > 0) {
        --this.subjob_count;
    }
    this.current_job_name = job.name;
    try {
        var start = Date.now();
        job.cb.apply(this, job.args);
        var end = Date.now();
    } finally {
        this.current_job_name = null;
        this.subjob_count += this.subjobs.length;
        while (this.subjobs.length > 0) {
            this.jobs_cur.push(this.subjobs.pop());
        }
    }
    return true;
};

BackgroundJobRunner.prototype.count = function() {
    var total = this.jobs_cur.length + this.jobs_new.length;
    return [total - this.subjob_count, this.subjob_count];
};


function Animation(sheet) {
    this.sheet = sheet;
    this._anim = null;
}

Animation.prototype.animate = function(i, j, len, fps, flip, now) {
    if (this._anim != null && i == this._anim.i && j == this._anim.j &&
            len == this._anim.len && fps == this._anim.fps && flip == this._anim.flip) {
        // The new animation is identical to the current one.  Let the
        // current one keep running so that the user doesn't see a skip.
        return;
    }

    this._anim = {
        i: i,
        j: j,
        len: len,
        fps: fps,
        flip: flip,
        start: now,
    };
};

Animation.prototype.drawAt = function(ctx, now, x, y) {
    var anim = this._anim;
    if (anim.flip) {
        ctx.scale(-1, 1);
        x = -x - this.sheet.item_width;
    }
    var frame = Math.floor((now - anim.start) * anim.fps / 1000) % anim.len;
    this.sheet.drawInto(ctx, anim.i, anim.j + frame, x, y);
    if (anim.flip) {
        ctx.scale(-1, 1);
    }
};


function Pony(sheet, x, y, z) {
    this._anim = new Animation(sheet);
    this._anim.animate(0, 2, 1, 1, false, 0);
    this._last_dir = { x: 1, y: 0 };
    this._forecast = new Forecast(new Vec(x - 16, y - 16, z), new Vec(32, 32, 32));
    phys.resetForecast(0, this._forecast, new Vec(0, 0, 0));
}

Pony.prototype.walk = function(now, speed, dx, dy, phys) {
    if (dx != 0 || dy != 0) {
        this._last_dir = { x: dx, y: dy };
    } else {
        dx = this._last_dir.x;
        dy = this._last_dir.y;
        speed = 0;
    }

    var anim = this._anim;
    var flip = dx < 0;
    // Direction, in [0..4].  0 = north, 2 = east, 4 = south.  For western
    // directions, we use [1..3] but also set `flip`.
    var dir = (2 - Math.abs(dx)) * dy + 2;

    if (speed == 0) {
        anim.animate(0, dir, 1, 1, flip, now);
    } else {
        anim.animate(speed, 6 * dir, 6, 6 + 2 * speed, flip, now);
    }

    var pixel_speed = 50 * speed;
    var target_v = new Vec(dx * pixel_speed, dy * pixel_speed, 0);
    phys.resetForecast(now, this._forecast, target_v);
};

Pony.prototype.position = function(now) {
    phys.updateForecast(now, this._forecast);
    var pos = this._forecast.position(now);
    pos.x += 16;
    pos.y += 16;
    return pos;
};

Pony.prototype.getSprite = function(now, base_x, base_y) {
    var pos = this.position(now).sub(new Vec(base_x, base_y, 0));
    var anim = this._anim;

    // Reference point for determining rendering order.
    var pos_x = pos.x;
    var pos_y = pos.y + 16;
    var pos_z = pos.z;

    // Actual point on the screen where the sprite will be rendered.
    var dst_x = pos.x - 48;
    var dst_y = pos.y - pos.z - 74;

    return ({
        draw: function(ctx) {
            anim.drawAt(ctx, now, dst_x, dst_y);
        },
        pos_x: pos_x,
        pos_u: pos_y + pos_z,
        pos_v: pos_y - pos_z,
        dst_x: dst_x,
        dst_y: dst_y,
    });
};


var CHUNK_SIZE = 16;
var TILE_SIZE = 32;
var LOCAL_SIZE = 8;
var LOCAL_TOTAL_SIZE = CHUNK_SIZE * TILE_SIZE * LOCAL_SIZE;

function Chunk() {
    var count = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
    this._tiles = new Uint16Array(count);
}

Chunk.prototype.getId = function(x, y, z) {
    var idx = (z * CHUNK_SIZE + y) * CHUNK_SIZE + x;
    return this._tiles[idx];
};

Chunk.prototype.get = function(x, y, z) {
    return TileDef.by_id[this.getId(x, y, z)];
};

Chunk.prototype.set = function(x, y, z, tile) {
    var tile_id;
    if (typeof tile === 'number') {
        tile_id = tile;
    } else if (typeof tile === 'string') {
        tile_id = TileDef.by_name[tile].id;
    } else if (typeof tile === 'object') {
        tile_id = tile.id;
    } else {
        console.assert(false, "Chunk.set: invalid tile", tile);
    }

    var idx = (z * CHUNK_SIZE + y) * CHUNK_SIZE + x;
    this._tiles[idx] = tile_id;
};


var SHAPE_EMPTY = 0;
var SHAPE_FLOOR = 1;
var SHAPE_SOLID = 2;
var SHAPE_RAMP_E = 3;
var SHAPE_RAMP_W = 4;
var SHAPE_RAMP_S = 5;
var SHAPE_RAMP_N = 6;
var SHAPE_RAMP_TOP = 7;

function TileDef(id, info) {
    this.id = id;
    this.name = info['name'];
    this.shape = info['shape'];

    var front = info['front'];
    if (front != null) {
        this.front = front[1] * 16 + front[0];
    } else {
        this.front = 0;
    }

    var back = info['back'];
    if (back != null) {
        this.back = back[1] * 16 + back[0];
    } else {
        this.back = 0;
    }

    var top = info['top'];
    if (top != null) {
        this.top = top[1] * 16 + top[0];
    } else {
        this.top = 0;
    }

    var bottom = info['bottom'];
    if (bottom != null) {
        this.bottom = bottom[1] * 16 + bottom[0];
    } else {
        this.bottom = 0;
    }
}

window.TileDef = TileDef;

TileDef.by_id = [];
TileDef.by_name = {};

TileDef.register = function(id, info) {
    if (info == null) {
        return;
    }

    var tile = new TileDef(id, info);
    while (TileDef.by_id.length <= tile.id) {
        TileDef.by_id.push(null);
    }
    TileDef.by_id[tile.id] = tile;
    TileDef.by_name[tile.name] = tile;
};


var HAS_TOP     = 0x01;
var HAS_BOTTOM  = 0x02;
var HAS_FRONT   = 0x04;
var HAS_BACK    = 0x08;

function TerrainGraphics(sheet) {
    this.sheet = sheet;

    var chunk_total = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
    this._asm = new Asm(chunk_total * 9);

    var local_total = LOCAL_SIZE * LOCAL_SIZE;
    this._chunks = [];
    for (var i = 0; i < local_total; ++i) {
        this._chunks.push(null);
    }
}

TerrainGraphics.prototype.loadChunk = function(ci, cj, tiles) {
    var view = this._asm.chunkFlagsView();
    console.assert(tiles.length == view.length,
            'expected ' + view.length + ' tiles, but got ' + tiles.length);

    for (var i = 0; i < tiles.length; ++i) {
        var tile = TileDef.by_id[tiles[i]];
        var flags = 0;
        if (tile.front != 0) {
            flags |= HAS_FRONT;
        }
        if (tile.back != 0) {
            flags |= HAS_BACK;
        }
        if (tile.top != 0) {
            flags |= HAS_TOP;
        }
        if (tile.bottom != 0) {
            flags |= HAS_BOTTOM;
        }
        view[i] = flags;
    }

    var result = this._asm.bakeChunk();
    this._chunks[ci * LOCAL_SIZE + cj] = new ChunkGraphics(tiles, result, this.sheet);
};

TerrainGraphics.prototype.unloadChunk = function(ci, cj) {
    this._chunks[ci * LOCAL_SIZE * cj] = null;
};

TerrainGraphics.prototype.render = function(ctx, ci, cj, sprites) {
    var chunk = this._chunks[ci * LOCAL_SIZE + cj];
    if (chunk != null) {
        chunk.render(ctx, sprites);
    }
};


var PAGE_WIDTH = 16;
var PAGE_HEIGHT = 32;

function ChunkGraphics(tiles, bake_result, sheet) {
    this._tiles = tiles;

    this._pages = [];
    for (var i = 0; i < bake_result.pages; ++i) {
        this._pages.push(new OffscreenContext(PAGE_WIDTH * TILE_SIZE, PAGE_HEIGHT * TILE_SIZE));
    }

    this._layers = [];
    for (var i = 0; i < bake_result.layers.length; ++i) {
        var layer = this._initLayer(bake_result.layers[i], sheet);
        this._layers.push(layer);
    }
    this._layers.sort(function(a, b) {
        if (a.pos_u != b.pos_u) {
            return a.pos_u - b.pos_u;
        } else if (a.pos_v != b.pos_v) {
            return a.pos_v - b.pos_v;
        } else {
            return a.pos_x - b.pos_x;
        }
    });
}

ChunkGraphics.prototype._initLayer = function(layer, sheet) {
    var page = this._pages[layer.page];
    var horiz = layer.min.z == layer.max.z;

    var height;
    if (horiz) {
        this._initLayerHoriz(layer, page, sheet);
        height = layer.max.y - layer.min.y;
    } else {
        this._initLayerVert(layer, page, sheet);
        height = layer.max.z - layer.min.z;
    }

    return ({
        page: layer.page,

        src_x: TILE_SIZE * layer.pos_x,
        src_y: TILE_SIZE * layer.pos_y,
        src_w: TILE_SIZE * (layer.max.x - layer.min.x),
        src_h: TILE_SIZE * height,

        // Output position in XUV space, used to determine ordering.
        pos_x: TILE_SIZE * layer.min.x,
        pos_u: TILE_SIZE * (layer.min.y + layer.min.z),
        pos_v: TILE_SIZE * (layer.min.y - layer.min.z),

        // Y-coordinate to use when rendering.
        dst_y: TILE_SIZE * (layer.min.y - layer.min.z - (horiz ? 0 : height)),
    });
};

ChunkGraphics.prototype._initLayerHoriz = function(layer, page, sheet) {
    var z = layer.min.z;

    if (z > 0) {
        for (var y = layer.min.y; y < layer.max.y; ++y) {
            for (var x = layer.min.x; x < layer.max.x; ++x) {
                var idx = ((z - 1) * CHUNK_SIZE + y) * CHUNK_SIZE + x;
                var image = TileDef.by_id[this._tiles[idx]].top;

                var x_out = x - layer.min.x + layer.pos_x;
                var y_out = y - layer.min.y + layer.pos_y;

                sheet.drawInto(page, image >> 4, image & 0xf,
                        x_out * TILE_SIZE, y_out * TILE_SIZE);
            }
        }
    }

    if (z < CHUNK_SIZE) {
        for (var y = layer.min.y; y < layer.max.y; ++y) {
            for (var x = layer.min.x; x < layer.max.x; ++x) {
                var idx = (z * CHUNK_SIZE + y) * CHUNK_SIZE + x;
                var image = TileDef.by_id[this._tiles[idx]].bottom;

                var x_out = x - layer.min.x + layer.pos_x;
                var y_out = y - layer.min.y + layer.pos_y;

                sheet.drawInto(page, image >> 4, image & 0xf,
                        x_out * TILE_SIZE, y_out * TILE_SIZE);
            }
        }
    }
};

ChunkGraphics.prototype._initLayerVert = function(layer, page, sheet) {
    var y = layer.min.y;

    if (y > 0) {
        for (var z = layer.min.z; z < layer.max.z; ++z) {
            for (var x = layer.min.x; x < layer.max.x; ++x) {
                var idx = (z * CHUNK_SIZE + (y - 1)) * CHUNK_SIZE + x;
                var image = TileDef.by_id[this._tiles[idx]].front;

                var x_out = x - layer.min.x + layer.pos_x;
                var y_out = layer.max.z - 1 - z + layer.pos_y;

                sheet.drawInto(page, image >> 4, image & 0xf,
                        x_out * TILE_SIZE, y_out * TILE_SIZE);
            }
        }
    }

    if (y < CHUNK_SIZE) {
        for (var z = layer.min.z; z < layer.max.z; ++z) {
            for (var x = layer.min.x; x < layer.max.x; ++x) {
                var idx = (z * CHUNK_SIZE + y) * CHUNK_SIZE + x;
                var image = TileDef.by_id[this._tiles[idx]].front;

                var x_out = x - layer.min.x + layer.pos_x;
                var y_out = layer.max.z - 1 - z + layer.pos_y;

                sheet.drawInto(page, image >> 4, image & 0xf,
                        x_out * TILE_SIZE, y_out * TILE_SIZE);
            }
        }
    }
};

ChunkGraphics.prototype.render = function(ctx, sprites) {
    var layers = this._layers;

    var i = 0;
    var j = 0;
    while (i < layers.length || j < sprites.length) {
        var which;
        if (i == layers.length) {
            which = 1;
        } else if (j == sprites.length) {
            which = 0;
        } else if (layers[i].pos_u > sprites[j].pos_u) {
            which = 1;
        } else if (layers[i].pos_v > sprites[j].pos_v) {
            which = 1;
        } else {
            which = 0;
        }

        if (which == 0) {
            var layer = layers[i];
            ++i;

            ctx.drawImage(this._pages[layer.page].canvas,
                    layer.src_x, layer.src_y, layer.src_w, layer.src_h,
                    layer.pos_x, layer.dst_y, layer.src_w, layer.src_h);
        } else {
            var sprite = sprites[j];
            ++j;

            sprite.draw(ctx);
        }
    }
};


function Physics() {
    var chunk_total = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
    var local_total = LOCAL_SIZE * LOCAL_SIZE;
    this._asm = new Asm(chunk_total * local_total);
}

Physics.prototype.loadChunk = function(ci, cj, tiles) {
    var view = this._asm.chunkShapeView(ci * LOCAL_SIZE + cj);
    console.assert(tiles.length == view.length,
            'expected ' + view.length + ' tiles, but got ' + tiles.length);

    for (var i = 0; i < tiles.length; ++i) {
        view[i] = TileDef.by_id[tiles[i]].shape;
    }
};

Physics.prototype.resetForecast = function(now, f, v) {
    this._step(now, f);
    f.target_v = v;
    this._forecast(f);
};

Physics.prototype.updateForecast = function(now, f) {
    var i;
    var LIMIT = 5;
    for (i = 0; i < LIMIT && !f.live(now); ++i) {
        var old_end_time = f.end_time;

        var time = Math.min(now, f.end_time);
        this._step(time, f);
        this._forecast(f);

        if (f.end_time == old_end_time) {
            // No progress has been made.
            return;
        }
    }
    //if (i == LIMIT) {
        //console.assert(false, "BUG: updateForecast got stuck but kept incrementing time");
    //}
};

// Step the forecast forward to the given time, and set actual velocity to zero.
Physics.prototype._step = function(time, f) {
    var pos = f.position(time);
    f.start = pos;
    f.end = pos;
    f.actual_v = new Vec(0, 0, 0);
    f.start_time = time;
    f.end_time = INT_MAX * 1000;
};

// Project the time of the next collision starting from start_time, and set
// velocities, end_time, and end position appropriately.
Physics.prototype._forecast = function(f) {
    var result = this._asm.collide(f.start, f.size, f.target_v);
    if (result.t == 0) {
        return;
    }
    f.end = new Vec(result.x, result.y, result.z);
    f.actual_v = f.end.sub(f.start).mulScalar(1000).divScalar(result.t);
    f.end_time = f.start_time + result.t;
};


function Forecast(pos, size) {
    this.start = pos;
    this.end = pos;
    this.size = size;
    this.target_v = new Vec(0, 0, 0);
    this.actual_v = new Vec(0, 0, 0);
    // Timestamps are unix time in milliseconds.  This works because javascript
    // numbers have 53 bits of precision.
    this.start_time = INT_MIN * 1000;
    this.end_time = INT_MAX * 1000;
}

Forecast.prototype.position = function(now) {
    if (now < this.start_time) {
        return this.start.clone();
    } else if (now >= this.end_time) {
        return this.end.clone();
    } else {
        var delta = now - this.start_time;
        var offset = this.actual_v.mulScalar(delta).divScalar(1000);
        return this.start.add(offset);
    }
};

Forecast.prototype.velocity = function() {
    return this.actual_v;
};

Forecast.prototype.target_velocity = function() {
    return this.target_v;
};

Forecast.prototype.live = function(now) {
    return now >= this.start_time && now < this.end_time;
};


window.timeit = function(f) {
    var start = Date.now();
    var i = 0;
    while (Date.now() < start + 3000) {
        f();
        f();
        f();
        f();
        f();
        i += 5;
    }
    var end = Date.now();
    console.log(i + ' iterations in ' + (end - start) + ' ms = ' +
            fstr1((end - start) * 1000 / i) + ' us/iter');
}

window.physBenchmark = function() {
    return phys._asm.collide(new Vec(0, 0, 0), new Vec(32, 32, 32), new Vec(30, 0, 0));
};


var anim_canvas = new AnimCanvas(frame);
window.anim_canvas = anim_canvas;
document.body.appendChild(anim_canvas.canvas);

anim_canvas.ctx.fillStyle = '#f0f';
anim_canvas.ctx.strokeStyle = '#0ff';
//anim_canvas.ctx.lineWidth = 2;
anim_canvas.ctx.imageSmoothingEnabled = false;
anim_canvas.ctx.mozImageSmoothingEnabled = false;

var dbg = new DebugMonitor();
window.dbg = dbg;
document.body.appendChild(dbg.container);


var loader = new AssetLoader();

loader.addImage('pony_f_base', 'assets/sprites/maresprite.png');
loader.addImage('pony_f_eyes_blue', 'assets/sprites/type1blue.png');
loader.addImage('pony_f_horn', 'assets/sprites/marehorn.png');
loader.addImage('pony_f_wing_front', 'assets/sprites/frontwingmare.png');
loader.addImage('pony_f_wing_back', 'assets/sprites/backwingmare.png');
loader.addImage('pony_f_mane_1', 'assets/sprites/maremane1.png');
loader.addImage('pony_f_tail_1', 'assets/sprites/maretail1.png');

loader.addImage('tiles1', 'assets/tiles/mountain_landscape_23.png');

loader.addJson(null, 'tiles.json', function(json) {
    var tiles = json['tiles'];
    console.log('register tiles', tiles.length);
    for (var i = 0; i < tiles.length; ++i) {
        TileDef.register(i, tiles[i]);
    }
});

var assets = loader.assets;
window.assets = assets;

function bake_sprite_sheet(runner) {
    var width = assets['pony_f_base'].width;
    var height = assets['pony_f_base'].height;

    var temp = new OffscreenContext(width, height);
    var baked = new OffscreenContext(width, height);

    function copy(img) {
        baked.drawImage(img, 0, 0);
    }

    function tinted(img, color) {
        this.subjob('copy', function() {
            temp.globalCompositeOperation = 'copy';
            temp.drawImage(img, 0, 0);
        });

        this.subjob('color', function() {
            temp.globalCompositeOperation = 'source-in';
            temp.fillStyle = color;
            temp.fillRect(0, 0, width, height);
        });

        this.subjob('multiply', function() {
            temp.globalCompositeOperation = 'multiply';
            temp.drawImage(img, 0, 0);
        });

        this.subjob('draw', function() {
            baked.drawImage(temp.canvas, 0, 0);
        });
    }

    var coat_color = '#c8f';
    var hair_color = '#84c';
    runner.job('bake', function() {
        runner.subjob('wing_back',  tinted, assets['pony_f_wing_back'], coat_color);
        runner.subjob('base',       tinted, assets['pony_f_base'], coat_color);
        runner.subjob('eyes',       copy, assets['pony_f_eyes_blue']);
        runner.subjob('wing_front', tinted, assets['pony_f_wing_front'], coat_color);
        runner.subjob('tail',       tinted, assets['pony_f_tail_1'], hair_color);
        runner.subjob('mane',       tinted, assets['pony_f_mane_1'], hair_color);
        runner.subjob('horn',       tinted, assets['pony_f_horn'], coat_color);
    });

    return baked.canvas;
}

var tileSheet = new Sheet(assets['tiles1'], 32, 32);
var sheet;
var pony;

var runner = new BackgroundJobRunner();

loader.onload = function() {
    sheet = new Sheet(bake_sprite_sheet(runner), 96, 96);
    pony = new Pony(sheet, 100, 100, 0);
    window.pony = pony;

    document.body.removeChild($('banner-bg'));
    anim_canvas.start();

    initTerrain();
};

loader.onprogress = function(loaded, total) {
    $('banner-text').textContent = 'Loading... (' + loaded + '/' + total + ')';
    $('banner-bar').style.width = Math.floor(loaded / total * 100) + '%';
};

var chunks = [];
for (var i = 0; i < LOCAL_SIZE * LOCAL_SIZE; ++i) {
    var chunk = new Chunk();
    chunks.push(chunk);
}

function initTerrain() {
    var rnd = 0;
    function next() {
        rnd = (Math.imul(rnd, 1103515245) + 12345)|0;
        return rnd & 0x7fffffff;
    }

    for (var i = 0; i < LOCAL_SIZE * LOCAL_SIZE; ++i) {
        var chunk = chunks[i];
        for (var y = 0; y < CHUNK_SIZE; ++y) {
            for (var x = 0; x < CHUNK_SIZE; ++x) {
                var rnd = (x * 7 + y * 13 + i * 31 + 59) >> 2;
                chunk.set(x, y, 0, 'grass/' + (rnd & 3));
            }
        }

        rnd = i;
        for (var y = 0; y < 2; ++y) {
            for (var x = 0; x < 2; ++x) {
                var ox = next() % 3;
                var oy = next() % 4;
                var big = next() % 2;
                for (var j = 0; j < 2; ++j) {
                    for (var k = 0; k < (big ? 4 : 3); ++k) {
                        chunk.set(x * 8 + ox + j, y * 8 + oy, k,
                                'tree/' + (big ? 'medium' : 'small') + '/' + j + k);
                    }
                }
            }
        }

        phys.loadChunk(0, i, chunk._tiles);
        gfx2.loadChunk(0, i, chunk._tiles);
    }
}

var phys = new Physics();
window.phys = phys;
window.physTrace = [];

var gfx2 = new TerrainGraphics(tileSheet);
window.gfx2 = gfx2;

function frame(ctx, now) {
    dbg.frameStart();
    var pos = pony.position(now);

    if (pos.x < LOCAL_TOTAL_SIZE / 2) {
        pony._forecast.start.x += LOCAL_TOTAL_SIZE;
        pony._forecast.end.x += LOCAL_TOTAL_SIZE;
    } else if (pos.x >= LOCAL_TOTAL_SIZE * 3 / 2) {
        pony._forecast.start.x -= LOCAL_TOTAL_SIZE;
        pony._forecast.end.x -= LOCAL_TOTAL_SIZE;
    }

    if (pos.y < LOCAL_TOTAL_SIZE / 2) {
        pony._forecast.start.y += LOCAL_TOTAL_SIZE;
        pony._forecast.end.y += LOCAL_TOTAL_SIZE;
    } else if (pos.y >= LOCAL_TOTAL_SIZE * 3 / 2) {
        pony._forecast.start.y -= LOCAL_TOTAL_SIZE;
        pony._forecast.end.y -= LOCAL_TOTAL_SIZE;
    }

    pos = pony.position(now);
    dbg.updatePos(pos);

    var camera_size = new Vec(ctx.canvas.width|0, ctx.canvas.height|0, 0);
    var camera_pos = pos.sub(camera_size.divScalar(2));

    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.clearRect(0, 0, ctx.canvas.width, ctx.canvas.height);

    ctx.translate(-camera_pos.x, -camera_pos.y);


    var chunk_px = CHUNK_SIZE * TILE_SIZE;
    var chunk_min = camera_pos.divScalar(chunk_px);
    var chunk_max = camera_pos.add(camera_size).addScalar(chunk_px - 1).divScalar(chunk_px);

    for (var raw_cy = chunk_min.y; raw_cy < chunk_max.y; ++raw_cy) {
        for (var raw_cx = chunk_min.x; raw_cx < chunk_max.x; ++raw_cx) {
            var cx = raw_cx % LOCAL_SIZE;
            var cy = raw_cy % LOCAL_SIZE;
            var ci = cy * LOCAL_SIZE + cx;

            var base_x = raw_cx * chunk_px;
            var base_y = raw_cy * chunk_px;
            ctx.save();
            ctx.translate(base_x, base_y);

            var sprites = [];
            if (pos.x + 32 >= base_x && pos.x < base_x + chunk_px &&
                    pos.y + 32 >= base_y && pos.y < base_y + chunk_px) {
                sprites.push(pony.getSprite(now, base_x, base_y));
            }

            gfx2.render(ctx, cy, cx, sprites);

            ctx.restore();
        }
    }



    var coll = window.physTrace;
    ctx.strokeStyle = '#00f';
    for (var i = 0; i < coll.length; ++i) {
        var p = coll[i];
        if (i == coll.length - 1) {
            ctx.strokeStyle = '#a00';
        }
        ctx.strokeRect(p[0], p[1], p[2], p[3]);
        ctx.fillText(i, p[0], p[1] + 10);
    }

    // Draw pony motion forecast
    var fc = pony._forecast;

    if (fc.start.z != 0) {
        ctx.strokeStyle = '#880';
        ctx.beginPath();
        ctx.moveTo(fc.start.x + 16, fc.start.y + 16);
        ctx.lineTo(fc.start.x + 16, fc.start.y + 16 - fc.start.z);
        ctx.stroke();
    }

    if (fc.end.z != 0) {
        ctx.strokeStyle = '#880';
        ctx.beginPath();
        ctx.moveTo(fc.end.x + 16, fc.end.y + 16);
        ctx.lineTo(fc.end.x + 16, fc.end.y + 16 - fc.end.z);
        ctx.stroke();
    }

    ctx.strokeStyle = '#cc0';
    ctx.beginPath();
    ctx.moveTo(fc.start.x + 16, fc.start.y + 16 - fc.start.z);
    ctx.lineTo(fc.end.x + 16, fc.end.y + 16 - fc.end.z);
    ctx.stroke();

    dbg.frameEnd();

    runner.run(now, 10);
    dbg.updateJobs(runner);

    dbg.gfxCtx.drawImage(gfx2._chunks[0]._pages[0].canvas,
            0, 0, 512, 1024,
            0, 0, 64, 128);
}


var dirsHeld = {
    'Up': false,
    'Down': false,
    'Left': false,
    'Right': false,
    'Shift': false,
};

document.addEventListener('keydown', function(evt) {
    var known = true;
    if (dirsHeld.hasOwnProperty(evt.key)) {
        if (!evt.repeat) {
            dirsHeld[evt.key] = true;
            updateWalkDir();
        }
    } else {
        known = false;
    }

    if (known) {
        evt.preventDefault();
        evt.stopPropagation();
    }
});

document.addEventListener('keyup', function(evt) {
    if (dirsHeld.hasOwnProperty(evt.key)) {
        evt.preventDefault();
        evt.stopPropagation();
        dirsHeld[evt.key] = false;
        updateWalkDir();
    }
});

function updateWalkDir() {
    var dx = 0;
    var dy = 0;
    var speed = 1;

    if (dirsHeld['Left']) {
        dx -= 1;
    }
    if (dirsHeld['Right']) {
        dx += 1;
    }

    if (dirsHeld['Up']) {
        dy -= 1;
    }
    if (dirsHeld['Down']) {
        dy += 1;
    }

    if (dirsHeld['Shift']) {
        speed = 3;
    }

    pony.walk(Date.now(), speed, dx, dy, phys);
}

window.physTest = function(a, b, c, d, e, f, g, h, i) {
    return phys._chunk_phys.test(new Vec(a, b, c), new Vec(d, e, f), new Vec(g, h, i));
}

window.gfxAsm = new Asm(0x10000);
window['gfxTest'] = function(a, b, c, d, e, f, g, h, i) {
    return window.gfxAsm.test(new Vec(a, b, c), new Vec(d, e, f), new Vec(g, h, i));
}

window['gfxTest2'] = function() {
    var count = gfxTest().t;
    console.log('got layers', count);
    var ctx = dbg.gfxCtx;
    var colors = ['black', 'red', 'orange', 'yellow', 'green', 'cyan', 'blue', 'purple'];
    for (var i = 0; i < count; ++i) {
        ctx.fillStyle = colors[i % colors.length];
        var view = new Uint8Array(gfxAsm.buffer, 0x3000 + 8 * i, 8);
        var pos = view[7] * 256 + view[6];
        var x = pos % 16;
        var y = (pos / 16)|0;
        if (y > 64) {
            break;
        } else if (y >= 32) {
            y -= 32;
            x += 16;
        }
        var w = view[3] - view[0];
        var h = Math.max(view[4] - view[1], view[5] - view[2]);
        console.log(x, y, w, h);
        ctx.fillRect(x * 4, y * 4, w * 4, h * 4);
    }
}

window.gfxTest3 = function() {
    //dbg.gfxCtx.drawImage(gfx2._chunks[0]._pages[0].canvas,
            //0, 0, 512, 1024,
            //0, 0, 64, 128);
    dbg.gfxCtx.scale(128 / 512, 128 / 512);

    var now = Date.now();
    var s = pony.getSprite(now, 4096, 4096);
    gfx2.render(dbg.gfxCtx, 0, 0, [s]);
}

})();
