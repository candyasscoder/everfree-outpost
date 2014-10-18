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


function Vec(x, y, z) {
    this.x = x || 0;
    this.y = y || 0;
    this.z = z || 0;
}
window.Vec = Vec;

Vec.prototype = {
    'clone': function() {
        return new Vec(this.x, this.y, this.z);
    },

    'add': function(other) {
        return new Vec(this.x + other.x, this.y + other.y, this.z + other.z);
    },

    'addScalar': function(c) {
        return new Vec(this.x + c, this.y + c, this.z + c);
    },

    'sub': function(other) {
        return new Vec(this.x - other.x, this.y - other.y, this.z - other.z);
    },

    'subScalar': function(c) {
        return new Vec(this.x - c, this.y - c, this.z - c);
    },

    'mul': function(other) {
        return new Vec((this.x * other.x)|0, (this.y * other.y)|0, (this.z * other.z)|0);
    },

    'mulScalar': function(c) {
        return new Vec((this.x * c)|0, (this.y * c)|0, (this.z * c)|0);
    },

    'div': function(other) {
        return new Vec((this.x / other.x)|0, (this.y / other.y)|0, (this.z / other.z)|0);
    },

    'divScalar': function(c) {
        return new Vec((this.x / c)|0, (this.y / c)|0, (this.z / c)|0);
    },

    'sign': function() {
        return new Vec(Math.sign(this.x), Math.sign(this.y), Math.sign(this.z));
    },

    'isPositive': function() {
        return new Vec(this.x > 0 ? 1 : 0, this.y > 0 ? 1 : 0, this.z > 0 ? 1 : 0);
    },

    'isNegative': function() {
        return new Vec(this.x < 0 ? 1 : 0, this.y < 0 ? 1 : 0, this.z < 0 ? 1 : 0);
    },

    'isZero': function() {
        return new Vec(this.x == 0 ? 1 : 0, this.y == 0 ? 1 : 0, this.z == 0 ? 1 : 0);
    },

    'choose': function(a, b) {
        return new Vec(
                this.x ? a.x : b.x,
                this.y ? a.y : b.y,
                this.z ? a.z : b.z);
    },

    'clamp': function(min, max) {
        return new Vec(
                Math.min(max, Math.max(min, this.x)),
                Math.min(max, Math.max(min, this.y)),
                Math.min(max, Math.max(min, this.z)));
    },

    'map': function(f) {
        return new Vec(f(this.x), f(this.y), f(this.z));
    },

    'forEach': function(f) {
        f(this.x);
        f(this.y);
        f(this.z);
    },

    'zip': function(a, f) {
        return new Vec(
                f(this.x, a.x),
                f(this.y, a.y),
                f(this.z, a.z));
    },

    'zip3': function(a, b, f) {
        return new Vec(
                f(this.x, a.x, b.x),
                f(this.y, a.y, b.y),
                f(this.z, a.z, b.z));
    },

    'zip4': function(a, b, c, f) {
        return new Vec(
                f(this.x, a.x, b.x, c.x),
                f(this.y, a.y, b.y, c.y),
                f(this.z, a.z, b.z, c.z));
    },

    'get': function(i) {
        if (i == 0) {
            return this.x;
        } else if (i == 1) {
            return this.y;
        } else if (i == 2) {
            return this.z;
        } else {
            throw 'Vec.get: bad index';
        }
    },

    'toString': function() {
        return [this.x, this.y, this.z].join(',');
    },
};


function Deque() {
    this._cur = [];
    this._new = [];
}

Deque.prototype = {
    'enqueue': function(x) {
        this._new.push(x);
    },

    'dequeue': function() {
        this._prepareRead();
        return this._cur.pop();
    },

    '_prepareRead': function() {
        if (this._cur.length == 0) {
            while (this._new.length > 0) {
                this._cur.push(this._new.pop());
            }
        }
    },

    'peek': function() {
        this._prepareRead();
        if (this._cur.length == 0) {
            return null;
        } else {
            return this._cur[this._cur.length - 1];
        }
    },

    'peek_back': function() {
        if (this._new.length > 0) {
            return this._new[this._new.length - 1];
        } else if (this._cur.length > 0) {
            return this._cur[0];
        } else {
            return null;
        }
    },
}


function TimeSeries(dur) {
    this._q = new Deque();
    this._dur = dur;
    this.sum = 0;
    this.count = 0;
    this._last_popped_time = Date.now();
}

TimeSeries.prototype = {
    'record': function(now, value) {
        var start = now - this._dur;
        while (true) {
            var item = this._q.peek();
            if (item == null) {
                break;
            }
            if (item[0] >= start) {
                break;
            }

            this._q.dequeue();
            this.sum -= item[1];
            --this.count;
            this._last_popped_time = item[0];
        }

        this._q.enqueue([now, value]);
        this.sum += value;
        ++this.count;
    },

    'duration': function() {
        return this._q.peek_back()[0] - this._last_popped_time;
    },
};


function AnimCanvas(frame_callback) {
    this.canvas = document.createElement('canvas');
    this.ctx = this.canvas.getContext('2d');
    this._handleResize();
    this.animating = false;

    var this_ = this;

    window.addEventListener('resize', function() {
        this_._handleResize();
    });

    function frameWrapper() {
        frame_callback(this_.ctx, Date.now());
        if (this_.animating) {
            window.requestAnimationFrame(frameWrapper);
        }
    }
    // Save frameWrapper for calls to `start()`.
    this._cb = frameWrapper;
}

