var $ = document.getElementById.bind(document);


var AssetLoader = require('loader').AssetLoader;
var BackgroundJobRunner = require('util/jobs').BackgroundJobRunner;
var Vec = require('util/vec').Vec;
var DebugMonitor = require('debug').DebugMonitor;
var Config = require('config').Config;

var AnimCanvas = require('graphics/canvas').AnimCanvas;
var OffscreenContext = require('graphics/canvas').OffscreenContext;
var Sheet = require('graphics/sheet').Sheet;
var LayeredTintedSheet = require('graphics/sheet').LayeredTintedSheet;
var Animation = require('graphics/sheet').Animation;

var Entity = require('entity').Entity;
var Motion = require('entity').Motion;

var InventoryTracker = require('inventory').InventoryTracker;

var Keyboard = require('keyboard').Keyboard;
var Dialog = require('ui/dialog').Dialog;
var Banner = require('ui/banner').Banner;
var ChatWindow = require('ui/chat').ChatWindow;
var InventoryUI = require('ui/inventory').InventoryUI;
var ContainerUI = require('ui/inventory').ContainerUI;
var CraftingUI = require('ui/crafting').CraftingUI;

var TileDef = require('data/chunk').TileDef;
var ItemDef = require('data/items').ItemDef;
var RecipeDef = require('data/recipes').RecipeDef;

var Chunk = require('data/chunk').Chunk;
var CHUNK_SIZE = require('data/chunk').CHUNK_SIZE;
var TILE_SIZE = require('data/chunk').TILE_SIZE;
var LOCAL_SIZE = require('data/chunk').LOCAL_SIZE;

var Renderer = require('graphics/renderer').Renderer;
var Physics = require('physics').Physics;
var Forecast = require('physics').Forecast;

var Connection = require('net').Connection;
var Timing = require('time').Timing;

var rle16Decode = require('util/misc').rle16Decode;
var buildArray = require('util/misc').buildArray;
var checkBrowser = require('util/browser').checkBrowser;
var util = require('util/misc');


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


var canvas;
var debug;
var dialog;
var banner;
var keyboard;
var chat;

var runner;
var loader;
var assets;


var entities;
var entity_appearance;
var player_entity;

var chunks;
var chunkLoaded;
var physics;

var pony_sheet;
var renderer = null;

var conn;
var timing;
var load_counter;
var inv_tracker;

var current_item;

// Top-level initialization function

