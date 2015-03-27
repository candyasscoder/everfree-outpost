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
        var result = decodeUtf8(new Uint8Array(view.buffer, offset));
        offset = view.buffer.byteLength;
        return result;
    }

    var opcode = get16();

    switch (opcode) {
        case OP_TERRAIN_CHUNK:
            if (this.onTerrainChunk != null) {
                var chunk_idx = get16();
                // TODO: byte order in the Uint16Array will be wrong on
                // big-endian systems.
                this.onTerrainChunk(chunk_idx, new Uint16Array(view.buffer, 4));
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
                var camera_x = get16();
                var camera_y = get16();
                var chunks = get8();
                var entities = get8();
                this.onInit(entity_id, camera_x, camera_y, chunks, entities);
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
                var args = [];
                while (offset < view.byteLength) {
                    args.push(get32());
                }
                this.onOpenDialog(idx, args);
            };
            break;

        case OP_INVENTORY_UPDATE:
            if (this.onInventoryUpdate != null) {
                var inventory_id = get32();
                var updates = [];
                while (offset < view.byteLength) {
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

        default:
            console.assert(false, 'received invalid opcode:', opcode.toString(16));
            break;
    }
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

MessageBuilder.prototype.done = function() {
    console.assert(this._offset == this._buf.byteLength);
    return this._buf;
};


Connection.prototype.sendGetTerrain = function() {
    console.error('deprecated message: GetTerrain');
    var msg = new MessageBuilder(2);
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
    var msg = new MessageBuilder(4);
    msg.put16(OP_PING);
    msg.put16(data);
    this.socket.send(msg.done());
};

Connection.prototype.sendInput = function(time, input) {
    var msg = new MessageBuilder(6);
    msg.put16(OP_INPUT);
    msg.put16(time);
    msg.put16(input);
    this.socket.send(msg.done());
};

Connection.prototype.sendLogin = function(name, secret) {
    var name_utf8 = unescape(encodeURIComponent(name));
    var msg = new MessageBuilder(2 + 16 + name_utf8.length);

    msg.put16(OP_LOGIN);
    for (var i = 0; i < 4; ++i) {
        msg.put32(secret[i]);
    }
    for (var i = 0; i < name_utf8.length; ++i) {
        msg.put8(name_utf8.charCodeAt(i));
    }

    this.socket.send(msg.done());
};

Connection.prototype.sendAction = function(time, action, arg) {
    console.error('deprecated message: Action');
    var msg = new MessageBuilder(10);
    msg.put16(OP_ACTION);
    msg.put16(time);
    msg.put16(action);
    msg.put32(arg);
    this.socket.send(msg.done());
};

Connection.prototype.sendUnsubscribeInventory = function(inventory_id) {
    var msg = new MessageBuilder(6);
    msg.put16(OP_UNSUBSCRIBE_INVENTORY);
    msg.put32(inventory_id);
    this.socket.send(msg.done());
};

Connection.prototype.sendMoveItem = function(from_inventory, to_inventory, item_id, amount) {
    var msg = new MessageBuilder(14);
    msg.put16(OP_MOVE_ITEM);
    msg.put32(from_inventory);
    msg.put32(to_inventory);
    msg.put16(item_id);
    msg.put16(amount);
    this.socket.send(msg.done());
};

Connection.prototype.sendCraftRecipe = function(station_id, inventory_id, recipe_id, count) {
    var msg = new MessageBuilder(14);
    msg.put16(OP_CRAFT_RECIPE);
    msg.put32(station_id);
    msg.put32(inventory_id);
    msg.put16(recipe_id);
    msg.put16(count);
    this.socket.send(msg.done());
};

Connection.prototype.sendChat = function(msg) {
    var utf8 = unescape(encodeURIComponent(msg));
    var msg = new MessageBuilder(2 + utf8.length);
    msg.put16(OP_CHAT);
    for (var i = 0; i < utf8.length; ++i) {
        msg.put8(utf8.charCodeAt(i));
    }
    this.socket.send(msg.done());
};

Connection.prototype.sendRegister = function(name, secret, appearance) {
    var utf8 = unescape(encodeURIComponent(name));
    var msg = new MessageBuilder(2 + 16 + 4 + utf8.length);

    msg.put16(OP_REGISTER);
    for (var i = 0; i < 4; ++i) {
        msg.put32(secret[i]);
    }
    msg.put32(appearance);
    for (var i = 0; i < utf8.length; ++i) {
        msg.put8(utf8.charCodeAt(i));
    }

    this.socket.send(msg.done());
};

Connection.prototype.sendInteract = function(time) {
    var msg = new MessageBuilder(4);
    msg.put16(OP_INTERACT);
    msg.put16(time);
    this.socket.send(msg.done());
};

Connection.prototype.sendUseItem = function(time, item_id) {
    var msg = new MessageBuilder(6);
    msg.put16(OP_USE_ITEM);
    msg.put16(time);
    msg.put16(item_id);
    this.socket.send(msg.done());
};

Connection.prototype.sendUseAbility = function(time, item_id) {
    var msg = new MessageBuilder(6);
    msg.put16(OP_USE_ABILITY);
    msg.put16(time);
    msg.put16(item_id);
    this.socket.send(msg.done());
};

Connection.prototype.sendOpenInventory = function() {
    var msg = new MessageBuilder(2);
    msg.put16(OP_OPEN_INVENTORY);
    this.socket.send(msg.done());
};
