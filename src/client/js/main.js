var $ = document.getElementById.bind(document);


var loader = require('loader');
var BackgroundJobRunner = require('util/jobs').BackgroundJobRunner;
var Vec = require('util/vec').Vec;
var DebugMonitor = require('debug').DebugMonitor;
var Config = require('config').Config;
var TimeVarying = require('util/timevarying').TimeVarying;

var AnimCanvas = require('graphics/canvas').AnimCanvas;
var OffscreenContext = require('graphics/canvas').OffscreenContext;
var PonyAppearance = require('graphics/appearance/pony').PonyAppearance;    
var Renderer = require('graphics/renderer').Renderer;
var Cursor = require('graphics/cursor').Cursor;
var glutil = require('graphics/glutil');
var Scene = require('graphics/scene').Scene;
var DayNight = require('graphics/daynight').DayNight;
var handleResize = require('resize').handleResize;

var Entity = require('entity').Entity;
var Motion = require('entity').Motion;
var Animation = require('entity').Animation;
var Structure = require('structure').Structure;

var InventoryTracker = require('inventory').InventoryTracker;

var Keyboard = require('keyboard').Keyboard;
var Dialog = require('ui/dialog').Dialog;
var Banner = require('ui/banner').Banner;
var ChatWindow = require('ui/chat').ChatWindow;
var InventoryUI = require('ui/inventory').InventoryUI;
var ContainerUI = require('ui/inventory').ContainerUI;
var CraftingUI = require('ui/crafting').CraftingUI;
var Iframe = require('ui/iframe').Iframe;
var KeyDisplay = require('ui/keydisplay').KeyDisplay;
var Menu = require('ui/menu').Menu;
var ConfigEditor = require('ui/configedit').ConfigEditor;
var MusicTest = require('ui/musictest').MusicTest;
var PonyEditor = require('ui/ponyedit').PonyEditor;
var KeybindingEditor = require('ui/keybinding').KeybindingEditor;
var widget = require('ui/widget');
var ErrorList = require('ui/errorlist').ErrorList;
var InventoryUpdateList = require('ui/invupdate').InventoryUpdateList;
var Hotbar = require('ui/hotbar').Hotbar;
var DIALOG_TYPES = require('ui/dialogs').DIALOG_TYPES;
var DNDState = require('ui/dnd').DNDState;

var BlockDef = require('data/chunk').BlockDef;
var ItemDef = require('data/items').ItemDef;
var RecipeDef = require('data/recipes').RecipeDef;
var TemplateDef = require('data/templates').TemplateDef;
var AnimationDef = require('data/animations').AnimationDef;
var AttachSlotDef = require('data/attachments').AttachSlotDef;
var ExtraDefs = require('data/extras').ExtraDefs;

var Chunk = require('data/chunk').Chunk;
var CHUNK_SIZE = require('data/chunk').CHUNK_SIZE;
var TILE_SIZE = require('data/chunk').TILE_SIZE;
var LOCAL_SIZE = require('data/chunk').LOCAL_SIZE;

var Physics = require('physics').Physics;
var Prediction = require('physics').Prediction;
var DummyPrediction = require('physics').DummyPrediction;

var net = require('net');
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


var canvas;
var ui_div;
var debug;
var dialog;
var banner;
var keyboard;
var chat;
var error_list;
var inv_update_list;
var music_test;
var hotbar;

var main_menu;
var debug_menu;

var runner;
var assets;


var entities;
var player_entity;
var structures;

var chunks;
var chunkLoaded;
var physics;
var prediction;

var renderer = null;
var cursor;
var show_cursor = false;
var slice_radius;
var day_night;
var synced = net.SYNC_LOADING;

var conn;
var timing;
var inv_tracker;

var item_inv;
var ability_inv;


// Top-level initialization function