AnimCanvas.prototype = {
    'start': function() {
        this.animating = true;
        window.requestAnimationFrame(this._cb);
    },

    'stop': function() {
        this.animating = false;
    },

    '_handleResize': function() {
        var width = window.innerWidth;
        var height = window.innerHeight;

        this.scale = calcScale(Math.max(width, height));

        var scale;
        var invScale;
        if (this.scale < 0) {
            invScale = -this.scale;
            scale = 1.0 / invScale;
        } else {
            scale = this.scale;
            invScale = 1.0 / scale;
        }

        var virtWidth = Math.ceil(width * invScale);
        var virtHeight = Math.ceil(height * invScale);

        var physWidth = Math.round(virtWidth * scale);
        var physHeight = Math.round(virtHeight * scale);

        this.canvas.width = virtWidth;
        this.canvas.height = virtHeight;
        this.canvas.style.width = physWidth + 'px';
        this.canvas.style.height = physHeight + 'px';

        console.log('resize', width, height, this.scale, virtWidth, virtHeight, physWidth, physHeight);
    },
}

function calcScale(px) {
    var target = 1024;
    if (px < target) {
        return -Math.round(target / px);
    } else {
        return Math.round(px / target);
    }
}


function DebugMonitor() {
    this.container = document.createElement('table');
    this.container.setAttribute('class', 'debug-monitor');

    this.fps = this._addRow('FPS');
    this.load = this._addRow('Load');
    this.jobs = this._addRow('Jobs');
    this.plan = this._addRow('Plan');

    this._frames = new TimeSeries(5000);
    this._frame_start = 0;
}

DebugMonitor.prototype = {
    '_addRow': function(label) {
        var row = document.createElement('tr');
        this.container.appendChild(row);

        var left = document.createElement('td');
        row.appendChild(left);
        left.textContent = label;

        var right = document.createElement('td');
        row.appendChild(right);
        return right;
    },

    'frameStart': function() {
        this._frame_start = Date.now();
    },

    'frameEnd': function() {
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
    },

    'updateJobs': function(runner) {
        var counts = runner.count();
        var total = counts[0] + counts[1];
        this.jobs.textContent = total + ' (' + counts[0] + ' + ' + counts[1] + ')';
    },

    'updatePlan': function(plan) {
        this.plan.innerHTML = plan.map(describe_render_step).join('<br>');
    },
};


function OffscreenContext(width, height) {
    var canvas = document.createElement('canvas');
    canvas.width = width;
    canvas.height = height;
    return canvas.getContext('2d');
}


function Sheet(image, item_width, item_height) {
    this.image = image;
    this.item_width = item_width;
    this.item_height = item_height;
}

Sheet.prototype = {
    'drawInto': function(ctx, i, j, x, y) {
        ctx.drawImage(this.image,
                j * this.item_width,
                i * this.item_height,
                this.item_width,
                this.item_height,
                x,
                y,
                this.item_width,
                this.item_height);
    },
};


function LayeredSheet(images, item_width, item_height) {
    this.images = images;
    this.item_width = item_width;
    this.item_height = item_height;
}

LayeredSheet.prototype = {
    'drawInto': function(ctx, i, j, x, y) {
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
    },
};


function AssetLoader() {
    this.assets = {}
    this.pending = 0;
    this.loaded = 0;
}

