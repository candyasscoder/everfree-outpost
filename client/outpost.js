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
            console.log('ran', job.name, 'in', end - start);
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


function Entity(sheet, x, y) {
    this.sheet = sheet;
    this._motion = {
        'last_x': x,
        'last_y': y,
        'velocity_x': 0,
        'velocity_y': 0,
        'start': 0,
    };
    this._anim = null;
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

    'move': function(vx, vy, now) {
        var pos = this.position(now);
        this._motion = {
            'last_x': pos.x,
            'last_y': pos.y,
            'velocity_x': vx,
            'velocity_y': vy,
            'start': now,
        };
    },

    'position': function(now) {
        var motion = this._motion;
        var delta = now - motion.start;
        var x = motion.last_x + Math.floor(delta * motion.velocity_x / 1000);
        var y = motion.last_y + Math.floor(delta * motion.velocity_y / 1000);
        return { 'x': x, 'y': y }
    },

    'drawInto': function(ctx, now) {
        var pos = this.position(now);
        var x = pos.x;
        var y = pos.y;

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


function Pony(sheet, x, y) {
    this._entity = new Entity(sheet, x, y);
    this._entity.animate(0, 2, 1, 1, false, 0);
    this._last_dir = { 'x': 1, 'y': 0 };
}

Pony.prototype = {
    'walk': function(now, speed, dx, dy) {
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
        entity.move(dx * pixel_speed, dy * pixel_speed, now);
    },

    'position': function(now) {
        return this._entity.position(now);
    },

    'drawInto': function(ctx, now) {
        this._entity.drawInto(ctx, now);
    },
};


var CHUNK_SIZE = 16;
var TILE_SIZE = 32;

function Chunk() {
    this._arr = new Uint32Array(CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE);
}

Chunk.prototype = {
    '_cell': function(x, y, z) {
        return this._arr[((z) * CHUNK_SIZE + y) * CHUNK_SIZE + x];
    },

    '_cellBits': function(x, y, z, start, count) {
        var mask = (1 << count) - 1;
        return (this._cell(x, y, z) >> start) & mask;
    },

    'bottom': function(x, y, z) {
        return this._cellBits(x, y, z, 0, 8);
    },

    'front': function(x, y, z) {
        return this._cellBits(x, y, z, 8, 8);
    },

    'set': function(x, y, z, bottom, front) {
        var cell = bottom | (front << 8);
        this._arr[((z) * CHUNK_SIZE + y) * CHUNK_SIZE + x] = cell;
    },
};


function ChunkRendering(chunk, sheet) {
    this.chunk = chunk;
    this.sheet = sheet;
}

ChunkRendering.prototype = {
    'drawBottom': function(ctx, z, dx, dy) {
        for (var y = 0; y < CHUNK_SIZE; ++y) {
            for (var x = 0; x < CHUNK_SIZE; ++x) {
                var tile = this.chunk.bottom(x, y, z);
                if (tile != 0) {
                    this.sheet.drawInto(ctx, tile >> 4, tile & 15,
                            dx + x * TILE_SIZE, dy + y * TILE_SIZE);
                }
            }
        }
    },

    'drawFront': function(ctx, z, min_y, max_y, dx, dy) {
        for (var y = min_y; y < max_y; ++y) {
            for (var x = 0; x < CHUNK_SIZE; ++x) {
                var tile = this.chunk.front(x, y, z);
                if (tile != 0) {
                    this.sheet.drawInto(ctx, tile >> 4, tile & 15,
                            dx + x * TILE_SIZE, dy + y * TILE_SIZE);
                }
            }
        }
    },
};


var anim_canvas = new AnimCanvas(frame);
window.anim_canvas = anim_canvas;
document.body.appendChild(anim_canvas.canvas);

anim_canvas.ctx.fillStyle = '#f0f';
anim_canvas.ctx.strokeStyle = '#0ff';
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
        runner.subjob('mane',       tinted, assets.pony_f_mane_1, hair_color);
        runner.subjob('tail',       tinted, assets.pony_f_tail_1, hair_color);
        runner.subjob('horn',       tinted, assets.pony_f_horn, coat_color);
        runner.subjob('wing_front', tinted, assets.pony_f_wing_front, coat_color);
    });

    return baked.canvas;
}

var tileSheet = new Sheet(assets.tiles1, 32, 32);
var sheet;
var pony;

var runner = new BackgroundJobRunner();

loader.onload = function() {
    sheet = new Sheet(bake_sprite_sheet(runner), 96, 96);
    pony = new Pony(sheet, 100, 100);
    window.pony = pony;

    document.body.removeChild($('banner-bg'));
    anim_canvas.start();
};

loader.onprogress = function(loaded, total) {
    $('banner-text').textContent = 'Loading... (' + loaded + '/' + total + ')';
    $('banner-bar').style.width = Math.floor(loaded / total * 100) + '%';
};

var chunks = [];
var chunkRender = [];
for (var i = 0; i < 4; ++i) {
    var chunk = new Chunk();
    chunks.push(chunk);
    for (var y = 0; y < CHUNK_SIZE; ++y) {
        for (var x = 0; x < CHUNK_SIZE; ++x) {
            var rnd = (x * 7 + y * 13 + 31) >> 2;
            var a = (rnd & 1);
            var b = (rnd & 2) >> 1;
            var tile = (4 + a) * 16 + 14 + b;
            chunk.set(x, y, 0, tile, 0);
        }
    }
    chunkRender.push(new ChunkRendering(chunk, tileSheet));
}

function frame(ctx, now) {
    dbg.frameStart();
    var pos = pony.position(now);
    ctx.clearRect(0, 0, ctx.canvas.width, ctx.canvas.height);

    for (var i = 0; i < 2; ++i) {
        for (var j = 0; j < 2; ++j) {
            chunkRender[i * 2 + j].drawBottom(ctx, 0, i * 512, j * 512);
        }
    }

    pony.drawInto(ctx, now);

    dbg.frameEnd();

    runner.run(now, 10);
    dbg.updateJobs(runner);
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

    pony.walk(Date.now(), speed, dx, dy);
}

})();