function init() {
    // Set up error_list first to catch errors in other parts of init.
    error_list = new ErrorList();
    error_list.attach(window);
    document.body.appendChild(error_list.container);

    canvas = new AnimCanvas(frame, 'webgl', [
            'WEBGL_depth_texture',
            'WEBGL_draw_buffers',
    ]);

    if (!glutil.hasExtension(canvas.ctx, 'WEBGL_depth_texture')) {
        throw 'missing extension: WEBGL_depth_texture';
    }
    if (!glutil.hasExtension(canvas.ctx, 'WEBGL_draw_buffers')) {
        console.warn('missing optional extension: WEBGL_draw_buffers - ' +
                'rendering in fallback mode');
    }

    ui_div = util.element('div', ['ui-container']);
    debug = new DebugMonitor();
    banner = new Banner();
    keyboard = new Keyboard();
    window.DND = new DNDState(keyboard);
    dialog = new Dialog(keyboard);
    chat = new ChatWindow();
    inv_update_list = new InventoryUpdateList();
    music_test = new MusicTest();
    hotbar = new Hotbar();

    canvas.canvas.addEventListener('webglcontextlost', function(evt) {
        throw 'context lost!';
    });

    initMenus();

    runner = new BackgroundJobRunner();
    assets = null;

    entities = {};
    player_entity = -1;
    structures = {};

    chunks = buildArray(LOCAL_SIZE * LOCAL_SIZE, function() { return new Chunk(); });
    chunkLoaded = buildArray(LOCAL_SIZE * LOCAL_SIZE, function() { return false; });
    physics = new Physics();
    prediction = Config.motion_prediction.get() ? new Prediction(physics) : new DummyPrediction();

    renderer = new Renderer(canvas.ctx);
    cursor = null;
    show_cursor = false;
    slice_radius = new TimeVarying(0, 0, 0, 0.9, 0);
    day_night = null;

    conn = null;    // Initialized after assets are loaded.
    timing = null;  // Initialized after connection is opened.

    item_inv = null;
    ability_inv = null;


    buildUI();

    checkBrowser(dialog, function() {
        loadAssets(function() {
            renderer.initGl(assets);
            runner.job('preload-textures', preloadTextures);

            cursor = new Cursor(canvas.ctx, assets, TILE_SIZE / 2 + 1);
            day_night = new DayNight(assets);

            hotbar.init();

            var info = assets['server_info'];
            openConn(info, function() {
                timing = new Timing(conn);
                timing.scheduleUpdates(5, 30);
                inv_tracker = new InventoryTracker(conn);

                maybeRegister(info, function() {
                    conn.sendLogin(Config.login_name.get(), Config.login_secret.get());

                    // Show "Loading World..." banner.
                    handleSyncStatus(net.SYNC_LOADING);
                    canvas.start();
                });
            });
        });
    });
}

document.addEventListener('DOMContentLoaded', init);


// Major initialization steps.

function loadAssets(next) {
    loader.loadJson('server.json', function(server_info) {
        // TODO: remove this hack since it prevents all caching
        loader.loadPack('outpost.pack?' + Date.now(), function(loaded, total) {
            banner.update('Loading... (' + (loaded >> 10)+ 'k / ' + (total >> 10) + 'k)', loaded / total);
        }, function(assets_) {
            assets = assets_;
            assets['server_info'] = server_info;

            var blocks = assets['block_defs'];
            for (var i = 0; i < blocks.length; ++i) {
                BlockDef.register(i, blocks[i]);
            }

            var items = assets['item_defs'];
            for (var i = 0; i < items.length; ++i) {
                ItemDef.register(i, items[i]);
            }

            var recipes = assets['recipe_defs'];
            for (var i = 0; i < recipes.length; ++i) {
                RecipeDef.register(i, recipes[i]);
            }

            var templates = assets['template_defs'];
            for (var i = 0; i < templates.length; ++i) {
                TemplateDef.register(i, templates[i], assets);
            }

            var animations = assets['animation_defs'];
            for (var i = 0; i < animations.length; ++i) {
                AnimationDef.register(i, animations[i]);
            }

            var attach_slots = assets['attach_slot_defs'];
            for (var i = 0; i < attach_slots.length; ++i) {
                AttachSlotDef.register(i, attach_slots[i]);
            }

            ExtraDefs.init(assets['extra_defs']);

            renderer.initData(BlockDef.by_id, TemplateDef.by_id, assets['model_defs']);

            var css = '.item-icon {' +
                'background-image: url("' + assets['items'] + '");' +
            '}';
            util.element('style', ['type=text/css', 'text=' + css], document.head);

            next();
        });
    });
}