function init() {
    canvas = new AnimCanvas(frame, 'webgl');
    debug = new DebugMonitor();
    banner = new Banner();
    keyboard = new Keyboard();
    dialog = new Dialog(keyboard);
    chat = new ChatWindow();

    runner = new BackgroundJobRunner();
    loader = new AssetLoader();
    assets = loader.assets;

    entities = {};
    entity_appearance = {};
    player_entity = -1;

    chunks = buildArray(LOCAL_SIZE * LOCAL_SIZE, function() { return new Chunk(); });
    chunkLoaded = buildArray(LOCAL_SIZE * LOCAL_SIZE, function() { return false; });
    physics = new Physics();

    pony_sheet = null;  // Initialized after assets are loaded.
    renderer = new Renderer(canvas.ctx);

    conn = null;    // Initialized after assets are loaded.
    timing = null;  // Initialized after connection is opened.
    load_counter = new LoadCounter(banner, keyboard);

    current_item = -1;


    buildUI();

    checkBrowser(dialog, function() {
        loadAssets(function() {
            renderer.initGl(assets);
            pony_sheet = buildPonySheet();
            runner.job('preload-textures', preloadTextures);

            openConn(assets['server_info']['name'], function() {
                timing = new Timing(conn);
                timing.scheduleUpdates(5, 30);
                inv_tracker = new InventoryTracker(conn);

                initLoginName();
                conn.sendLogin([1, 2, 3, 4], Config.login_name.get());

                banner.hide();
                canvas.start();
            });
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

    loader.addJson(null, 'items.json', function(json) {
        var items = json['items'];
        for (var i = 0; i < items.length; ++i) {
            ItemDef.register(i, items[i]);
        }
    });

    loader.addJson(null, 'recipes.json', function(json) {
        var recipes = json['recipes'];
        for (var i = 0; i < recipes.length; ++i) {
            RecipeDef.register(i, recipes[i]);
        }
    });

    loader.addJson('server_info', 'server.json');

    loader.addText('terrain.frag', 'assets/shaders/terrain.frag');
    loader.addText('terrain.vert', 'assets/shaders/terrain.vert');

    loader.addText('sprite.frag', 'assets/shaders/sprite.frag');
    loader.addText('sprite.vert', 'assets/shaders/sprite.vert');

    loader.addText('sprite_layered.frag', 'assets/shaders/sprite_layered.frag');
}

function openConn(url, next) {
    banner.update('Connecting to server...', 0);
    conn = new Connection(url);
    conn.onOpen = next;
    conn.onClose = handleClose;
    conn.onInit = handleInit;
    conn.onTerrainChunk = handleTerrainChunk;
    conn.onEntityUpdate = handleEntityUpdate;
    conn.onUnloadChunk = handleUnloadChunk;
    conn.onOpenDialog = handleOpenDialog;
    conn.onOpenCrafting = handleOpenCrafting;
    conn.onChatUpdate = handleChatUpdate;
    conn.onEntityAppear = handleEntityAppear;
    conn.onEntityGone = handleEntityGone;
}


// Initialization helpers

function buildUI() {
    keyboard.attach(document);
    setupKeyHandler();

    document.body.appendChild(canvas.canvas);
    document.body.appendChild($('key-list'));
    document.body.appendChild($('item-box'));
    document.body.appendChild(chat.container);
    document.body.appendChild(banner.container);
    document.body.appendChild(dialog.container);
    document.body.appendChild(debug.container);

    if (!Config.show_controls.get()) {
        $('key-list').classList.add('hidden');
    }

    if (!Config.debug_show_panel.get()) {
        debug.container.classList.add('hidden');
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
var ACTION_USE_ITEM =   3;

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
            return true;
        }

        var binding = Config.keybindings.get()[evt.keyCode];
        if (binding == null) {
            return false;
        }

        if (dirs_held.hasOwnProperty(binding)) {
            dirs_held[binding] = down;
            updateWalkDir();
        } else if (down) {
            if (binding == 'show_controls') {
                var show = Config.show_controls.toggle();
                $('key-list').classList.toggle('hidden', !show);
            } else if (binding == 'debug_show_panel') {
                var show = Config.debug_show_panel.toggle();
                debug.container.classList.toggle('hidden', !show);
            } else if (binding == 'chat') {
                chat.startTyping(keyboard, conn);
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
        conn.sendInput(timing.encodeSend(now), bits);
    }

    function sendActionForKey(action) {
        var code = 0;
        var arg = 0;
        switch (action) {
            case 'interact': code = ACTION_USE; break;
            case 'inventory': code = ACTION_INVENTORY; break;
            case 'use_item':
                code = ACTION_USE_ITEM;
                arg = current_item;
                break;
            default: return false;
        }

        var now = Date.now();
        conn.sendAction(timing.encodeSend(now), code, arg);
        return true;
    }
}

function initLoginName() {
    if (Config.login_name.get() != null) {
        return;
    }

    var type = "EPU"[Math.floor(Math.random() * 3)];
    var color_r = Math.floor(Math.random() * 3) + 1;
    var color_g = Math.floor(Math.random() * 3) + 1;
    var color_b = Math.floor(Math.random() * 3) + 1;
    var extra = '' + Math.floor(Math.random() * 1000);
    while (extra.length < 3) {
        extra = '0' + extra;
    }

    var name = "Anon" + type + color_r + color_g + color_b + extra;
    Config.login_name.set(name);
}


// Connection message callbacks

function handleClose(evt, reason) {
    var reason_elt = document.createElement('p');
    if (reason != null) {
        reason_elt.textContent = 'Reason: ' + reason;
    }
    dialog.show({
        container: util.fromTemplate("disconnected", {'reason': reason_elt}),
        handleOpen: function(d) {
            d.keyboard.pushHandler(function() { return false; });
        },
    });
}

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
    if (entities[id] == null) {
        return;
    }

    var offset = new Vec(16, 32, 0);
    var m = new Motion(motion.start_pos.add(offset));
    m.end_pos = motion.end_pos.add(offset);

    var now = Date.now();
    m.start_time = timing.decodeRecv(motion.start_time, now);
    m.end_time = timing.decodeRecv(motion.end_time, now);
    if (m.end_time < m.start_time) {
        m.end_time += 0x10000;
    }

    m.anim_id = anim;

    entities[id].queueMotion(m);

    load_counter.update(0, 1);
}

function handleUnloadChunk(idx) {
    chunkLoaded[idx] = false;
}

function handleOpenDialog(idx, args) {
    if (idx == 0) {
        var inv = inv_tracker.subscribe(args[0]);
        var ui = new InventoryUI(inv);
        dialog.show(ui);

        ui.enableSelect(current_item, function(new_id) {
            current_item = new_id;
            if (new_id == -1) {
                $('item-box').firstElementChild.style.backgroundPosition = '0rem 0rem';
            } else {
                var info = ItemDef.by_id[new_id];
                $('item-box').firstElementChild.style.backgroundPosition =
                    '-' + info.tile_x + 'rem -' + info.tile_y + 'rem';
            }
        });

        ui.onclose = function() {
            inv.unsubscribe();
        };
    } else if (idx == 1) {
        var inv1 = inv_tracker.subscribe(args[0]);
        var inv2 = inv_tracker.subscribe(args[1]);

        var ui = new ContainerUI(inv1, inv2);
        dialog.show(ui);
        ui.ontransfer = function(from_inventory, to_inventory, item_id, amount) {
            conn.sendMoveItem(from_inventory, to_inventory, item_id, amount);
        };

        ui.onclose = function() {
            inv1.unsubscribe();
            inv2.unsubscribe();
        };
    }
}

function handleOpenCrafting(station_type, station_id, inventory_id) {
    var inv = inv_tracker.subscribe(inventory_id);

    var ui = new CraftingUI(station_type, station_id, inv);
    dialog.show(ui);

    ui.onaction = function(station_id, inventory_id, recipe_id, count) {
        conn.sendCraftRecipe(station_id, inventory_id, recipe_id, count);
    };

    ui.onclose = function() {
        inv.unsubscribe();
    };
}

function handleChatUpdate(msg) {
    chat.addMessage(msg);
}

function handleEntityAppear(id, appearance) {
    console.log('appear: ', id);
    entities[id] = new Entity(pony_sheet, pony_anims, new Vec(0, 0, 0), {x: 48, y: 90});
}

function handleEntityGone(id, time) {
    console.log('gone: ', id);
    // TODO: actually delay until the specified time
    delete entities[id];
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
        debug.updateMotions(pony);
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
    debug.updateTiming(timing);

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
