var $ = document.getElementById.bind(document);


var Vec = require('vec').Vec;
var AnimCanvas = require('canvas').AnimCanvas;
var OffscreenContext = require('canvas').OffscreenContext;
var DebugMonitor = require('debug').DebugMonitor;
var Sheet = require('sheet').Sheet;
var Animation = require('sheet').Animation;
var AssetLoader = require('loader').AssetLoader;
var BackgroundJobRunner = require('jobs').BackgroundJobRunner;
var Entity = require('entity').Entity;
var Motion = require('entity').Motion;

var Chunk = require('chunk').Chunk;
var TileDef = require('chunk').TileDef;
var CHUNK_SIZE = require('chunk').CHUNK_SIZE;
var TILE_SIZE = require('chunk').TILE_SIZE;
var LOCAL_SIZE = require('chunk').LOCAL_SIZE;

var Renderer = require('graphics').Renderer;
var Physics = require('physics').Physics;
var Forecast = require('physics').Forecast;

var Connection = require('net').Connection;

var rle16Decode = require('util').rle16Decode;


var anim_dirs = [
    // Start facing in +x, then cycle toward +y (clockwise, since y points
    // downward).
    {idx: 2, flip: false},
    {idx: 3, flip: false},
    {idx: 4, flip: false},
    {idx: 3, flip: true},
    {idx: 2, flip: true},
    {idx: 1, flip: true},
    {idx: 0, flip: false},
    {idx: 1, flip: false},
];

var pony_anims = new Array(4 * anim_dirs.length);
for (var i = 0; i < anim_dirs.length; ++i) {
    var idx = anim_dirs[i].idx;
    var flip = anim_dirs[i].flip;

    pony_anims[i] = {
        i: 0,
        j: idx,
        len: 1,
        fps: 1,
        flip: flip,
    };

    for (var speed = 1; speed < 4; ++speed) {
        pony_anims[speed * anim_dirs.length + i] = {
            i: speed,
            j: idx * 6,
            len: 6,
            fps: 6 + 2 * speed,
            flip: flip,
        };
    }
}

function Pony(sheet, x, y, z, physics) {
    this._entity = new Entity(sheet, pony_anims, new Vec(x, y + 16, z), {x: 48, y: 74});
    this._entity.setAnimation(0, 0);
    this._last_dir = { x: 1, y: 0 };
    this._forecast = new Forecast(new Vec(x - 16, y - 16, z), new Vec(32, 32, 32));
    this._phys = physics;
    this.onMotionChange = null;
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

    var idx = 3 * (dx + 1) + (dy + 1);
    var dir = [5, 4, 3, 6, -1, 2, 7, 0, 1][idx];
    this._entity.setAnimation(now, speed * anim_dirs.length + dir);

    var pixel_speed = 50 * speed;
    var target_v = new Vec(dx * pixel_speed, dy * pixel_speed, 0);
    this._phys.resetForecast(now, this._forecast, target_v);
    if (this.onMotionChange != null) {
        this.onMotionChange(this._forecast);
        this._entity.setMotion(Motion.fromForecast(this._forecast, new Vec(16, 32, 16)));
    }
};

Pony.prototype.position = function(now) {
    var old_start = this._forecast.start_time;
    this._phys.updateForecast(now, this._forecast);
    if (this._forecast.start_time != old_start && this.onMotionChange != null) {
        this.onMotionChange(this._forecast);
        this._entity.setMotion(Motion.fromForecast(this._forecast, new Vec(16, 32, 16)));
    }

    var pos = this._forecast.position(now);
    pos.x += 16;
    pos.y += 16;
    return pos;
};

Pony.prototype.getSprite = function(now) {
    this.position(now); // update forecast, then ignore result
    return this._entity.getSprite(now);
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
var renderer;

var conn;

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
    var tile_sheet = new Sheet(assets['tiles'], 32, 32);
    renderer = new Renderer(tile_sheet);

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

    loader.addImage('tiles', 'assets/tiles.png');

    loader.addJson(null, 'tiles.json', function(json) {
        var tiles = json['tiles'];
        for (var i = 0; i < tiles.length; ++i) {
            TileDef.register(i, tiles[i]);
        }
        renderer.loadBlockData(TileDef.by_id);
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
    pony.onMotionChange = sendMotionChange;

    document.body.removeChild($('banner-bg'));
    canvas.start();


    conn = new Connection('ws://' + window.location.host + '/ws');
    conn.onOpen = connOpen;
    conn.onTerrainChunk = handleTerrainChunk;
}

function connOpen() {
    conn.sendGetTerrain();
}

function handleTerrainChunk(i, data) {
    var chunk = chunks[i];
    var raw_length = rle16Decode(data, chunk._tiles);

    if (raw_length != CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) {
        console.assert(false,
                'chunk data contained wrong number of tiles:', raw_length);
    }

    if (i == 0) {
        chunk.set(0, 0, 0, 26);
        chunk.set(0, 0, 1, 27);
        chunk.set(0, 0, 2, 28);
        chunk.set(1, 0, 0, 29);
        chunk.set(1, 0, 1, 30);
        chunk.set(1, 0, 2, 31);
    }

    runner.job('load-chunk-' + i, function() {
        physics.loadChunk(0, i, chunk._tiles);
        renderer.loadChunk(0, i, chunk);
    });
}

function sendMotionChange(forecast) {
    var data = new Uint16Array(8);
    data[0] = forecast.start.x;
    data[1] = forecast.start.y;
    data[2] = forecast.start.z;
    data[3] = forecast.start_time & 0xffff;
    data[4] = forecast.end.x;
    data[5] = forecast.end.y;
    data[6] = forecast.end.z;
    data[7] = forecast.end_time & 0xffff;
    conn.sendUpdateMotion(data);
}

function frame(ctx, now) {
    debug.frameStart();
    var pos = pony.position(now);

    var local_total_size = CHUNK_SIZE * TILE_SIZE * LOCAL_SIZE;

    if (pos.x < local_total_size / 2) {
        pony._forecast.start.x += local_total_size;
        pony._forecast.end.x += local_total_size;
        pony._entity._motion.start_pos.x += local_total_size;
        pony._entity._motion.end_pos.x += local_total_size;
    } else if (pos.x >= local_total_size * 3 / 2) {
        pony._forecast.start.x -= local_total_size;
        pony._forecast.end.x -= local_total_size;
        pony._entity._motion.start_pos.x -= local_total_size;
        pony._entity._motion.end_pos.x -= local_total_size;
    }

    if (pos.y < local_total_size / 2) {
        pony._forecast.start.y += local_total_size;
        pony._forecast.end.y += local_total_size;
        pony._entity._motion.start_pos.y += local_total_size;
        pony._entity._motion.end_pos.y += local_total_size;
    } else if (pos.y >= local_total_size * 3 / 2) {
        pony._forecast.start.y -= local_total_size;
        pony._forecast.end.y -= local_total_size;
        pony._entity._motion.start_pos.y -= local_total_size;
        pony._entity._motion.end_pos.y -= local_total_size;
    }

    pos = pony.position(now);
    debug.updatePos(pos);

    var camera_size = new Vec(ctx.canvas.width|0, ctx.canvas.height|0, 0);
    var camera_pos = pos.sub(camera_size.divScalar(2));

    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.clearRect(0, 0, ctx.canvas.width, ctx.canvas.height);

    ctx.translate(-camera_pos.x, -camera_pos.y);


    var sprites = [pony.getSprite(now)];

    renderer.render(ctx,
            camera_pos.x, camera_pos.y,
            ctx.canvas.width, ctx.canvas.height,
            sprites);


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