function openConn(info, next) {
    var url = info['url'];
    if (url == null) {
        var elt = util.element('div', []);
        elt.innerHTML = info['message'];
        var w = new widget.Template('server-offline', {'msg': elt});
        var f = new widget.Form(w);
        f.oncancel = function() {};
        dialog.show(f);
        return;
    }

    banner.update('Connecting to server...', 0);
    conn = new net.Connection(url);
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
    conn.onStructureAppear = handleStructureAppear;
    conn.onStructureGone = handleStructureGone;
    conn.onMainInventory = handleMainInventory;
    conn.onAbilityInventory = handleAbilityInventory;
    conn.onPlaneFlags = handlePlaneFlags;
    conn.onGetInteractArgs = handleGetInteractArgs;
    conn.onGetUseItemArgs = handleGetUseItemArgs;
    conn.onGetUseAbilityArgs = handleGetUseAbilityArgs;
    conn.onSyncStatus = handleSyncStatus;
    conn.onStructureReplace = handleStructureReplace;
}

function maybeRegister(info, next) {
    if (Config.login_name.isSet() && Config.login_secret.isSet() &&
            Config.world_version.get() == info['world_version']) {
        console.log('secret already set');
        next();
        return;
    }

    var default_name = Config.login_name.get() || generateName();
    var secret = makeSecret();

    var editor = new PonyEditor(default_name, drawPony);

    var last_name = null;

    function send_register(name, app_info) {
        editor.onfinish = null;
        editor.setMessage("Registering...");
        last_name = name;

        var appearance = calcAppearance(app_info);
        saveAppearance(app_info);
        conn.onRegisterResult = handle_result;
        conn.sendRegister(name,
                          secret,
                          appearance);
    }

    function handle_result(code, msg) {
        conn.onRegisterResult = null;
        if (code == 0) {
            Config.login_name.set(last_name);
            Config.login_secret.set(secret);
            Config.world_version.set(info['world_version']);
            dialog.hide();
            next();
        } else {
            editor.setError(code, msg);
            editor.onfinish = send_register;
        }
    }

    editor.onsubmit = send_register;
    editor.oncancel = function() {};
    dialog.show(editor);
}


// Initialization helpers

function buildUI() {
    keyboard.attach(document);
    setupKeyHandler();

    function doResize() {
        handleResize(canvas, ui_div, window.innerWidth, window.innerHeight);
    }
    window.addEventListener('resize', doResize);
    doResize();

    var key_list = $('key-list');

    ui_div.appendChild(key_list);
    ui_div.appendChild(hotbar.dom);
    ui_div.appendChild(chat.container);
    ui_div.appendChild(inv_update_list.container);
    ui_div.appendChild(banner.container);
    ui_div.appendChild(dialog.container);
    ui_div.appendChild(debug.container);

    if (Config.show_key_display.get()) {
        var key_display = new KeyDisplay();
        ui_div.appendChild(key_display.container);
        keyboard.monitor = function(down, evt) {
            if (down) {
                key_display.onKeyDown(evt);
            } else {
                key_display.onKeyUp(evt);
            }
        };
    }

    if (!Config.show_controls.get()) {
        key_list.classList.add('hidden');
    }

    if (!Config.debug_show_panel.get()) {
        debug.container.classList.add('hidden');
    }

    banner.show('Loading...', 0, keyboard, function() { return false; });

    document.body.appendChild(canvas.canvas);
    document.body.appendChild(ui_div);
}

