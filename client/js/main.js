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
var Timing = require('time').Timing;

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

/** @constructor */
function Pony(sheet, x, y, z, physics) {
    this._entity = new Entity(sheet, pony_anims, new Vec(x, y + 16, z), {x: 48, y: 74});
    this._entity.setAnimation(0, 0);
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

    var idx = 3 * (dx + 1) + (dy + 1);
    var dir = [5, 4, 3, 6, -1, 2, 7, 0, 1][idx];
    this._entity.setAnimation(now, speed * anim_dirs.length + dir);

    var pixel_speed = 50 * speed;
    var target_v = new Vec(dx * pixel_speed, dy * pixel_speed, 0);
    this._phys.resetForecast(now, this._forecast, target_v);
    this._entity.setMotion(Motion.fromForecast(this._forecast, new Vec(16, 32, 0)));
};

Pony.prototype.position = function(now) {
    var old_start = this._forecast.start_time;
    this._phys.updateForecast(now, this._forecast);
    if (this._forecast.start_time != old_start) {
        this._entity.setMotion(Motion.fromForecast(this._forecast, new Vec(16, 32, 0)));
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

Pony.prototype.translateMotion = function(offset) {
    this._entity.translateMotion(offset);
    this._forecast.start = this._forecast.start.add(offset);
    this._forecast.end = this._forecast.end.add(offset);
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
        runner.subjob('copy', function() {
            temp.globalCompositeOperation = 'copy';
            temp.drawImage(img, 0, 0);
        });

        runner.subjob('color', function() {
            temp.globalCompositeOperation = 'source-in';
            temp.fillStyle = color;
            temp.fillRect(0, 0, width, height);
        });

        runner.subjob('multiply', function() {
            temp.globalCompositeOperation = 'multiply';
            temp.drawImage(img, 0, 0);
        });

        runner.subjob('draw', function() {
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
var debug;
var runner;
var loader;
var assets;

var pony_sheet;
var entities;
var player_entity;

var chunks;
var chunkLoaded;
var physics;
var renderer = null;

var conn;
var timing;

function init() {
    canvas = new AnimCanvas(frame, 'webgl');
    document.body.appendChild(canvas.canvas);

    debug = new DebugMonitor();
    document.body.appendChild(debug.container);

    runner = new BackgroundJobRunner();

    loader = new AssetLoader();
    assets = loader.assets;
    loader.onprogress = assetProgress;
    loader.onload = postInit;
    initAssets(loader);

    entities = {};
    player_entity = -1;

    chunks = initChunks();
    physics = new Physics();
    var tile_sheet = new Sheet(assets['tiles'], 32, 32);

    chunkLoaded = new Array(LOCAL_SIZE * LOCAL_SIZE);
    for (var i = 0; i < chunkLoaded.length; ++i) {
        chunkLoaded[i] = false;
    }

    renderer = new Renderer(canvas.ctx);

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
        var tiles = json['blocks'];
        for (var i = 0; i < tiles.length; ++i) {
            TileDef.register(i, tiles[i]);
        }
        renderer.loadBlockData(TileDef.by_id);
    });

    loader.addText('terrain.frag', 'assets/shaders/terrain.frag');
    loader.addText('terrain.vert', 'assets/shaders/terrain.vert');

    loader.addText('sprite.frag', 'assets/shaders/sprite.frag');
    loader.addText('sprite.vert', 'assets/shaders/sprite.vert');
}

function initChunks() {
    var chunks = [];
    for (var i = 0; i < LOCAL_SIZE * LOCAL_SIZE; ++i) {
        chunks.push(new Chunk());
    }
    return chunks;
}

var INPUT_LEFT =    0x0001;
var INPUT_RIGHT =   0x0002;
var INPUT_UP =      0x0004;
var INPUT_DOWN =    0x0008;
var INPUT_RUN =     0x0010;

var ACTION_USE =    0x0001;

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
            known = sendActionForKey(evt.key);
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
        var bits = 0;

        if (dirs_held['Left']) {
            bits |= INPUT_LEFT;
        }
        if (dirs_held['Right']) {
            bits |= INPUT_RIGHT;
        }

        if (dirs_held['Up']) {
            bits |= INPUT_UP;
        }
        if (dirs_held['Down']) {
            bits |= INPUT_DOWN;
        }

        if (dirs_held['Shift']) {
            bits |= INPUT_RUN;
        }

        var now = Date.now();
        conn.sendInput(timing.encodeSend(now + 10), bits);
    }

    function sendActionForKey(key) {
        var code = 0;
        switch (key) {
            case ' ': code = ACTION_USE; break;
            default: return false;
        }

        var now = Date.now();
        conn.sendAction(timing.encodeSend(now + 10), code);
        return true;
    }
}

function assetProgress(loaded, total) {
    $('banner-text').textContent = 'Loading... (' + loaded + '/' + total + ')';
    $('banner-bar').style.width = Math.floor(loaded / total * 100) + '%';
};

function postInit() {
    $('banner-text').textContent = 'Connecting to server...';
    conn = new Connection('ws://' + window.location.host + '/ws');
    conn.onOpen = connOpen;
    conn.onInit = handleInit;
    conn.onTerrainChunk = handleTerrainChunk;
    conn.onEntityUpdate = handleEntityUpdate;
    conn.onUnloadChunk = handleUnloadChunk;

    renderer.initGl(assets);
}

function connOpen() {
    timing = new Timing(conn);
    conn.sendLogin([1, 2, 3, 4], "Pony");

    pony_sheet = new Sheet(bakeSpriteSheet(runner, assets), 96, 96);
    runner.job('load', function() {
        renderer.refreshTexture(pony_sheet.image);
    });

    document.body.removeChild($('banner-bg'));
    canvas.start();
}

function handleInit(entity_id, camera_x, camera_y, chunks, entities) {
    player_entity = entity_id;
}

function handleTerrainChunk(i, data) {
    var chunk = chunks[i];
    var raw_length = rle16Decode(data, chunk._tiles);

    if (raw_length != CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) {
        console.assert(false,
                'chunk data contained wrong number of tiles:', raw_length);
    }

    runner.job('load-chunk-' + i, function() {
        physics.loadChunk(0, i, chunk._tiles);
        renderer.loadChunk(0, i, chunk);
    });

    chunkLoaded[i] = true;
}

function handleEntityUpdate(id, motion, anim) {
    var offset = new Vec(16, 32, 0);
    var m = new Motion(motion.start_pos.add(offset));
    m.end_pos = motion.end_pos.add(offset);

    var now = Date.now();
    m.start_time = timing.decodeRecv(motion.start_time, now);
    m.end_time = timing.decodeRecv(motion.end_time, now);
    if (m.end_time < m.start_time) {
        m.end_time += 0x10000;
    }

    if (entities[id] == null) {
        entities[id] = new Entity(pony_sheet, pony_anims, motion.start_pos, {x: 48, y: 90});
    }
    entities[id].setMotion(m);
    entities[id].setAnimation(m.start_time, anim);
}

function handleUnloadChunk(idx) {
    chunkLoaded[idx] = false;
}

function localSprite(now, entity, camera_mid) {
    var local_px = CHUNK_SIZE * TILE_SIZE * LOCAL_SIZE;
    if (camera_mid == null) {
        camera_mid = new Vec(local_px, local_px, 0);
    }
    var min = camera_mid.subScalar((local_px / 2)|0);
    var max = camera_mid.addScalar((local_px / 2)|0);

    var sprite = entity.getSprite(now);

    var adjusted = false;

    if (sprite.ref_x < min.x) {
        entity.translateMotion(new Vec(local_px, 0, 0));
        adjusted = true;
    } else if (sprite.ref_x >= max.x) {
        entity.translateMotion(new Vec(-local_px, 0, 0));
        adjusted = true;
    }

    if (sprite.ref_y < min.y) {
        entity.translateMotion(new Vec(0, local_px, 0));
        adjusted = true;
    } else if (sprite.ref_y >= max.y) {
        entity.translateMotion(new Vec(0, -local_px, 0));
        adjusted = true;
    }

    if (adjusted) {
        sprite = entity.getSprite(now);
    }
    return sprite;
}


function frame(gl, now) {
    debug.frameStart();

    gl.viewport(0, 0, gl.canvas.width, gl.canvas.height);
    gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);


    var pos = new Vec(4096, 4096, 0);
    var pony = null;
    if (player_entity >= 0 && entities[player_entity] != null) {
        pos = entities[player_entity].position(now);
        pony = entities[player_entity];
    }
    debug.updatePos(pos);

    var camera_size = new Vec(gl.canvas.width|0, gl.canvas.height|0, 0);
    var camera_pos = pos.sub(camera_size.divScalar(2));


    var entity_ids = Object.getOwnPropertyNames(entities);
    var sprites = new Array(entity_ids.length);
    for (var i = 0; i < entity_ids.length; ++i) {
        var entity = entities[entity_ids[i]];
        sprites[i] = localSprite(now, entity, pos);
    }

    renderer.render(gl,
            camera_pos.x, camera_pos.y,
            gl.canvas.width, gl.canvas.height,
            sprites);

    debug.frameEnd();
    debug.updateJobs(runner);

    /*
    debug.gfxCtx.clearRect(0, 0, 128, 128);
    var chunk_pos = pos.divScalar(CHUNK_SIZE * TILE_SIZE).modScalar(LOCAL_SIZE);
    var px = 128 / LOCAL_SIZE;
    for (var y = 0; y < LOCAL_SIZE; ++y) {
        for (var x = 0; x < LOCAL_SIZE; ++x) {
            if (x == chunk_pos.x && y == chunk_pos.y) {
                debug.gfxCtx.fillStyle = 'green';
            } else {
                debug.gfxCtx.fillStyle = 'red';
            }

            if (chunkLoaded[y * LOCAL_SIZE + x]) {
                debug.gfxCtx.fillRect(x * px, y * px, px, px);
            }
        }
    }
    */
}
