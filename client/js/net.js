var OP_GET_TERRAIN =        0x0001;

var OP_TERRAIN_CHUNK =      0x8001;

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
        default:
            console.assert(false, 'received invalid opcode:', opcode);
            break;
    }
};

Connection.prototype.sendGetTerrain = function() {
    var buf = new Uint16Array(1);
    buf[0] = OP_GET_TERRAIN;
    this.socket.send(buf);
};