function initMenus() {
    main_menu = new Menu([
            ['&Instructions', function() {
                dialog.show(new widget.Form(new Iframe('instructions.html', keyboard)));
            }],
            ['&Keyboard Controls', function() {
                dialog.show(new KeybindingEditor(keyboard));
            }],
            ['&Debug Menu', function() { dialog.show(debug_menu); }],
            ['&Credits', function() {
                dialog.show(new widget.Form(new Iframe('credits.html', keyboard)));
            }],
    ]);

    debug_menu = new Menu([
            ['&Config Editor', function() { dialog.show(new ConfigEditor()); }],
            ['&Music Test', function() { dialog.show(music_test); }],
    ]);
}

function generateName() {
    var number = '' + Math.floor(Math.random() * 10000);
    while (number.length < 4) {
        number = '0' + number;
    }

    return "Anon" + number;
}

function makeSecret() {
    console.log('producing secret');
    var secret_buf = [0, 0, 0, 0];
    if (window.crypto.getRandomValues) {
        var typedBuf = new Uint32Array(4);
        window.crypto.getRandomValues(typedBuf);
        for (var i = 0; i < 4; ++i) {
            secret_buf[i] = typedBuf[i];
        }
    } else {
        console.log("warning: window.crypto.getRandomValues is not available.  " +
                "Login secret will be weak!");
        for (var i = 0; i < 4; ++i) {
            secret_buf[i] = Math.floor(Math.random() * 0xffffffff);
        }
    }
    return secret_buf;
}

function calcAppearance(a) {
    var appearance =
        (a.eyes << 16) |
        (a.tail << 13) |
        (a.mane << 10) |
        (a.sex << 8) |
        (a.tribe << 6) |
        (a.red << 4) |
        (a.green << 2) |
        (a.blue << 0);

    return appearance;
}

function saveAppearance(a) {
    Config.last_appearance.set({
        'eyes': a.eyes,
        'tail': a.tail,
        'mane': a.mane,
        'sex': a.sex,
        'tribe': a.tribe,
        'red': a.red,
        'green': a.green,
        'blue': a.blue,
    });
}

function drawPony(ctx, app_info) {
    var bits = calcAppearance(app_info);
    var app = new PonyAppearance(assets, bits, '');
    var anim_def = AnimationDef.by_id[ExtraDefs.editor_anim];
    var frame = new Animation(anim_def, 0).frameInfo(0);
    var sprite = app.buildSprite(new Vec(0, 0, 0), frame);
    sprite.setRefPosition(sprite.anchor_x, sprite.anchor_y, 0);

    ctx.clearRect(0, 0, 96, 96);
    app.draw2D(ctx, [0, 0], sprite);
}