AssetLoader.prototype = {
    'addImage': function(name, url) {
        var img = new Image();

        var this_ = this;
        img.onload = function() { this_._handleAssetLoad(); };

        img.src = url;
        this._addPendingAsset(name, img);
    },

    '_addPendingAsset': function(name, asset) {
        this.assets[name] = asset;
        this.pending += 1;
        this._handleProgress();
    },

    '_handleAssetLoad': function() {
        this.pending -= 1;
        this.loaded += 1;
        this._handleProgress();
        if (this.pending == 0 && typeof this.onload == 'function') {
            this.onload();
        }
    },

    '_handleProgress': function() {
        if (typeof this.onprogress == 'function') {
            this.onprogress(this.loaded, this.pending + this.loaded);
        }
    },
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

BackgroundJobRunner.prototype = {
    'job': function(name, cb) {
        var args = Array.prototype.slice.call(arguments, 2);
        this.jobs_new.push({ 'name': name, 'cb': cb, 'args': args });
    },

    'subjob': function(name, cb) {
        console.assert(this.current_job_name != null);
        var args = Array.prototype.slice.call(arguments, 2);
        var full_name = this.current_job_name + '/' + name;
        this.subjobs.push({ 'name': full_name, 'cb': cb, 'args': args });
    },

    'run': function(start, max_dur) {
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
    },

    'run_one': function() {
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
    },

    'count': function() {
        var total = this.jobs_cur.length + this.jobs_new.length;
        return [total - this.subjob_count, this.subjob_count];
    },
};


function Entity(sheet, ax, ay, x, y) {
    this.sheet = sheet;
    this._motion = {
        'last_x': x,
        'last_y': y,
        'velocity_x': 0,
        'velocity_y': 0,
        'start': 0,
        'dur': 0,
    };
    this._anim = null;
    this.anchor = { 'x': ax, 'y': ay };
}

Entity.prototype = {
    'animate': function(i, j, len, fps, flip, now) {
        this._anim = {
            'i': i,
            'j': j,
            'len': len,
            'fps': fps,
            'flip': flip,
            'start': now,
        };
    },

    'move': function(vx, vy, now, end) {
        var pos = this.position(now);
        this._motion = {
            'last_x': pos.x,
            'last_y': pos.y,
            'velocity_x': vx,
            'velocity_y': vy,
            'start': now,
            'dur': end - now,
        };
    },

    'position': function(now) {
        var motion = this._motion;
        var delta = now - motion.start;
        delta = Math.min(delta, motion.dur);
        var x = motion.last_x + Math.floor(delta * motion.velocity_x / 1000);
        var y = motion.last_y + Math.floor(delta * motion.velocity_y / 1000);
        return { 'x': x, 'y': y }
    },

    'drawInto': function(ctx, now) {
        var pos = this.position(now);
        this.drawAt(ctx, now, pos.x, pos.y);
    },

    'drawAt': function(ctx, now, pos_x, pos_y) {
        var x = pos_x - this.anchor.x;
        var y = pos_y - this.anchor.y;

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
    },
};


function Pony(sheet, x, y, z) {
    this._entity = new Entity(sheet, 48, 74, x, y);
    this._entity.animate(0, 2, 1, 1, false, 0);
    this._last_dir = { 'x': 1, 'y': 0 };
    this._forecast = new Forecast(new Vec(x - 16, y - 16, z), new Vec(32, 32, 32));
    phys.resetForecast(0, this._forecast, new Vec(0, 0, 0));
}

Pony.prototype = {
    'walk': function(now, speed, dx, dy, phys) {
        if (dx != 0 || dy != 0) {
            this._last_dir = { 'x': dx, 'y': dy };
        } else {
            dx = this._last_dir.x;
            dy = this._last_dir.y;
            speed = 0;
        }

        var entity = this._entity;
        var flip = dx < 0;
        // Direction, in [0..4].  0 = north, 2 = east, 4 = south.  For western
        // directions, we use [1..3] but also set `flip`.
        var dir = (2 - Math.abs(dx)) * dy + 2;

        if (speed == 0) {
            entity.animate(0, dir, 1, 1, flip, now);
        } else {
            entity.animate(speed, 6 * dir, 6, 6 + 2 * speed, flip, now);
        }

        var pixel_speed = 30 * speed;
        var pos = entity.position(now);
        var target_v = new Vec(dx * pixel_speed, dy * pixel_speed, 0);
        phys.resetForecast(now, this._forecast, target_v);
    },

    'position': function(now) {
        phys.updateForecast(now, this._forecast);
        var pos = this._forecast.position(now);
        pos.x += 16;
        pos.y += 16;
        return pos;
    },

    'drawInto': function(ctx, now) {
        var pos = this.position(now);

        ctx.strokeStyle = '#0aa';
        ctx.strokeRect(pos.x - 16, pos.y - 16, this._forecast.size.x, this._forecast.size.y);
        ctx.beginPath();
        ctx.moveTo(pos.x, pos.y);
        ctx.lineTo(pos.x, pos.y - pos.z);
        ctx.stroke();

        ctx.strokeStyle = '#0ff';
        ctx.strokeRect(pos.x - 16, pos.y - 16 - pos.z, this._forecast.size.x, this._forecast.size.y);

        this._entity.drawAt(ctx, now, pos.x, pos.y - pos.z);
    },
};


var CHUNK_SIZE = 16;
var TILE_SIZE = 32;

var SHAPE_EMPTY = 0;
var SHAPE_FLOOR = 1;
var SHAPE_SOLID = 2;
var SHAPE_RAMP_E = 3;

function Chunk() {
    var count = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
    this._storage = new ArrayBuffer(count * 3);
    this._bottom = new Uint8Array(this._storage, count * 0, count);
    this._front = new Uint8Array(this._storage, count * 1, count);
    this._shape = new Uint8Array(this._storage, count * 2, count);
}

(function() {
    function index(x, y, z) {
        return ((z) * CHUNK_SIZE + y) * CHUNK_SIZE + x;
    }

    Chunk.prototype = {
        'bottom': function(x, y, z) {
            return this._bottom[index(x,y,z)];
        },

        'setBottom': function(x, y, z, bottom) {
            this._bottom[index(x,y,z)] = bottom;
        },

        'front': function(x, y, z) {
            return this._front[index(x,y,z)];
        },

        'setFront': function(x, y, z, front) {
            this._front[index(x,y,z)] = front;
        },

        'shape': function(x, y, z) {
            return this._shape[index(x,y,z)];
        },

        'setShape': function(x, y, z, shape) {
            this._shape[index(x, y, z)] = shape;
        },

        'set': function(x, y, z, bottom, front, shape) {
            this.setBottom(x, y, z, bottom);
            this.setFront(x, y, z, front);
            this.setShape(x, y, z, shape);
        },
    };
})();


function ChunkRendering(chunk, sheet) {
    this.chunk = chunk;
    this.sheet = sheet;
    this._bakedBottom = [];
    this._bakedFront = [];
    for (var i = 0; i < CHUNK_SIZE; ++i) {
        this._bakedBottom.push(null);
        this._bakedFront.push(null);
    }
}

ChunkRendering.prototype = {
    'bake': function() {
        for (var z = 0; z < CHUNK_SIZE; ++z) {
            var baked = this._prepareBaked(z, 0);
            this._bakeLayer(z, 0, baked);
            this._bakedBottom[z] = baked;

            var baked = this._prepareBaked(z, 1);
            this._bakeLayer(z, 1, baked);
            this._bakedFront[z] = baked;
        }
    },

    '_layerCell': function(x, y, z, l) {
        if (l == 0) {
            return this.chunk.bottom(x, y, z);
        } else {
            return this.chunk.front(x, y, z);
        }
    },

    '_prepareBaked': function(z, l) {
        var min_x = CHUNK_SIZE;
        var max_x = 0;
        var min_y = CHUNK_SIZE;
        var max_y = 0;
        for (var y = 0; y < CHUNK_SIZE; ++y) {
            for (var x = 0; x < CHUNK_SIZE; ++x) {
                if (this._layerCell(x, y, z, l) == 0) {
                    continue;
                }
                min_x = Math.min(x, min_x);
                max_x = Math.max(x + 1, max_x);
                min_y = Math.min(y, min_y);
                max_y = Math.max(y + 1, max_y);
            }
        }

        var size_x = Math.max(0, max_x - min_x);
        var size_y = Math.max(0, max_y - min_y);

        if (size_x == 0 || size_y == 0) {
            return null;
        } else {
            return ({
                'x': min_x,
                'y': min_y,
                'w': size_x,
                'h': size_y,
                'ctx': new OffscreenContext(size_x * TILE_SIZE, size_y * TILE_SIZE),
            });
        }
    },

    '_bakeLayer': function(z, l, baked) {
        if (baked == null) {
            return;
        }
        var base_x = baked.x;
        var base_y = baked.y;
        for (var y = 0; y < baked.h; ++y) {
            for (var x = 0; x < baked.w; ++x) {
                var tile = this._layerCell(x + base_x, y + base_y, z, l);
                if (tile != 0) {
                    this.sheet.drawInto(baked.ctx, tile >> 4, tile & 15,
                            x * TILE_SIZE, y * TILE_SIZE);
                    //baked.ctx.strokeRect(x * TILE_SIZE, y * TILE_SIZE, TILE_SIZE, TILE_SIZE);
                }
            }
        }
    },

    'drawBottom': function(ctx, z, min_y, max_y, dx, dy) {
        this._drawLayer(ctx, this._bakedBottom, z, min_y, max_y, dx, dy);
    },

    'drawFront': function(ctx, z, min_y, max_y, dx, dy) {
        this._drawLayer(ctx, this._bakedFront, z, min_y, max_y, dx, dy);
    },

    'draw': function(ctx, z, min_y, max_y, dx, dy) {
        // TODO: these calls can probably be merged together
        this.drawBottom(ctx, z, min_y, max_y, dx, dy);
        this.drawFront(ctx, z, min_y, max_y, dx, dy);
    },

    '_drawLayer': function(ctx, bakedArr, z, min_y, max_y, dx, dy) {
        var baked = bakedArr[z];
        if (baked == null) {
            return;
        }

        var real_min_y = Math.max(min_y, baked.y);
        var real_max_y = Math.min(max_y, baked.y + baked.h);

        // Requested rows do not overlap the baked layer.
        if (real_max_y <= real_min_y) {
            return;
        }

        var src_offset_y = (real_min_y - baked.y) * TILE_SIZE;
        var dest_offset_x = baked.x * TILE_SIZE;
        var dest_offset_y = real_min_y * TILE_SIZE;
        var px_width = baked.w * TILE_SIZE;
        var px_height = (real_max_y - real_min_y) * TILE_SIZE;

        ctx.drawImage(baked.ctx.canvas,
                0, src_offset_y, px_width, px_height,
                dx + dest_offset_x, dy + dest_offset_y, px_width, px_height);
    },
};


var PLAN_FULL_LAYERS = 1;
var PLAN_PARTIAL_LAYERS = 2;
var PLAN_FULL_LINES = 3;
var PLAN_PARTIAL_LINE = 4;
var PLAN_SPRITE = 5;

function RenderPlanner() {
    this.y_occupy = new Array(CHUNK_SIZE);
    this.z_occupy = new Array(CHUNK_SIZE);
    this.plan_ = [];
    this.plan_len = 0;
    this.sprites = null;
    this.y_sprites = [];
    this.y_sprites_len = 0;
}

RenderPlanner.prototype = {
    '_init': function(sprites) {
        this.sprites = sprites;
        this.sprites.sort(function(a, b) {
            return a.z - b.z;
        });

        for (var i = 0; i < CHUNK_SIZE; ++i) {
            this.z_occupy[i] = 0;
        }

        this.plan_len = 0;
        for (var i = 0; i < this.plan_.length; ++i) {
            this.plan_[i] = null;
        }
    },

    '_cleanup': function() {
        this.sprites = null;
        this._clearYSprites();
    },

    '_clearYSprites': function() {
        for (var i = 0; i < this.y_sprites.length; ++i) {
            this.y_sprites[i] = null;
        }
        this.y_sprites_len = 0;
    },

    '_recordYSprite': function(idx, sprite) {
        if (this.y_sprites_len == this.y_sprites.length) {
            this.y_sprites.push(sprite);
        } else {
            this.y_sprites[this.y_sprites_len] = sprite;
        }
        ++this.y_sprites_len;
    },

    '_sortYSprites': function() {
        this.y_sprites.sort(function(a, b) {
            if (a == null && b == null) {
                return 0;
            } else if (a == null) {
                return -1;
            } else if (b == null) {
                return 1;
            } else {
                if (a.y != b.y) {
                    return a.y - b.y;
                } else {
                    return a.z - b.z;
                }
            }
        });
    },

    '_plan': function() {
        var sprites = this.sprites;
        var z_occupy = this.z_occupy;

        for (var i = 0; i < sprites.length; ++i) {
            var sprite = sprites[i];
            var min_z = sprite.z;
            var max_z = min_z + sprite.size_z;
            for (var z = min_z; z < max_z; ++z) {
                ++z_occupy[z];
            }
        }

        var start = 0;
        var cur_mode = z_occupy[0] != 0;

        for (var i = 1; i < CHUNK_SIZE; ++i) {
            var mode = z_occupy[i] != 0;
            if (mode != cur_mode) {
                this._planLayers(start, i, cur_mode);
                cur_mode = mode;
                start = i;
            }
        }

        this._planLayers(start, CHUNK_SIZE, cur_mode);
    },

    '_planLayers': function(min_z, max_z, has_sprites) {
        if (!has_sprites) {
            this._planOne3(PLAN_FULL_LAYERS, min_z, max_z);
            return;
        }

        var sprites = this.sprites;
        var y_occupy = this.y_occupy;

        for (var i = 0; i < CHUNK_SIZE; ++i) {
            this.y_occupy[i] = 0;
        }

        this._clearYSprites();
        // TODO: use binary search to find start
        for (var i = 0; i < sprites.length; ++i) {
            var sprite = sprites[i];
            if (sprite.z < min_z) {
                continue;
            } else if (sprite.z >= max_z) {
                break;
            }
            this._recordYSprite(i, sprite);
            ++y_occupy[sprite.y];
        }
        this._sortYSprites();

        var start = 0;

        for (var i = 0; i < CHUNK_SIZE; ++i) {
            if (y_occupy[i] != 0) {
                if (start != i) {
                    this._planPartialLayers(start, i, min_z, max_z);
                }
                this._planLinesWithSprites(i, min_z, max_z);
                start = i + 1;
            }
        }

        if (start != CHUNK_SIZE) {
            this._planPartialLayers(start, CHUNK_SIZE, min_z, max_z);
        }
    },

    '_planPartialLayers': function(min_y, max_y, min_z, max_z) {
        this._planOne5(PLAN_PARTIAL_LAYERS, min_z, max_z, min_y, max_y);
    },

    '_planLinesWithSprites': function(y, min_z, max_z) {
        if (y == CHUNK_SIZE) {
            // This happens when the last y is occupied.
            return;
        }

        // TODO: use binary search to find start
        var start_i = 0;
        var end_i = this.y_sprites_len;
        for (var i = 0; i < this.y_sprites_len; ++i) {
            var sprite = this.y_sprites[i];
            if (sprite.y < y) {
                start_i = i + 1;
                continue;
            } else if (sprite.y > y) {
                end_i = i;
                break;
            }
        }

        // When open_z != -1, that means the line (*, y, open_z) has had the
        // bottom rendered, but not the front.
        var open_z = min_z - 1;

        if (start_i < this.y_sprites_len && this.y_sprites[start_i].z == 0) {
            open_z = min_z;
            this._planOne4(PLAN_PARTIAL_LINE, 0, y, 0);
        }

        for (var i = start_i; i < end_i; ++i) {
            var sprite = this.y_sprites[i];
            var z = sprite.z;
            if (z != open_z) {
                // Close open_z
                this._planOne4(PLAN_PARTIAL_LINE, open_z, y, 1);
                // Draw complete lines between open_z (exclusive) and z
                if (open_z + 1 < z) {
                    this._planOne4(PLAN_FULL_LINES, open_z + 1, z, y);
                }
                // Open the new z
                open_z = z;
                this._planOne(PLAN_PARTIAL_LINE, open_z, y, 0);
            }
            this._planOne2(PLAN_SPRITE, sprite.id);
        }

        // Close open_z if necessary.
        if (open_z != -1) {
            this._planOne4(PLAN_PARTIAL_LINE, open_z, y, 1);
        }
        // Draw remaining lines
        if (open_z + 1 < CHUNK_SIZE) {
            this._planOne4(PLAN_FULL_LINES, open_z + 1, max_z, y);
        }
    },

    '_planOne': function(item) {
        if (this.plan_len == this.plan_.length) {
            this.plan_.push(item);
        } else {
            this.plan_[this.plan_len] = item;
        }
        ++this.plan_len;
    },

    '_planOne2': function(a, b) {
        this._planOne((a & 0xf) | (b & 0xf) << 4);
    },

    '_planOne3': function(a, b, c) {
        this._planOne((a & 0xf) | (b & 0xf) << 4 | (c & 0xf) << 8);
    },

    '_planOne4': function(a, b, c, d) {
        this._planOne((a & 0xf) | (b & 0xf) << 4 | (c & 0xf) << 8 | (d & 0xf) << 12);
    },

    '_planOne5': function(a, b, c, d, e) {
        this._planOne((a & 0xf) | (b & 0xf) << 4 | (c & 0xf) << 8 | (d & 0xf) << 12 | (e & 0xf) << 16);
    },

    'plan': function(sprites) {
        this._init(sprites);
        this._plan();
        this._cleanup();
        return this.plan_;
    },
};

function run_render_step(ctx, step, chunk, dx, dy, draw_sprite) {
    var type = step & 0xf;
    var arg0 = (step >> 4) & 0xf;
    var arg1 = (step >> 8) & 0xf;
    var arg2 = (step >> 12) & 0xf;
    var arg3 = (step >> 16) & 0xf;
    if (type == PLAN_FULL_LAYERS) {
        var min_z = arg0;
        var max_z = arg1 || 16;
        for (var z = min_z; z < max_z; ++z) {
            chunk.draw(ctx, z, 0, CHUNK_SIZE, dx, dy - z * TILE_SIZE);
        }
    } else if (type == PLAN_PARTIAL_LAYERS) {
        var min_z = arg0;
        var max_z = arg1 || 16;
        for (var z = min_z; z < max_z; ++z) {
            var min_y = arg2;
            var max_y = arg3 || 16;
            chunk.draw(ctx, z, min_y, max_y, dx, dy - z * TILE_SIZE);
        }
    } else if (type == PLAN_FULL_LINES) {
        var min_z = arg0;
        var max_z = arg1 || 16;
        var y = arg2;
        for (var z = min_z; z < max_z; ++z) {
            chunk.draw(ctx, z, y, y + 1, dx, dy - z * TILE_SIZE);
        }
    } else if (type == PLAN_PARTIAL_LINE) {
        var z = arg0;
        var y = arg1;
        var l = arg2;
        if (l == 0) {
            chunk.drawBottom(ctx, z, y, y + 1, dx, dy - z * TILE_SIZE);
        } else {
            chunk.drawFront(ctx, z, y, y + 1, dx, dy - z * TILE_SIZE);
        }
    } else if (type == PLAN_SPRITE) {
        draw_sprite(arg0);
    }
}

function describe_render_step(step) {
    var type = step & 0xf;
    var arg0 = (step >> 4) & 0xf;
    var arg1 = (step >> 8) & 0xf;
    var arg2 = (step >> 12) & 0xf;
    var arg3 = (step >> 16) & 0xf;
    if (type == PLAN_FULL_LAYERS) {
        var min_z = arg0;
        var max_z = (arg1 || 16) - 1;
        return 'FL: ' + ['_', '_', min_z + '..' + max_z].join(' x ');
    } else if (type == PLAN_PARTIAL_LAYERS) {
        var min_z = arg0;
        var max_z = (arg1 || 16) - 1;
        var min_y = arg2;
        var max_y = (arg3 || 16) - 1;
        return 'PL: ' + ['_', min_y + '..' + max_y, min_z + '..' + max_z].join(' x ');
    } else if (type == PLAN_FULL_LINES) {
        var min_z = arg0;
        var max_z = (arg1 || 16) - 1;
        var y = arg2;
        return 'FN: ' + ['_', y, min_z + '..' + max_z].join(' x ');
    } else if (type == PLAN_PARTIAL_LINE) {
        var z = arg0;
        var y = arg1;
        var l = arg2;
        return 'PN: ' + ['_', y, z].join(' x ') + (l == 0 ? ' (B)' : ' (F)');
    } else if (type == PLAN_SPRITE) {
        return 'Sprite: ' + arg0;
    }
}


function Physics(chunk_phys) {
    this._chunk_phys = chunk_phys;
}

Physics.prototype = {
    'resetForecast': function(now, f, v) {
        this._step(now, f);
        f.target_v = v;
        this._forecast(f);
    },

    'updateForecast': function(now, f) {
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
        if (i == LIMIT) {
            console.assert(false, "BUG: updateForecast got stuck but kept incrementing time");
        }
    },

    // Step the forecast forward to the given time, and set actual velocity to zero.
    '_step': function(time, f) {
        var pos = f.position(time);
        f.start = pos;
        f.end = pos;
        f.actual_v = new Vec(0, 0, 0);
        f.start_time = time;
        f.end_time = INT_MAX * 1000;
    },

    // Project the time of the next collision starting from start_time, and set
    // velocities, end_time, and end position appropriately.
    '_forecast': function(f) {
        var actual_v = f.target_v.clone();

        var slide_x = 0;
        var slide_y = 0;
        var slide_z = 0;

        // TODO: compute time limit for sliding
        while (actual_v.x != 0 || actual_v.y != 0 || actual_v.z != 0) {
            var coll = this._chunk_phys.collide(f.start, f.size, actual_v);

            if (coll.t == 0) {
                console.log('immediate collision with reason', coll.type);

                if (coll.type == COLLIDE_WALL || coll.type == COLLIDE_NO_FLOOR ||
                        coll.type == COLLIDE_CHUNK_BORDER) {
                    console.assert((coll.d & 0x111) != 0,
                            'immediate collision with no direction',
                            actual_v.x, actual_v.y, actual_v.z);
                    if (coll.d & 0x100) {
                        slide_x = Math.sign(actual_v.x);
                        actual_v.x = 0;
                    }
                    if (coll.d & 0x010) {
                        slide_y = Math.sign(actual_v.y);
                        actual_v.y = 0;
                    }
                    if (coll.d & 0x001) {
                        slide_z = Math.sign(actual_v.z);
                        actual_v.z = 0;
                    }
                    continue;
                } else if (coll.type == COLLIDE_RAMP_BOTTOM) {
                    // Only ramp type at the moment is RAMP_E
                    actual_v.z = actual_v.x;
                    console.log('RAMP');
                    coll = this._chunk_phys.collide(f.start, f.size, actual_v);
                } else {
                    console.assert('unknown collision type', coll.type);
                }
            }

            // Otherwise, the collision hasn't happened yet, so set up the
            // forecast for motion.

            f.end = new Vec(coll.x, coll.y, coll.z);
            f.end_time = f.start_time + coll.t;
            f.actual_v = actual_v;
            break;
        }

        // Can't move or slide along any axis.  Give up, and leave the forecast
        // in its current state.
    },
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

Forecast.prototype = {
    'position': function(now) {
        if (now < this.start_time) {
            return this.start.clone();
        } else if (now >= this.end_time) {
            return this.end.clone();
        } else {
            var delta = now - this.start_time;
            var offset = this.actual_v.mulScalar(delta).addScalar(500).divScalar(1000);
            return this.start.add(offset);
        }
    },

    'velocity': function() {
        return this.actual_v;
    },

    'target_velocity': function() {
        return this.target_v;
    },

    'live': function(now) {
        return now >= this.start_time && now < this.end_time;
    }
};


function ChunkPhysics(chunk) {
    this.chunk = chunk;
}

var COLLIDE_ZERO_VELOCITY = 1;
var COLLIDE_NO_FLOOR = 2;
var COLLIDE_WALL = 3;
var COLLIDE_SLIDE_END = 4;
var COLLIDE_CHUNK_BORDER = 5;
var COLLIDE_RAMP_BOTTOM = 6;

ChunkPhysics.prototype = {
    'walk': function(pos, velocity, callback) {
        if (velocity.x == 0 && velocity.y == 0 && velocity.z == 0) {
            throw 'ChunkPhysics.walk: zero velocity';
        }

        // We subdivide both time and space into `u` subpixels and `u`
        // timesteps per second.  The result is that all interesting events
        // occur at an integer number of subpixels and timesteps.
        var units = 1;
        velocity.forEach(function(w) {
            if (w != 0) {
                units = lcm(units, Math.abs(w));
            }
        });

        var side = velocity.isPositive();
        var first_plane = pos.add(side.mulScalar(TILE_SIZE - 1))
            .divScalar(TILE_SIZE).mulScalar(TILE_SIZE);

        var pixel_time = velocity.map(function(v) {
            return v != 0 ? Math.abs((units / v)|0) : 0;
        });

        var next = pixel_time.zip3(first_plane, pos, function(p, first, start) {
            return p != 0 ? p * Math.abs(first - start) : 999999999;
        });

        var time = 0;

        for (var steps = 0; steps < CHUNK_SIZE * 3; ++steps) {
            time = Math.min(next.x, next.y, next.z);
            var hit = next.map(function(t) { return t == time ? 1 : 0; });
            next = next.add(pixel_time.mul(hit).mulScalar(TILE_SIZE));

            var cur_pos = pos.add(velocity.mulScalar(time).divScalar(units));
            var cur_time = (1000 * time / units)|0;
            var result = callback(cur_pos, cur_time, hit);
            if (result != null) {
                return result;
            }
        }

        return null;
    },

    'collide': function(pos, size, velocity) {
        var side = velocity.isPositive();
        var dir = velocity.sign();
        var corner = pos.add(side.mul(size));

        var chunk = this.chunk;

        function make_result(pos, dirs, time, reason) {
            return ({
                'x': pos.x,
                'y': pos.y,
                'z': pos.z,
                'd': dirs,
                't': time,
                'type': reason,
            });
        }

        //window.physTrace = [];

        var this_ = this;

        function check_plane(min, max, dir) {
            var seen_ramp_bottom = false;
            var seen_floor = false;

            var reason = this_.checkPlane(min, max, dir, function(x, y, z) {
                //window.physTrace.push([x,y,z]);
                var shape = this_.chunk.shape(x, y, z);
                if (z == min.z) {
                    if (shape == SHAPE_EMPTY) {
                        return COLLIDE_NO_FLOOR;
                    } else if (shape == SHAPE_RAMP_E && dir.x == 1 && dir.y == 0 && dir.z == 0) {
                        seen_ramp_bottom = true;
                    } else if (shape == SHAPE_FLOOR) {
                        seen_floor = true;
                    } else {
                        return COLLIDE_WALL;
                    }
                } else {
                    if (shape != SHAPE_EMPTY) {
                        return COLLIDE_WALL;
                    }
                }
                return 0;
            });

            if (reason == 0 && seen_ramp_bottom) {
                if (!seen_floor) {
                    return COLLIDE_RAMP_BOTTOM;
                } else {
                    return COLLIDE_WALL;
                }
            } else {
                return reason;
            }
        }

        return this.walk(corner, velocity, function(cur, time, hit) {
            var base = cur.sub(side.mul(size));

            var bounds = this_.hitChunkBoundaries(cur, hit, side);
            if (bounds != 0) {
                return make_result(base, bounds, time, COLLIDE_CHUNK_BORDER);
            }

            var min = base.divScalar(TILE_SIZE);
            var max = base.add(size).addScalar(TILE_SIZE - 1).divScalar(TILE_SIZE);
            // Tile coordinates of the plane(s) we collided with.
            var facing = cur.divScalar(TILE_SIZE);

            var DIRS = [0x100, 0x010, 0x001, 0x110, 0x011, 0x101, 0x111];
            var all_dirs = (hit.x << 8) | (hit.y << 4) | (hit.z);
            for (var i = 0; i < DIRS.length; ++i) {
                var dirs = DIRS[i];
                if (dirs & ~all_dirs) {
                    continue;
                }

                var cur_hit = new Vec(!!(dirs & 0x100), !!(dirs & 0x010), !!(dirs & 0x001));

                var reason = check_plane(
                        cur_hit.choose(facing, min),
                        cur_hit.choose(facing, max),
                        dir.mul(cur_hit));
                if (reason != 0) {
                    return make_result(base, dirs, time, reason);
                }
            }

            return null;
        });
    },

    'hitChunkBoundaries': function(cur, hit, side) {
        var bound_x = hit.x && cur.x == side.x * CHUNK_SIZE * TILE_SIZE;
        var bound_y = hit.y && cur.y == side.y * CHUNK_SIZE * TILE_SIZE;
        var bound_z = hit.z && cur.z == side.z * CHUNK_SIZE * TILE_SIZE;
        return (bound_x << 8) | (bound_y << 4) | (bound_z);
    },

    'checkPlane': function(min, max, dir, callback) {
        min = min.sub(dir.isNegative()).clamp(0, CHUNK_SIZE);
        max = max.add(dir.isPositive()).clamp(0, CHUNK_SIZE);

        for (var z = min.z; z < max.z; ++z) {
            for (var y = min.y; y < max.y; ++y) {
                for (var x = min.x; x < max.x; ++x) {
                    var reason = callback(x, y, z);
                    if (reason != 0) {
                        return reason;
                    }
                }
            }
        }

        return 0;
    },
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
    phys._chunk_phys.collide(new Vec(0, 0, 0), new Vec(32, 32, 32), new Vec(30, 0, 0));
};


var anim_canvas = new AnimCanvas(frame);
window.anim_canvas = anim_canvas;
document.body.appendChild(anim_canvas.canvas);

anim_canvas.ctx.fillStyle = '#f0f';
anim_canvas.ctx.strokeStyle = '#0ff';
anim_canvas.ctx.lineWidth = 2;
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

var assets = loader.assets;
window.assets = assets;

function bake_sprite_sheet(runner) {
    var width = assets.pony_f_base.width;
    var height = assets.pony_f_base.height;

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
        runner.subjob('wing_back',  tinted, assets.pony_f_wing_back, coat_color);
        runner.subjob('base',       tinted, assets.pony_f_base, coat_color);
        runner.subjob('eyes',       copy, assets.pony_f_eyes_blue);
        runner.subjob('wing_front', tinted, assets.pony_f_wing_front, coat_color);
        runner.subjob('tail',       tinted, assets.pony_f_tail_1, hair_color);
        runner.subjob('mane',       tinted, assets.pony_f_mane_1, hair_color);
        runner.subjob('horn',       tinted, assets.pony_f_horn, coat_color);
    });

    return baked.canvas;
}

