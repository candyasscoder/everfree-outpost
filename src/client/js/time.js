var Config = require('config').Config;

var TIMER_RANGE = 0x10000;
var TIMER_MASK = 0xffff;

// There are two interesting timelines: client time and server time.  We cannot
// observer server time directly (we only receive the 16-bit truncated
// version), so we instead use "pseudo-server time", which is offset from
// actual server time such that time zero is set based on client startup.
// (Specifically, PST 0 is set such that the time returned by the first Pong
// has the same representation in ST and PST.)
//
// Pseudo-server time is offset from real server time by a multiple of 2^16, so
// the 16-bit truncation of PST is the same as the truncation of ST.  PST and
// ST advance in lockstep, so this is true at all points in time.
//
// The offset between client time and (pseudo-)server time varies based on the
// current ping.
//
// Nearly everything outside this module operates on PST.  Rendering shows a
// snapshot of the state of the world as of a particular server time.  Input
// handling uses the predicted server time when the OP_INPUT message will be
// received by the server.  All CT <-> PST conversion is handled here.

/** @constructor */
function Timing(conn) {
    this.conn = conn;

    // The CT timestamp corresponding to PST 0.
    this.client_base = null;
    this.ping = 0;

    var this_ = this;
    this.conn.onPong = function(cs, s, cr) { this_._handlePong(cs, s, cr); };
    this.update();
}
exports.Timing = Timing;

Timing.prototype.update = function() {
    this.conn.sendPing(Date.now() & TIMER_MASK);
};

Timing.prototype.scheduleUpdates = function(delay, interval) {
    var this_ = this;
    function callback() {
        this_.update();
        setTimeout(callback, interval * 1000);
    }

    setTimeout(callback, delay * 1000);
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


    this.ping = (client_recv_raw - client_send) & TIMER_MASK;

    if (this.client_base == null) {
        this.client_base = client_recv_raw;
    }
    var server_pst = this.decodeRecv(server_time, client_recv_raw);
    this.client_base = client_recv_raw - server_pst;
};

// There are two server timestamps we might want to get.
//
//   Server ------ A --------- B ------
//                   \       /
//                    \     /
//                     \   /
//   Client ------------ C ------------
//
// `C` is the current client time (Date.now()).  `A` is the "latest visible
// time" - the latest time such that all server messages generated before that
// time have already been received by this client.  `B` is the "next arrival
// time" - the earliest time that a message sent to the server could possibly
// arrive.

// Get the latest visible time.
Timing.prototype.visibleNow = function() {
    if (this.client_base == null) {
        return -0xffff;
    }
    return (Date.now() - this.client_base)|0;
};

// Get the next arrival time.
Timing.prototype.nextArrival = function() {
    if (this.client_base == null) {
        return -0xffff;
    }
    return (Date.now() - this.client_base + this.ping)|0;
};

// Get the PST corresponding to a truncated ST, using the current CT as a base.
Timing.prototype.decodeRecv = function(server, client) {
    if (client == null) {
        client = Date.now();
    }
    var base_pst = (client - this.client_base)|0;
    var offset = (server - base_pst) & TIMER_MASK;
    if (offset >= TIMER_RANGE / 2) {
        offset -= TIMER_RANGE;
    }
    return base_pst + offset;
};

Timing.prototype.encodeSend = function(server) {
    return server & TIMER_MASK;
}
