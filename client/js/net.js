var Vec = require('util/vec').Vec;
var decodeUtf8 = require('util/misc').decodeUtf8;

var OP_GET_TERRAIN =            0x0001;
var OP_UPDATE_MOTION =          0x0002;
var OP_PING =                   0x0003;
var OP_INPUT =                  0x0004;
var OP_LOGIN =                  0x0005;
var OP_ACTION =                 0x0006;
var OP_UNSUBSCRIBE_INVENTORY =  0x0007;
var OP_MOVE_ITEM =              0x0008;
var OP_CRAFT_RECIPE =           0x0009;
var OP_CHAT =                   0x000a;
var OP_REGISTER =               0x000b;
var OP_INTERACT =               0x000c;
var OP_USE_ITEM =               0x000d;
var OP_USE_ABILITY =            0x000e;
var OP_OPEN_INVENTORY =         0x000f;
var OP_INTERACT_WITH_ARGS =     0x0010;
var OP_USE_ITEM_WITH_ARGS =     0x0011;
var OP_USE_ABILITY_WITH_ARGS =  0x0012;

var OP_TERRAIN_CHUNK =          0x8001;
var OP_PLAYER_MOTION =          0x8002;
var OP_PONG =                   0x8003;
var OP_ENTITY_UPDATE =          0x8004;
var OP_INIT =                   0x8005;
var OP_KICK_REASON =            0x8006;
var OP_UNLOAD_CHUNK =           0x8007;
var OP_OPEN_DIALOG =            0x8008;
var OP_INVENTORY_UPDATE =       0x8009;
var OP_OPEN_CRAFTING =          0x800a;
var OP_CHAT_UPDATE =            0x800b;
var OP_ENTITY_APPEAR =          0x800c;
var OP_ENTITY_GONE =            0x800d;
var OP_REGISTER_RESULT =        0x800e;
var OP_STRUCTURE_APPEAR =       0x800f;
var OP_STRUCTURE_GONE =         0x8010;
var OP_MAIN_INVENTORY =         0x8011;
var OP_ABILITY_INVENTORY =      0x8012;
var OP_PLANE_FLAGS =            0x8013;
var OP_GET_INTERACT_ARGS =      0x8014;
var OP_GET_USE_ITEM_ARGS =      0x8015;
var OP_GET_USE_ABILITY_ARGS =   0x8016;
var OP_SYNC_STATUS =            0x8017;

/** @constructor */
function Connection(url) {
    var this_ = this;

    var socket = new WebSocket(url);
    socket.binaryType = 'arraybuffer';
    socket.onopen = function(evt) { this_._handleOpen(evt); };
    socket.onmessage = function(evt) { this_._handleMessage(evt); };
    socket.onclose = function(evt) { this_._handleClose(evt); };
    this.socket = socket;

    this._last_kick_reason = null;

    this.onOpen = null;
    this.onClose = null;
    this.onTerrainChunk = null;
    this.onPlayerMotion = null;
    this.onPong = null;
    this.onEntityUpdate = null;
    this.onInit = null;
    this.onUnloadChunk = null;
    this.onOpenDialog = null;
    this.onInventoryUpdate = null;
    this.onChatUpdate = null;
    this.onEntityAppear = null;
    this.onEntityGone = null;
    this.onRegisterResult = null;
    this.onStructureAppear = null;
    this.onStructureGone = null;
    this.onMainInventory = null;
    this.onAbilityInventory = null;
    this.onPlaneFlags = null;
    this.onGetInteractArgs = null;
    this.onGetUseItemArgs = null;
    this.onGetUseAbilityArgs = null;
    this.onSyncStatus = null;
}
exports.Connection = Connection;

Connection.prototype._handleOpen = function(evt) {
    if (this.onOpen != null) {
        this.onOpen(evt);
    }
};

Connection.prototype._handleClose = function(evt) {
    if (this.onClose != null) {
        this.onClose(evt, this._last_kick_reason);
    }
};