var tileSheet = new Sheet(assets.tiles1, 32, 32);
var sheet;
var pony;

var runner = new BackgroundJobRunner();

loader.onload = function() {
    sheet = new Sheet(bake_sprite_sheet(runner), 96, 96);
    pony = new Pony(sheet, 100, 100, 0);
    window.pony = pony;

    document.body.removeChild($('banner-bg'));
    anim_canvas.start();

    for (var i = 0; i < 4; ++i) {
        chunkRender[i].bake();
    }
};

loader.onprogress = function(loaded, total) {
    $('banner-text').textContent = 'Loading... (' + loaded + '/' + total + ')';
    $('banner-bar').style.width = Math.floor(loaded / total * 100) + '%';
};

var chunks = [];
var chunkRender = [];
window.chunkRender = chunkRender;
for (var i = 0; i < 4; ++i) {
    var chunk = new Chunk();
    chunks.push(chunk);
    for (var y = 0; y < CHUNK_SIZE; ++y) {
        for (var x = 0; x < CHUNK_SIZE; ++x) {
            var rnd = (x * 7 + y * 13 + 31) >> 2;
            var a = (rnd & 1);
            var b = (rnd & 2) >> 1;
            var tile = (4 + a) * 16 + 14 + b;
            chunk.set(x, y, 0, tile, 0, SHAPE_FLOOR);
        }
    }
    chunkRender.push(new ChunkRendering(chunk, tileSheet));
}

