var $ = document.getElementById.bind(document);


var Vec = require('vec').Vec;
var AnimCanvas = require('canvas').AnimCanvas;
var OffscreenContext = require('canvas').OffscreenContext;
var DebugMonitor = require('debug').DebugMonitor;
var Sheet = require('sheet').Sheet;
var LayeredTintedSheet = require('sheet').LayeredTintedSheet;
var Animation = require('sheet').Animation;
var AssetLoader = require('loader').AssetLoader;
var BackgroundJobRunner = require('jobs').BackgroundJobRunner;
var Entity = require('entity').Entity;
var Motion = require('entity').Motion;

var Config = require('config').Config;

var Keyboard = require('keyboard').Keyboard;
var Dialog = require('dialog').Dialog;
var Banner = require('banner').Banner;
//var Inventory = require('inventory').Inventory;
var ItemRow = require('inventory').ItemRow;

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
var buildArray = require('util').buildArray;


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


/** @constructor */
function LoadCounter(banner, keyboard) {
    this.banner = banner;
    this.keyboard = keyboard;

    this.chunks = 0;
    this.entities = 0;
    this.total_chunks = 0;
    this.total_entities = 0;
    this.loading = false;
}

function load_cost(chunks, entities) {
    return 2 * chunks + 1 * entities;
}

LoadCounter.prototype._current = function() {
    return load_cost(this.chunks, this.entities);
}

LoadCounter.prototype._total = function() {
    return load_cost(this.total_chunks, this.total_entities);
}

LoadCounter.prototype._buildMessage = function() {
    return 'Loading World... (' + this.chunks + '/' + this.total_chunks  +
                ' + ' + this.entities + '/' + this.total_entities + ')';
};

LoadCounter.prototype.begin = function(chunks, entities) {
    if (chunks == 0 && entities == 0) {
        return;
    }

    this.chunks = 0;
    this.entities = 0;
    this.total_chunks = chunks;
    this.total_entities = entities;
    this.loading = true;

    this.banner.show(this._buildMessage(), 0, this.keyboard, function() { return false; });
};

LoadCounter.prototype.update = function(chunks, entities) {
    if (!this.loading) {
        return;
    }
    this.chunks += chunks;
    this.entities += entities;

    if (this.chunks >= this.total_chunks && this.entities >= this.total_entities) {
        this.reset();
    } else {
        this.banner.update(this._buildMessage(), this._current() / this._total());
    }
};

LoadCounter.prototype.reset = function() {
    this.loading = false;
    this.banner.hide();
};


var config;

var canvas;
var debug;
var dialog;
var banner;
var keyboard;

var runner;
var loader;
var assets;


var entities;
var player_entity;

var chunks;
var chunkLoaded;
var physics;

var pony_sheet;
var renderer = null;

var conn;
var timing;
var load_counter;

// Top-level initialization function

function init() {
    config = new Config();

    canvas = new AnimCanvas(frame, 'webgl');
    debug = new DebugMonitor();
    dialog = new Dialog();
    banner = new Banner();
    keyboard = new Keyboard();

    runner = new BackgroundJobRunner();
    loader = new AssetLoader();
    assets = loader.assets;

    entities = {};
    player_entity = -1;

    chunks = buildArray(LOCAL_SIZE * LOCAL_SIZE, function() { return new Chunk(); });
    chunkLoaded = buildArray(LOCAL_SIZE * LOCAL_SIZE, function() { return false; });
    physics = new Physics();

    pony_sheet = null;  // Initialized after assets are loaded.
    renderer = new Renderer(canvas.ctx);

    conn = null;    // Initialized after assets are loaded.
    timing = null;  // Initialized after connection is opened.
    load_counter = new LoadCounter(banner, keyboard);


    buildUI();

    loadAssets(function() {
        renderer.initGl(assets);
        pony_sheet = buildPonySheet();
        runner.job('preload-textures', preloadTextures);

        openConn(function() {
            timing = new Timing(conn);
            conn.sendLogin([1, 2, 3, 4], "Pony");
            banner.hide();
            canvas.start();
        });
    });

}

document.addEventListener('DOMContentLoaded', init);


// Major initialization steps.

function loadAssets(next) {
    loader.onprogress = function(loaded, total) {
        banner.update('Loading... (' + loaded + '/' + total + ')', loaded / total);
    };
    loader.onload = next;

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

    loader.addText('sprite_layered.frag', 'assets/shaders/sprite_layered.frag');
}