function preloadTextures() {
    var textures = ['tiles',
                    'pony_f_base_E-0',
                    'pony_f_base_P-0',
                    'pony_f_base_U-0',
                    'pony_f_eyes_0-0',
                    'pony_f_mane_0-0',
                    'pony_f_tail_0-0'];
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
        if (down && evt.repeat) {
            return true;
        }

        var shouldStop = alwaysStop(evt);

        var binding = Config.keybindings.get()[evt.keyCode];
        if (binding == null || evt.ctrlKey || evt.altKey || evt.metaKey) {
            return shouldStop;
        }

        if (dirs_held.hasOwnProperty(binding)) {
            dirs_held[binding] = down;
            updateWalkDir();
            return true;
        } else if (down) {
            var time = timing.encodeSend(timing.nextArrival());

            switch (binding) {
                // UI actions
                case 'show_controls':
                    var show = Config.show_controls.toggle();
                    $('key-list').classList.toggle('hidden', !show);
                    break;
                case 'debug_show_panel':
                    var show = Config.debug_show_panel.toggle();
                    debug.container.classList.toggle('hidden', !show);
                    break;
                case 'debug_test':
                    dialog.show(new PonyEditor(Config.login_name.get()));
                    break;
                case 'chat':
                    chat.startTyping(keyboard, conn, '');
                    break;
                case 'chat_command':
                    chat.startTyping(keyboard, conn, '/');
                    break;
                case 'show_menu':
                    dialog.show(main_menu);
                    break;
                case 'toggle_cursor':
                    show_cursor = !show_cursor;
                    break;

                case 'inventory':
                    if (item_inv == null) {
                        break;
                    }
                    var inv = item_inv.clone();
                    var ui = new InventoryUI(inv);
                    dialog.show(ui);

                    ui.enableSelect(hotbar.getItem(), function(idx, new_id) {
                        hotbar.setSlot(idx, new_id, true);
                        hotbar.selectSlot(idx);
                    });

                    ui.oncancel = function() {
                        dialog.hide();
                        inv.release();
                    };
                    break;

                case 'abilities':
                    if (ability_inv == null) {
                        break;
                    }
                    var inv = ability_inv.clone();
                    var ui = new InventoryUI(inv, 'Abilities');
                    dialog.show(ui);

                    ui.enableSelect(hotbar.getAbility(), function(idx, new_id) {
                        hotbar.setSlot(idx, new_id,  false);
                        hotbar.selectSlot(idx);
                    });

                    ui.oncancel = function() {
                        dialog.hide();
                        inv.release();
                    };
                    break;

                // Commands to the server
                case 'interact':
                    conn.sendInteract(time);
                    break;
                case 'use_item':
                    conn.sendUseItem(time, hotbar.getItem());
                    break;
                case 'use_ability':
                    conn.sendUseAbility(time, hotbar.getAbility());
                    break;

                default:
                    if (binding != null && binding.startsWith('hotbar_')) {
                        var idx = +binding.substring(7) - 1;
                        hotbar.selectSlot(idx);
                        break;
                    } else {
                        return shouldStop;
                    }
            }

            return true;
        } else {
            return shouldStop;
        }
    });

    function updateWalkDir() {
        var bits = 0;
        var target_velocity = new Vec(0, 0, 0);

        if (dirs_held['move_left']) {
            bits |= INPUT_LEFT;
            target_velocity.x -= 1;
        }
        if (dirs_held['move_right']) {
            bits |= INPUT_RIGHT;
            target_velocity.x += 1;
        }

        if (dirs_held['move_up']) {
            bits |= INPUT_UP;
            target_velocity.y -= 1;
        }
        if (dirs_held['move_down']) {
            bits |= INPUT_DOWN;
            target_velocity.y += 1;
        }

        if (dirs_held['run']) {
            bits |= INPUT_RUN;
            target_velocity = target_velocity.mulScalar(150);
        } else {
            target_velocity = target_velocity.mulScalar(50);
        }

        var arrival = timing.nextArrival() + Config.input_delay.get();
        conn.sendInput(timing.encodeSend(arrival), bits);

        if (player_entity != null && entities[player_entity] != null) {
            var pony = entities[player_entity];
            prediction.predict(arrival, pony, target_velocity);
        }
    }

    function alwaysStop(evt) {
        // Allow Ctrl + anything
        if (evt.ctrlKey) {
            return false;
        }
        // Allow F5-F12
        if (evt.keyCode >= 111 + 5 && evt.keyCode <= 111 + 12) {
            return false;
        }

        // Stop all other events.
        return true;
    }
}


// Connection message callbacks

function handleClose(evt, reason) {
    var reason_elt = document.createElement('p');
    if (reason != null) {
        reason_elt.textContent = 'Reason: ' + reason;
    }

    var w = new widget.Template('disconnected', {'reason': reason_elt});
    var f = new widget.Form(w);
    f.oncancel = function() {};
    dialog.show(f);
}

function handleInit(entity_id, now, cycle_base, cycle_ms) {
    player_entity = entity_id;
    var pst_now = timing.decodeRecv(now);
    day_night.base_time = pst_now - cycle_base;
    day_night.cycle_ms = cycle_ms;
}

function handleTerrainChunk(i, data) {
    var chunk = chunks[i];
    var raw_length = rle16Decode(data, chunk._tiles);

    if (raw_length != CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) {
        console.assert(false,
                'chunk data contained wrong number of tiles:', raw_length);
    }

    runner.job('load-chunk-' + i, function() {
        physics.loadChunk((i / LOCAL_SIZE)|0, (i % LOCAL_SIZE)|0, chunk._tiles);
        renderer.loadChunk((i / LOCAL_SIZE)|0, (i % LOCAL_SIZE)|0, chunk);
    });

    chunkLoaded[i] = true;
}

