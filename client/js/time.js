var TIMER_RANGE = 0x10000;
var TIMER_MASK = 0xffff;

/** @constructor */
function Timing(conn) {
    this.conn = conn;
    this.offset_send = -1;
    this.offset_recv = -1;

    var this_ = this;
    this.conn.onPong = function(cs, s, cr) { this_._handlePong(cs, s, cr); };
    this.update();
}
exports.Timing = Timing;

Timing.prototype.update = function() {
    this.conn.sendPing(Date.now() & TIMER_MASK);
};

Timing.prototype._handlePong = function(client_send, server_time, client_recv_raw) {
    // TODO: on firefox, event.timeStamp appears to be in microseconds instead
    // of milliseconds.  Just use Date.now() instead.
    client_recv_raw = Date.now();
    /*
    if (client_recv_raw == null) {
        client_recv_raw = Date.now();
    }
    */

    var client_recv = client_recv_raw % TIMER_RANGE;
    this.offset_send = (server_time - client_send) & TIMER_MASK;
    this.offset_recv = (client_recv - server_time) & TIMER_MASK;
};

Timing.prototype.encodeSend = function(client) {
    return (client + this.offset_send) & TIMER_MASK;
};

Timing.prototype.decodeRecv = function(server, now) {
    var client_masked = (server + this.offset_recv) & TIMER_MASK;
    var offset = (client_masked - now) & TIMER_MASK;
    if (offset >= TIMER_RANGE / 2) {
        offset -= TIMER_RANGE;
    }
    return now + offset;
};