Connection.prototype._handleMessage = function(evt) {
    var view = new DataView(evt.data);
    var offset = 0;

    function get8() {
        var result = view.getUint8(offset);
        offset += 1;
        return result;
    }

    function get16() {
        var result = view.getUint16(offset, true);
        offset += 2;
        return result;
    }

    function get32() {
        var result = view.getUint32(offset, true);
        offset += 4;
        return result;
    }

    function getString() {
        var len = get16();
        var result = decodeUtf8(new Uint8Array(view.buffer, offset, len));
        offset += len;
        return result;
    }

    function getArg() {
        var tag = get8();
        switch (tag) {
            case 0: return get32();
            case 1: return getString();

            case 2:
                var len = get16();
                console.log('reading array', len);
                var arr = new Array(len);
                for (var i = 0; i < len; ++i) {
                    arr[i] = getArg();
                }
                return arr;

            case 3:
                var len = get16();
                var map = new Object();
                console.log('reading map', len);
                for (var i = 0; i < len; ++i) {
                    var k = getArg();
                    var v = getArg();
                    map[k] = v;
                }
                return map;
        }
    }

    var opcode = get16();

    switch (opcode) {
        case OP_TERRAIN_CHUNK:
            if (this.onTerrainChunk != null) {
                var chunk_idx = get16();
                // TODO: byte order in the Uint16Array will be wrong on
                // big-endian systems.
                var len = get16();
                this.onTerrainChunk(chunk_idx, new Uint16Array(view.buffer, offset, len));
                offset += 2 * len;
            }
            break;

        case OP_PLAYER_MOTION:
            if (this.onPlayerMotion != null) {
                var id =            get16();
                var start_x =       get16();
                var start_y =       get16();
                var start_z =       get16();
                var start_time =    get16();
                var end_x =         get16();
                var end_y =         get16();
                var end_z =         get16();
                var end_time =      get16();
                var motion = {
                    start_pos:  new Vec(start_x, start_y, start_z),
                    start_time: start_time,
                    end_pos:    new Vec(end_x, end_y, end_z),
                    end_time:   end_time,
                };
                this.onPlayerMotion(id, motion);
            }
            break;

        case OP_PONG:
            if (this.onPong != null) {
                var msg = get16();
                var server_time = get16();
                this.onPong(msg, server_time, evt.timeStamp);
            }
            break;

        case OP_ENTITY_UPDATE:
            if (this.onEntityUpdate != null) {
                var id =            get32();
                var start_x =       get16();
                var start_y =       get16();
                var start_z =       get16();
                var start_time =    get16();
                var end_x =         get16();
                var end_y =         get16();
                var end_z =         get16();
                var end_time =      get16();
                var anim =          get16();
                var motion = {
                    start_pos:  new Vec(start_x, start_y, start_z),
                    start_time: start_time,
                    end_pos:    new Vec(end_x, end_y, end_z),
                    end_time:   end_time,
                };
                this.onEntityUpdate(id, motion, anim);
            }
            break;

        case OP_INIT:
            if (this.onInit != null) {
                var entity_id = get32();
                var now = get16();
                var cycle_base = get32();
                var cycle_ms = get32();
                this.onInit(entity_id, now, cycle_base, cycle_ms);
            }
            break;

        case OP_KICK_REASON:
            var msg = getString();
            this._last_kick_reason = msg;
            break;

        case OP_UNLOAD_CHUNK:
            if (this.onUnloadChunk != null) {
                var idx = get16();
                this.onUnloadChunk(idx);
            };
            break;

        case OP_OPEN_DIALOG:
            if (this.onOpenDialog != null) {
                var idx = get32();
                var len = get16();
                var args = [];
                for (var i = 0; i < len; ++i) {
                    args.push(get32());
                }
                this.onOpenDialog(idx, args);
            };
            break;

        case OP_INVENTORY_UPDATE:
            if (this.onInventoryUpdate != null) {
                var inventory_id = get32();
                var len = get16();
                var updates = [];
                for (var i = 0; i < len; ++i) {
                    var item_id = get16();
                    var old_count = get8();
                    var new_count = get8();
                    updates.push({
                        id: item_id,
                        old_count: old_count,
                        new_count: new_count,
                    });
                }
                this.onInventoryUpdate(inventory_id, updates);
            };
            break;

        case OP_OPEN_CRAFTING:
            if (this.onOpenCrafting != null) {
                var station_type = get32();
                var station_id = get32();
                var inventory_id = get32();
                this.onOpenCrafting(station_type, station_id, inventory_id);
            }
            break;

        case OP_CHAT_UPDATE:
            if (this.onChatUpdate != null) {
                var msg = getString();
                this.onChatUpdate(msg);
            }
            break;

        case OP_ENTITY_APPEAR:
            if (this.onEntityAppear != null) {
                var entity_id = get32();
                var appearance = get32();
                var name = getString();
                this.onEntityAppear(entity_id, appearance, name);
            }
            break;

        case OP_ENTITY_GONE:
            if (this.onEntityGone != null) {
                var entity_id = get32();
                var time = get16();
                this.onEntityGone(entity_id, time);
            }
            break;

        case OP_REGISTER_RESULT:
            if (this.onRegisterResult != null) {
                var code = get32();
                var msg = getString();
                this.onRegisterResult(code, msg);
            }
            break;

        case OP_STRUCTURE_APPEAR:
            if (this.onStructureAppear != null) {
                var structure_id = get32();
                var template_id = get32();
                var x = get16();
                var y = get16();
                var z = get16();
                this.onStructureAppear(structure_id, template_id, x, y, z);
            }
            break;

        case OP_STRUCTURE_GONE:
            if (this.onStructureGone != null) {
                var structure_id = get32();
                this.onStructureGone(structure_id);
            }
            break;

        case OP_MAIN_INVENTORY:
            if (this.onMainInventory != null) {
                var inventory_id = get32();
                this.onMainInventory(inventory_id);
            }
            break;

        case OP_ABILITY_INVENTORY:
            if (this.onAbilityInventory != null) {
                var inventory_id = get32();
                this.onAbilityInventory(inventory_id);
            }
            break;

        case OP_PLANE_FLAGS:
            if (this.onPlaneFlags != null) {
                var flags = get32();
                this.onPlaneFlags(flags);
            }
            break;

        case OP_GET_INTERACT_ARGS:
            if (this.onGetInteractArgs != null) {
                var dialog_id = get32();
                var args = getArg();
                this.onGetInteractArgs(dialog_id, args);
            }
            break;

        case OP_GET_USE_ITEM_ARGS:
            if (this.onGetUseItemArgs != null) {
                var item_id = get16();
                var dialog_id = get32();
                var args = getArg();
                this.onGetUseItemArgs(item_id, dialog_id, args);
            }
            break;

        case OP_GET_USE_ABILITY_ARGS:
            if (this.onGetUseItemArgs != null) {
                var item_id = get16();
                var dialog_id = get32();
                var args = getArg();
                this.onGetUseAbilityArgs(item_id, dialog_id, args);
            }
            break;

        case OP_SYNC_STATUS:
            if (this.onSyncStatus != null) {
                var synced = get8() != 0;
                this.onSyncStatus(synced);
            }
            break;

        default:
            console.assert(false, 'received invalid opcode:', opcode.toString(16));
            break;
    }

    console.assert(offset == view.buffer.byteLength,
            'received message with bad length');
};