function handleEntityUpdate(id, motion, anim) {
    if (entities[id] == null) {
        return;
    }

    var m = new Motion(motion.start_pos);
    m.end_pos = motion.end_pos;

    var now = timing.visibleNow();
    m.start_time = timing.decodeRecv(motion.start_time);
    m.end_time = timing.decodeRecv(motion.end_time);
    if (m.start_time > now + 2000) {
        m.start_time -= 0x10000;
    }
    if (m.end_time < m.start_time) {
        m.end_time += 0x10000;
    }

    m.anim_id = anim;

    if (id != player_entity) {
        entities[id].queueMotion(m);
    } else {
        prediction.receivedMotion(m, entities[id]);
    }
}

function handleUnloadChunk(idx) {
    chunkLoaded[idx] = false;
}

function handleOpenDialog(idx, args) {
    if (idx == 0) {
        // Cancel server-side subscription.
        inv_tracker.unsubscribe(args[0]);
    } else if (idx == 1) {
        var inv1 = inv_tracker.get(args[0]);
        var inv2 = inv_tracker.get(args[1]);

        var ui = new ContainerUI(inv1, inv2);
        dialog.show(ui);
        ui.ontransfer = function(from_inventory, to_inventory, item_id, amount) {
            conn.sendMoveItem(from_inventory, to_inventory, item_id, amount);
        };

        ui.oncancel = function() {
            dialog.hide();
            inv1.unsubscribe();
            inv2.unsubscribe();
        };
    }
}

function handleOpenCrafting(station_type, station_id, inventory_id) {
    var inv = inv_tracker.get(inventory_id);

    var ui = new CraftingUI(station_type, station_id, inv);
    dialog.show(ui);

    ui.onaction = function(station_id, inventory_id, recipe_id, count) {
        conn.sendCraftRecipe(station_id, inventory_id, recipe_id, count);
    };

    ui.oncancel = function() {
        dialog.hide();
        inv.unsubscribe();
    };
}

function handleChatUpdate(msg) {
    chat.addMessage(msg);
}

function handleEntityAppear(id, appearance_bits, name) {
    if (id == player_entity) {
        name = '';
    }
    var app = new PonyAppearance(assets, appearance_bits, name);
    if (entities[id] != null) {
        entities[id].setAppearance(app);
    } else {
        entities[id] = new Entity(
                app,
                new Animation(AnimationDef.by_id[ExtraDefs.default_anim], 0),
                new Vec(0, 0, 0));
    }

    if ((appearance_bits & (1 << 9)) != 0) {
        entities[id].setLight(200, [100, 180, 255]);
    } else {
        entities[id].setLight(0, null);
    }
}

function handleEntityGone(id, time) {
    // TODO: actually delay until the specified time
    delete entities[id];
}

function handleStructureAppear(id, template_id, x, y, z) {
    var now = timing.visibleNow();
    var template = TemplateDef.by_id[template_id];

    var idx = renderer.addStructure(now, id, x, y, z, template);

    var pos = new Vec(x, y, z).divScalar(TILE_SIZE);

    structures[id] = new Structure(pos, template, idx);
    physics.addStructure(structures[id]);
}

function handleStructureGone(id, time) {
    // TODO: pay attention to the time
    if (structures[id] != null) {
        physics.removeStructure(structures[id]);

        var new_id = renderer.removeStructure(structures[id]);
        // Structure `new_id` has been moved to the slot just vacated by `id`.
        structures[new_id].render_index = structures[id].render_index;
    }
    delete structures[id];
}

function handleStructureReplace(id, template_id) {
    if (structures[id] != null) {
        var now = timing.visibleNow();
        var pos = structures[id].pos.mulScalar(TILE_SIZE);
        handleStructureGone(id, now);
        handleStructureAppear(id, template_id, pos.x, pos.y, pos.z);
    }
}

function handleMainInventory(iid) {
    if (item_inv != null) {
        item_inv.unsubscribe();
    }
    item_inv = inv_tracker.get(iid);
    hotbar.attachItems(item_inv.clone());
    if (Config.show_inventory_updates.get()) {
        inv_update_list.attach(item_inv.clone());
    }
}

