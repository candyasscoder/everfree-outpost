var Vec = require('vec').Vec;
var decodeUtf8 = require('util').decodeUtf8;

var OP_GET_TERRAIN =        0x0001;
var OP_UPDATE_MOTION =      0x0002;
var OP_PING =               0x0003;
var OP_INPUT =              0x0004;
var OP_LOGIN =              0x0005;

var OP_TERRAIN_CHUNK =      0x8001;
var OP_PLAYER_MOTION =      0x8002;
var OP_PONG =               0x8003;
var OP_ENTITY_UPDATE =      0x8004;
var OP_INIT =               0x8005;
var OP_KICK_REASON =        0x8006;

/** @constructor */
function Connection(url) {
    var this_ = this;

    var socket = new WebSocket(url);
    socket.binaryType = 'arraybuffer';
    socket.onopen = function(evt) { this_._handleOpen(evt); };
    socket.onmessage = function(evt) { this_._handleMessage(evt); };
    socket.onclose = function(evt) { this_._handleClose(evt); };
    this.socket = socket;

    this.onOpen = null;
    this.onClose = null;
    this.onTerrainChunk = null;
    this.onPlayerMotion = null;
    this.onPong = null;
    this.onEntityUpdate = null;
    this.onInit = null;
    this.onKickReason = null;
}
exports.Connection = Connection;

Connection.prototype._handleOpen = function(evt) {
    if (this.onOpen != null) {
        this.onOpen(evt);
    }
};

Connection.prototype._handleClose = function(evt) {
    if (this.onClose != null) {
        this.onClose(evt);
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
            if (this.onKickReason != null) {
                var msg = decodeUtf8(new Uint8Array(view.buffer, 2));
            }
            break;

        default:
            console.assert(false, 'received invalid opcode:', opcode);
            break;
    }
};

Connection.prototype.sendGetTerrain = function() {
    // TODO: using uint16array will break on big-endian
    var buf = new Uint16Array(1);
    buf[0] = OP_GET_TERRAIN;
    this.socket.send(buf);
};

Connection.prototype.sendUpdateMotion = function(data) {
    if (this.socket.readyState != WebSocket.OPEN) {
        return;
    }

    var buf = new Uint16Array(1 + data.length);
    buf[0] = OP_UPDATE_MOTION;
    buf.subarray(1).set(data);
    this.socket.send(buf);
};

Connection.prototype.sendPing = function(msg) {
    var buf = new Uint16Array(2);
    buf[0] = OP_PING;
    buf[1] = msg;
    this.socket.send(buf);
};

Connection.prototype.sendInput = function(time, input) {
    var buf = new Uint16Array(3);
    buf[0] = OP_INPUT;
    buf[1] = time;
    buf[2] = input;
    this.socket.send(buf);
};

Connection.prototype.sendLogin = function(secret, name) {
    var name_utf8 = unescape(encodeURIComponent(name));
    var buf = new DataView(new ArrayBuffer(2 + 32 + name_utf8.length));

    buf.setUint16(0, OP_LOGIN, true);
    for (var i = 0; i < 4; ++i) {
        buf.setInt32(2 + i * 4, secret[i], true);
    }
    for (var i = 0; i < name_utf8.length; ++i) {
        buf.setUint8(2 + 32 + i, name_utf8.charCodeAt(i));
    }

    this.socket.send(buf);
};
