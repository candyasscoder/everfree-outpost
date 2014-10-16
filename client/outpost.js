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

    'setBottom': function(x, y, z, bottom) {
        this.set(x, y, z, bottom, this.front(x, y, z));
    },

    'setFront': function(x, y, z, front) {
        this.set(x, y, z, this.bottom(x, y, z), front);
    },
};


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
            chunk.set(x, y, 0, tile, 0);
        }
    }
    chunkRender.push(new ChunkRendering(chunk, tileSheet));
}

for (var i = 0; i < 3; ++i) {
    for (var j = 0; j < 2; ++j) {
        chunks[0].setFront(10+j, 10, i, (15-i) * 16 + 7+j);
    }
}

var planner = new RenderPlanner();

function frame(ctx, now) {
    dbg.frameStart();
    var pos = pony.position(now);
    ctx.clearRect(0, 0, ctx.canvas.width, ctx.canvas.height);

    var sprite_y = Math.floor((pos.y + 78) / 32);


    var sprites = [
            {'y': sprite_y, 'z': 0, 'size_z': 2, 'id': 0},
        ];
    var plan = planner.plan(sprites);

    for (var i = 0; i < plan.length; ++i) {
        run_render_step(ctx, plan[i], chunkRender[0], 0, 0, function(i) {
            pony.drawInto(ctx, now);
        });
    }

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

    pony.walk(Date.now(), speed, dx, dy);
}

})();
