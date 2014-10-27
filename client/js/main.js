var $ = document.getElementById.bind(document);


var Vec = require('vec').Vec;
var AnimCanvas = require('canvas').AnimCanvas;
var OffscreenContext = require('canvas').OffscreenContext;
var DebugMonitor = require('debug').DebugMonitor;
var Sheet = require('sheet').Sheet;
var Animation = require('sheet').Animation;
var AssetLoader = require('loader').AssetLoader;
var BackgroundJobRunner = require('jobs').BackgroundJobRunner;

var Chunk = require('chunk').Chunk;
var TileDef = require('chunk').TileDef;
var CHUNK_SIZE = require('chunk').CHUNK_SIZE;
var TILE_SIZE = require('chunk').TILE_SIZE;
var LOCAL_SIZE = require('chunk').LOCAL_SIZE;

var TerrainGraphics = require('graphics').TerrainGraphics;
var Physics = require('physics').Physics;
var Forecast = require('physics').Forecast;


function Pony(sheet, x, y, z, physics) {
    this._anim = new Animation(sheet);
    this._anim.animate(0, 2, 1, 1, false, 0);
    this._last_dir = { x: 1, y: 0 };
    this._forecast = new Forecast(new Vec(x - 16, y - 16, z), new Vec(32, 32, 32));
    this._phys = physics;
    this._phys.resetForecast(0, this._forecast, new Vec(0, 0, 0));
}

Pony.prototype.walk = function(now, speed, dx, dy) {
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
    this._phys.resetForecast(now, this._forecast, target_v);
};

Pony.prototype.position = function(now) {
    this._phys.updateForecast(now, this._forecast);
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


function bakeSpriteSheet(runner, assets) {
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
        runner.subjob('eyes',       copy,   assets['pony_f_eyes_blue']);
        runner.subjob('wing_front', tinted, assets['pony_f_wing_front'], coat_color);
        runner.subjob('tail',       tinted, assets['pony_f_tail_1'], hair_color);
        runner.subjob('mane',       tinted, assets['pony_f_mane_1'], hair_color);
        runner.subjob('horn',       tinted, assets['pony_f_horn'], coat_color);
    });

    return baked.canvas;
}


var canvas;
var ctx;
var debug;
var runner;
var loader;
var assets;

var pony;

var chunks;
var physics;
var graphics;

var socket;

function init() {
    canvas = new AnimCanvas(frame);
    document.body.appendChild(canvas.canvas);

    ctx = canvas.ctx;
    ctx.fillStyle = '#f0f';
    ctx.strokeStyle = '#0ff';
    ctx.imageSmoothingEnabled = false;
    ctx.mozImageSmoothingEnabled = false;

    debug = new DebugMonitor();
    document.body.appendChild(debug.container);

    runner = new BackgroundJobRunner();

    loader = new AssetLoader();
    assets = loader.assets;
    loader.onprogress = assetProgress;
    loader.onload = postInit;
    initAssets(loader);

    pony = null;

    chunks = initChunks();
    physics = new Physics();
    var tile_sheet = new Sheet(assets['tiles1'], 32, 32);
    graphics = new TerrainGraphics(tile_sheet);

    initInput();
}
document.addEventListener('DOMContentLoaded', init);

function initAssets() {
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
        for (var i = 0; i < tiles.length; ++i) {
            TileDef.register(i, tiles[i]);
        }
    });
}

function initChunks() {
    var chunks = [];
    for (var i = 0; i < LOCAL_SIZE * LOCAL_SIZE; ++i) {
        chunks.push(new Chunk());
    }
    return chunks;
}

function initInput() {
    var dirs_held = {
        'Up': false,
        'Down': false,
        'Left': false,
        'Right': false,
        'Shift': false,
    };

    document.addEventListener('keydown', function(evt) {
        var known = true;
        if (dirs_held.hasOwnProperty(evt.key)) {
            if (!evt.repeat) {
                dirs_held[evt.key] = true;
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
        if (dirs_held.hasOwnProperty(evt.key)) {
            evt.preventDefault();
            evt.stopPropagation();
            dirs_held[evt.key] = false;
            updateWalkDir();
        }
    });

    function updateWalkDir() {
        var dx = 0;
        var dy = 0;
        var speed = 1;

        if (dirs_held['Left']) {
            dx -= 1;
        }
        if (dirs_held['Right']) {
            dx += 1;
        }

        if (dirs_held['Up']) {
            dy -= 1;
        }
        if (dirs_held['Down']) {
            dy += 1;
        }

        if (dirs_held['Shift']) {
            speed = 3;
        }

        pony.walk(Date.now(), speed, dx, dy, physics);
    }
}

function assetProgress(loaded, total) {
    $('banner-text').textContent = 'Loading... (' + loaded + '/' + total + ')';
    $('banner-bar').style.width = Math.floor(loaded / total * 100) + '%';
};

function postInit() {
    var pony_sheet = new Sheet(bakeSpriteSheet(runner, assets), 96, 96);
    pony = new Pony(pony_sheet, 100, 100, 0, physics);

    document.body.removeChild($('banner-bg'));
    canvas.start();

    socket = new WebSocket('ws://' + window.location.host + '/ws');
    socket.binaryType = 'arraybuffer';
    socket.onopen = socketOpen;
    socket.onmessage = socketMessage;
}

function socketOpen() {
    var buf = new Uint16Array(1);
    buf[0] = 0x0001;
    socket.send(buf);
    console.log('sent request');
}

function socketMessage(evt) {
    var buf = new Uint16Array(evt.data);

    if (buf[0] == 0x8001) {
        if (buf.length != 2 + CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) {
            console.assert(false, 'bad length for message:', buf.length);
            return;
        }
        var i = buf[1];
        var chunk = chunks[i];
        chunk._tiles.set(buf.subarray(2));
        runner.job('load-chunk-' + i, function() {
            physics.loadChunk(0, i, chunk._tiles);
            graphics.loadChunk(0, i, chunk._tiles);
        });
    }
}

function frame(ctx, now) {
    debug.frameStart();
    var pos = pony.position(now);

    var local_total_size = CHUNK_SIZE * TILE_SIZE * LOCAL_SIZE;

    if (pos.x < local_total_size / 2) {
        pony._forecast.start.x += local_total_size;
        pony._forecast.end.x += local_total_size;
    } else if (pos.x >= local_total_size * 3 / 2) {
        pony._forecast.start.x -= local_total_size;
        pony._forecast.end.x -= local_total_size;
    }

    if (pos.y < local_total_size / 2) {
        pony._forecast.start.y += local_total_size;
        pony._forecast.end.y += local_total_size;
    } else if (pos.y >= local_total_size * 3 / 2) {
        pony._forecast.start.y -= local_total_size;
        pony._forecast.end.y -= local_total_size;
    }

    pos = pony.position(now);
    debug.updatePos(pos);

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

            graphics.render(ctx, cy, cx, sprites);

            ctx.restore();
        }
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


    debug.frameEnd();

    debug.updateJobs(runner);
}