for (var i = 0; i < 3; ++i) {
    for (var j = 0; j < 2; ++j) {
        chunks[0].setFront(10+j, 10, i, (15-i) * 16 + 7+j);
        chunks[0].setShape(10+j, 10, i, SHAPE_SOLID);
    }
}

chunks[0].setFront(10, 4, 0, 12 * 16 + 14);
chunks[0].setFront(11, 4, 0, 12 * 16 + 15);
chunks[0].setFront(10, 5, 2, 11 * 16 + 14);
chunks[0].setFront(11, 5, 2, 11 * 16 + 15);
chunks[0].setBottom(10, 4, 2, 10 * 16 + 14);
chunks[0].setBottom(11, 4, 2, 10 * 16 + 15);

chunks[0].set(5, 10, 0, 0, 0, SHAPE_EMPTY);
chunks[0].set(7, 10, 0, 0, 0, SHAPE_EMPTY);

chunks[0].setShape(5, 3, 0, SHAPE_RAMP_E);
chunks[0].setShape(5, 4, 0, SHAPE_RAMP_E);
chunks[0].setShape(6, 3, 1, SHAPE_RAMP_E);
chunks[0].setShape(6, 4, 1, SHAPE_RAMP_E);
chunks[0].setShape(7, 3, 2, SHAPE_FLOOR);
chunks[0].setShape(7, 4, 2, SHAPE_FLOOR);
chunks[0].setShape(8, 3, 2, SHAPE_FLOOR);
chunks[0].setShape(8, 4, 2, SHAPE_FLOOR);