function handleAbilityInventory(iid) {
    if (ability_inv != null) {
        ability_inv.unsubscribe();
    }
    ability_inv = inv_tracker.get(iid);
    hotbar.attachAbilities(ability_inv.clone());
}

function handlePlaneFlags(flags) {
    day_night.active = (flags == 0);
}

function handleGetInteractArgs(dialog_id, parts) {
    handleGenericGetArgs(dialog_id, parts, function(time, args) {
        conn.sendInteractWithArgs(time, args);
    });
}

function handleGetUseItemArgs(item_id, dialog_id, parts) {
    handleGenericGetArgs(dialog_id, parts, function(time, args) {
        conn.sendUseItemWithArgs(time, item_id, args);
    });
}

function handleGetUseAbilityArgs(item_id, dialog_id, parts) {
    handleGenericGetArgs(dialog_id, parts, function(time, args) {
        conn.sendUseAbilityWithArgs(time, item_id, args);
    });
}

function handleGenericGetArgs(dialog_id, parts, cb) {
    var d = new (DIALOG_TYPES[dialog_id])(parts);
    d.onsubmit = function(args) {
        dialog.hide();
        var time = timing.encodeSend(timing.nextArrival());
        cb(time, args);
    };
    dialog.show(d);
}

function handleSyncStatus(new_synced) {
    synced = new_synced;
    if (synced == net.SYNC_REFRESH) {
        window.location.reload(true);
    } else if (synced == net.SYNC_LOADING) {
        banner.show('Loading World...', 0, keyboard, function() { return false; });
    } else if (synced == net.SYNC_RESET) {
        banner.show('Server restarting...', 0, keyboard, function() { return false; });

        if (synced == net.SYNC_RESET) {
            resetAll();
        }
    } else {
        banner.hide();
    }
}

// Reset (nearly) all client-side state to pre-login conditions.
function resetAll() {
    var now = timing.visibleNow();

    inv_tracker.reset();
    item_inv = null;
    ability_inv = null;

    if (dialog.isVisible()) {
        dialog.hide();
    }

    Object.getOwnPropertyNames(entities).forEach(function(id) {
        handleEntityGone(id, now);
    });
    player_entity = -1;

    Object.getOwnPropertyNames(structures).forEach(function(id) {
        handleStructureGone(id, now);
    });
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

    // TODO: hacky adjustment
    sprite.ref_x += 16;
    sprite.ref_y += 32;
    return sprite;
}

function checkLocalSprite(sprite, camera_mid) {
    var local_px = CHUNK_SIZE * TILE_SIZE * LOCAL_SIZE;
    if (camera_mid == null) {
        camera_mid = new Vec(local_px, local_px, 0);
    }
    var min = camera_mid.subScalar((local_px / 2)|0);
    var max = camera_mid.addScalar((local_px / 2)|0);

    // TODO: it's ugly to adjust the Structure object's sprite from here.

    if (sprite.ref_x < min.x) {
        sprite.ref_x += local_px;
    } else if (sprite.ref_x >= max.x) {
        sprite.ref_x -= local_px;
    }

    if (sprite.ref_y < min.y) {
        sprite.ref_y += local_px;
    } else if (sprite.ref_y >= max.y) {
        sprite.ref_y -= local_px;
    }
}

function needs_mask(now, pony) {
    if (pony == null) {
        return false;
    }

    var pos = pony.position(now);
    var ceiling = physics.findCeiling(pos);
    return (ceiling < 16);
}

var FACINGS = [
    new Vec( 1,  0,  0),
    new Vec( 1,  1,  0),
    new Vec( 0,  1,  0),
    new Vec(-1,  1,  0),
    new Vec(-1,  0,  0),
    new Vec(-1, -1,  0),
    new Vec( 0, -1,  0),
    new Vec( 1, -1,  0),
];

