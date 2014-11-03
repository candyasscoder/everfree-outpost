var Vec = require('vec').Vec;

var OP_GET_TERRAIN =        0x0001;
var OP_UPDATE_MOTION =      0x0002;
var OP_PING =               0x0003;

var OP_TERRAIN_CHUNK =      0x8001;
var OP_PLAYER_MOTION =      0x8002;
var OP_PONG =               0x8003;

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
    this.onPong = null;
    this.onTerrainChunk = null;
    this.onPlayerMotion = null;
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
    var view = new Uint16Array(evt.data);
    var opcode = view[0];

    switch (opcode) {
        case OP_TERRAIN_CHUNK:
            if (this.onTerrainChunk != null) {
                var chunk_idx = view[1];
                this.onTerrainChunk(chunk_idx, view.subarray(2));
            }
            break;
        case OP_PLAYER_MOTION:
            if (this.onPlayerMotion != null) {
                var id = view[1];
                var motion = {
                    start_pos:  new Vec(view[2], view[3], view[4]),
                    start_time: view[5],
                    end_pos:    new Vec(view[6], view[7], view[8]),
                    end_time:   view[9],
                };
                this.onPlayerMotion(id, motion);
            }
            break;
        case OP_PONG:
            if (this.onPong != null) {
                var msg = view[1];
                var server_time = view[2];
                this.onPong(msg, server_time);
            }
            break;
        default:
            console.assert(false, 'received invalid opcode:', opcode);
            break;
    }
};

Connection.prototype.sendPing = function(msg) {
    var buf = new Uint16Array(2);
    buf[0] = OP_PING;
    buf[1] = msg;
    this.socket.send(buf);
};

Connection.prototype.sendGetTerrain = function() {
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