var planner = new RenderPlanner();
var phys = new Physics(new ChunkPhysics(chunks[0]));
window.phys = phys;
window.planner = planner;
window.physTrace = [];

function frame(ctx, now) {
    dbg.frameStart();
    var pos = pony.position(now);
    ctx.clearRect(0, 0, ctx.canvas.width, ctx.canvas.height);

    var sprite_y = Math.floor((pos.y + 16) / 32);

    var sprites = [
            {'y': sprite_y, 'z': 0, 'size_z': 2, 'id': 0},
        ];
    window.sprites = sprites;
    var plan = planner.plan(sprites);

    for (var i = 0; i < plan.length; ++i) {
        run_render_step(ctx, plan[i], chunkRender[0], 0, 0, function(i) {
            pony.drawInto(ctx, now);
        });
    }

    var coll = window.physTrace;
    ctx.strokeStyle = '#00f';
    for (var i = 0; i < coll.length; ++i) {
        var p = coll[i];
        if (i == coll.length - 1) {
            ctx.strokeStyle = '#a00';
        }
        ctx.strokeRect(p[0] * 32, p[1] * 32, 32, 32);
        ctx.fillText(i, p[0] * 32, p[1] * 32 + 10);
    }

    ctx.strokeStyle = '#cc0';
    ctx.beginPath();
    var fc = pony._forecast;
    ctx.moveTo(fc.start.x + 16, fc.start.y + 16 - fc.start.z);
    ctx.lineTo(fc.end.x + 16, fc.end.y + 16 - fc.start.z);
    ctx.stroke();

    ctx.strokeStyle = '#888';
    ctx.beginPath();
    ctx.moveTo(5*32, 3*32);
    ctx.lineTo(7*32, 1*32);
    ctx.lineTo(9*32, 1*32);
    ctx.lineTo(9*32, 3*32);
    ctx.lineTo(7*32, 3*32);
    ctx.lineTo(5*32, 5*32);
    ctx.closePath();
    ctx.moveTo(7*32, 1*32);
    ctx.lineTo(7*32, 3*32);
    ctx.moveTo(9*32, 3*32);
    ctx.lineTo(9*32, 5*32);
    ctx.lineTo(5*32, 5*32);
    ctx.stroke();

    dbg.frameEnd();

    runner.run(now, 10);
    dbg.updateJobs(runner);
    dbg.updatePlan(plan);
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

})();