function openConn(next) {
    banner.update('Connecting to server...', 0);
    conn = new Connection('ws://' + window.location.host + '/ws');
    conn.onOpen = next;
    conn.onInit = handleInit;
    conn.onTerrainChunk = handleTerrainChunk;
    conn.onEntityUpdate = handleEntityUpdate;
    conn.onUnloadChunk = handleUnloadChunk;
}


// Initialization helpers

function buildUI() {
    keyboard.attach(document);
    setupKeyHandler();

    document.body.appendChild(canvas.canvas);
    document.body.appendChild(debug.container);
    document.body.appendChild($('key-list'));
    document.body.appendChild(dialog.container);

    if (!config.show_controls.get()) {
        $('key-list').classList.add('hidden');
    }

    banner.show('Loading...', 0, keyboard, function() { return false; });
}

function buildPonySheet() {
    return new LayeredTintedSheet([
            { image: assets['pony_f_wing_back'],    color: 0xcc88ff,    skip: false },
            { image: assets['pony_f_base'],         color: 0xcc88ff,    skip: false },
            { image: assets['pony_f_eyes_blue'],    color: 0xffffff,    skip: false },
            { image: assets['pony_f_wing_front'],   color: 0xcc88ff,    skip: false },
            { image: assets['pony_f_tail_1'],       color: 0x8844cc,    skip: false },
            { image: assets['pony_f_mane_1'],       color: 0x8844cc,    skip: false },
            { image: assets['pony_f_horn'],         color: 0xcc88ff,    skip: false },
            ], 96, 96);
}

function preloadTextures() {
    var textures = ['tiles',
                    'pony_f_wing_back',
                    'pony_f_base',
                    'pony_f_eyes_blue',
                    'pony_f_wing_front',
                    'pony_f_tail_1',
                    'pony_f_mane_1',
                    'pony_f_horn'];
    for (var i = 0; i < textures.length; ++i) {
        (function(key) {
            runner.subjob(key, function() {
                renderer.cacheTexture(assets[key]);
            });
        })(textures[i]);
    }
}


var INPUT_LEFT =    0x0001;
var INPUT_RIGHT =   0x0002;
var INPUT_UP =      0x0004;
var INPUT_DOWN =    0x0008;
var INPUT_RUN =     0x0010;

var ACTION_USE =        1;
var ACTION_INVENTORY =  2;

function setupKeyHandler() {
    var dirs_held = {
        'move_up': false,
        'move_down': false,
        'move_left': false,
        'move_right': false,
        'run': false,
    };

    keyboard.pushHandler(function(down, evt) {
        if (evt.repeat) {
            return false;
        }

        var binding = config.keybindings.get()[evt.keyCode];
        if (binding == null) {
            return false;
        }

        if (dirs_held.hasOwnProperty(binding)) {
            dirs_held[binding] = down;
            updateWalkDir();
        } else if (down) {
            if (binding == 'show_controls') {
                var show = config.show_controls.toggle();
                $('key-list').classList.toggle('hidden', !show);
            } else {
                sendActionForKey(binding);
            }
        }
            return true;
    });

    function updateWalkDir() {
        var bits = 0;

        if (dirs_held['move_left']) {
            bits |= INPUT_LEFT;
        }
        if (dirs_held['move_right']) {
            bits |= INPUT_RIGHT;
        }

        if (dirs_held['move_up']) {
            bits |= INPUT_UP;
        }
        if (dirs_held['move_down']) {
            bits |= INPUT_DOWN;
        }

        if (dirs_held['run']) {
            bits |= INPUT_RUN;
        }

        var now = Date.now();
        conn.sendInput(timing.encodeSend(now + 10), bits);
    }

    function sendActionForKey(action) {
        var code = 0;
        switch (action) {
            case 'interact': code = ACTION_USE; break;
            case 'inventory': code = ACTION_INVENTORY; break;
            default: return false;
        }

        var now = Date.now();
        conn.sendAction(timing.encodeSend(now + 10), code);
        return true;
    }
}


// Connection message callbacks

function handleInit(entity_id, camera_x, camera_y, chunks, entities) {
    player_entity = entity_id;
    load_counter.begin(chunks, entities);
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
    load_counter.update(1, 0);
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

    load_counter.update(0, 1);
}

function handleUnloadChunk(idx) {
    chunkLoaded[idx] = false;
}


// Rendering

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