/** @constructor */
function MessageBuilder(length) {
    this._buf = new DataView(new ArrayBuffer(length));
    this._offset = 0;
}

MessageBuilder.prototype.put8 = function(n) {
    this._buf.setUint8(this._offset, n);
    this._offset += 1;
};

MessageBuilder.prototype.put16 = function(n) {
    this._buf.setUint16(this._offset, n, true);
    this._offset += 2;
};

MessageBuilder.prototype.put32 = function(n) {
    this._buf.setUint32(this._offset, n, true);
    this._offset += 4;
};

MessageBuilder.prototype.putString = function(s) {
    var utf8 = unescape(encodeURIComponent(s));
    this.put16(utf8.length);
    for (var i = 0; i < utf8.length; ++i) {
        this.put8(utf8.charCodeAt(i));
    }
};

MessageBuilder.prototype.putArg = function(a) {
    switch (typeof(a)) {
        case 'boolean':
        case 'number':
            this.put8(0);
            this.put32(a);
            break;

        case 'string':
            this.put8(1);
            this.putString(a);
            break;

        default:
            if (a.constructor == Array) {
                this.put8(2);
                this.put16(a.length);
                for (var i = 0; i < a.length; ++i) {
                    this.putArg(a[i]);
                }
            } else {
                this.put8(3);
                var props = Object.getOwnPropertyNames(a);
                this.put16(props.length);
                for (var i = 0; i < props.length; ++i) {
                    this.putArg(props[i]);
                    this.putArg(a[props[i]]);
                }
            }
            break;
    }
}

MessageBuilder.prototype.done = function() {
    var buf = new Uint8Array(this._buf.buffer, 0, this._offset);
    return buf;
};

MessageBuilder.prototype.reset = function() {
    this._offset = 0;
    return this;
};


var MESSAGE_BUILDER = new MessageBuilder(8192);


Connection.prototype.sendGetTerrain = function() {
    console.error('deprecated message: GetTerrain');
    var msg = MESSAGE_BUILDER.reset();
    msg.put16(OP_GET_TERRAIN);
    this.socket.send(msg.done());
};

Connection.prototype.sendUpdateMotion = function(data) {
    console.error('deprecated message: UpdateMotion');
    var buf = new Uint16Array(1 + data.length);
    buf[0] = OP_UPDATE_MOTION;
    buf.subarray(1).set(data);
    this.socket.send(buf);
};