function frame(ac, client_now) {
    if (synced != net.SYNC_OK) {
        return;
    }

    var now = timing.visibleNow();

    // Here's the math on client-side motion prediction.
    //
    //                <<<<<<<
    //   Server ----- A --- C --------
    //                 \   / \
    //                  \ /   \
    //   Client -------- B --- D -----
    //
    // `A` is the latest visible time.  For the player entity only, we have a
    // predicted motion (starting at time C) based on the inputs we last sent
    // to the server (at time B).  We want to display that predicted motion
    // now, as if it started at time A instead of C.  To be consistent, we
    // always do this translation, drawing the player entity as we expect it to
    // appear `timing.ping` msec in the future instead of how it actually is.
    var predict_now;
    if (Config.motion_prediction.get()) {
        predict_now = now + timing.ping;
    } else {
        predict_now = now;
    }

    debug.frameStart();
    var gl = ac.ctx;

    gl.viewport(0, 0, ac.canvas.width, ac.canvas.height);
    gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);

    var pos = new Vec(4096, 4096, 0);
    var pony = null;
    if (player_entity >= 0 && entities[player_entity] != null) {
        pony = entities[player_entity];

        var motion_end = pony.motionEndTime(predict_now);
        if (motion_end <= predict_now) {
            prediction.refreshMotion(predict_now, pony);
        }

        // Make sure the camera remains within the middle of the local space.
        localSprite(predict_now, pony, null);
        // TODO: another hacky offset
        pos = pony.position(predict_now).add(new Vec(16, 16, 0));

        debug.updateMotions(pony, timing);
    }
    debug.updatePos(pos);

    var view_width = ac.virtualWidth;
    var view_height = ac.virtualHeight;

    var camera_size = new Vec(view_width, view_height, 0);
    var camera_pos = pos.sub(camera_size.divScalar(2));
    camera_pos.y -= camera_pos.z;


    var s = new Scene();
    s.now = now;


    var entity_ids = Object.getOwnPropertyNames(entities);
    s.sprites = new Array(entity_ids.length);
    var player_sprite = null;

    for (var i = 0; i < entity_ids.length; ++i) {
        var entity = entities[entity_ids[i]];
        if (entity_ids[i] != player_entity) {
            s.sprites[i] = localSprite(now, entity, pos);
        } else {
            s.sprites[i] = localSprite(predict_now, entity, pos);
            player_sprite = s.sprites[i];
        }

        var light = entity.getLight();
        if (light != null) {
            s.lights.push({
                pos: new Vec(
                    s.sprites[i].ref_x,
                    s.sprites[i].ref_y,
                    s.sprites[i].ref_z),
                color: light.color,
                radius: light.radius,
            });
        }
    }


    if (needs_mask(predict_now, pony)) {
        if (slice_radius.velocity <= 0) {
            slice_radius.setVelocity(predict_now, 2);
        }
    } else {
        if (slice_radius.velocity >= 0) {
            slice_radius.setVelocity(predict_now, -2);
        }
    }

    function draw_extra(fb_idx, r) {
        if (player_sprite != null && Config.render_outline.get()) {
            // TODO: make outlines work in new system
            //r.renderSpecial(fb_idx, player_sprite, 'pony_outline');
        }
    }

    s.camera_pos = [camera_pos.x, camera_pos.y];
    s.camera_size = [camera_size.x, camera_size.y];
    s.ambient_color = day_night.getAmbientColor(predict_now);

    var radius = slice_radius.get(predict_now);
    if (radius > 0 && pony != null) {
        s.slice_frac = radius;
        s.slice_z = 2 + (pony.position(predict_now).z / TILE_SIZE)|0;
    }
    renderer.render(s, draw_extra);


    if (show_cursor && pony != null) {
        var facing = FACINGS[pony.animId() % FACINGS.length];
        var cursor_pos = pos.divScalar(TILE_SIZE).add(facing);
        cursor_pos.y -= cursor_pos.z;
        cursor.draw(camera_pos, camera_size, cursor_pos);
    }

    debug.frameEnd();
    debug.updateJobs(runner);
    debug.updateTiming(timing);
    debug.updateGraphics(renderer);
}