Connection.prototype.sendPing = function(data) {
    var msg = MESSAGE_BUILDER.reset();
    msg.put16(OP_PING);
    msg.put16(data);
    this.socket.send(msg.done());
};

Connection.prototype.sendInput = function(time, input) {
    var msg = MESSAGE_BUILDER.reset();
    msg.put16(OP_INPUT);
    msg.put16(time);
    msg.put16(input);
    this.socket.send(msg.done());
};

Connection.prototype.sendLogin = function(name, secret) {
    var msg = MESSAGE_BUILDER.reset();

    msg.put16(OP_LOGIN);
    for (var i = 0; i < 4; ++i) {
        msg.put32(secret[i]);
    }
    msg.putString(name);

    this.socket.send(msg.done());
};

Connection.prototype.sendAction = function(time, action, arg) {
    console.error('deprecated message: Action');
    var msg = MESSAGE_BUILDER.reset();
    msg.put16(OP_ACTION);
    msg.put16(time);
    msg.put16(action);
    msg.put32(arg);
    this.socket.send(msg.done());
};

Connection.prototype.sendUnsubscribeInventory = function(inventory_id) {
    var msg = MESSAGE_BUILDER.reset();
    msg.put16(OP_UNSUBSCRIBE_INVENTORY);
    msg.put32(inventory_id);
    this.socket.send(msg.done());
};

Connection.prototype.sendMoveItem = function(from_inventory, to_inventory, item_id, amount) {
    var msg = MESSAGE_BUILDER.reset();
    msg.put16(OP_MOVE_ITEM);
    msg.put32(from_inventory);
    msg.put32(to_inventory);
    msg.put16(item_id);
    msg.put16(amount);
    this.socket.send(msg.done());
};

Connection.prototype.sendCraftRecipe = function(station_id, inventory_id, recipe_id, count) {
    var msg = MESSAGE_BUILDER.reset();
    msg.put16(OP_CRAFT_RECIPE);
    msg.put32(station_id);
    msg.put32(inventory_id);
    msg.put16(recipe_id);
    msg.put16(count);
    this.socket.send(msg.done());
};

Connection.prototype.sendChat = function(text) {
    var msg = MESSAGE_BUILDER.reset();
    msg.put16(OP_CHAT);
    msg.putString(text);
    this.socket.send(msg.done());
};

Connection.prototype.sendRegister = function(name, secret, appearance) {
    var msg = MESSAGE_BUILDER.reset();

    msg.put16(OP_REGISTER);
    for (var i = 0; i < 4; ++i) {
        msg.put32(secret[i]);
    }
    msg.put32(appearance);
    msg.putString(name);

    this.socket.send(msg.done());
};

Connection.prototype.sendInteract = function(time) {
    var msg = MESSAGE_BUILDER.reset();
    msg.put16(OP_INTERACT);
    msg.put16(time);
    this.socket.send(msg.done());
};

Connection.prototype.sendUseItem = function(time, item_id) {
    var msg = MESSAGE_BUILDER.reset();
    msg.put16(OP_USE_ITEM);
    msg.put16(time);
    msg.put16(item_id);
    this.socket.send(msg.done());
};

Connection.prototype.sendUseAbility = function(time, item_id) {
    var msg = MESSAGE_BUILDER.reset();
    msg.put16(OP_USE_ABILITY);
    msg.put16(time);
    msg.put16(item_id);
    this.socket.send(msg.done());
};

Connection.prototype.sendOpenInventory = function() {
    var msg = MESSAGE_BUILDER.reset();
    msg.put16(OP_OPEN_INVENTORY);
    this.socket.send(msg.done());
};

Connection.prototype.sendInteractWithArgs = function(time, args) {
    var msg = MESSAGE_BUILDER.reset();
    msg.put16(OP_INTERACT_WITH_ARGS);
    msg.put16(time);
    msg.putArg(args);
    this.socket.send(msg.done());
};

Connection.prototype.sendUseItemWithArgs = function(time, item_id, args) {
    var msg = MESSAGE_BUILDER.reset();
    msg.put16(OP_USE_ITEM_WITH_ARGS);
    msg.put16(time);
    msg.put16(item_id);
    msg.putArg(args);
    this.socket.send(msg.done());
};

Connection.prototype.sendUseAbilityWithArgs = function(time, item_id, args) {
    var msg = MESSAGE_BUILDER.reset();
    msg.put16(OP_USE_ABILITY_WITH_ARGS);
    msg.put16(time);
    msg.put16(item_id);
    msg.putArg(args);
    this.socket.send(msg.done());
};

