var Deque = require('util/misc').Deque;
var Vec = require('util/vec').Vec;
var Asm = require('asmlibs').Asm;
var getPhysicsHeapSize = require('asmlibs').getPhysicsHeapSize;
var Motion = require('entity').Motion;
var BlockDef = require('data/chunk').BlockDef;
var CHUNK_SIZE = require('data/chunk').CHUNK_SIZE;
var LOCAL_SIZE = require('data/chunk').LOCAL_SIZE;

var INT_MAX = 0x7fffffff;
var INT_MIN = -INT_MAX - 1;

var DURATION_MAX = 0xffff;


/** @constructor */
function Physics() {
    var chunk_total = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
    var local_total = LOCAL_SIZE * LOCAL_SIZE;
    this._asm = new Asm(getPhysicsHeapSize());
}
exports.Physics = Physics;

Physics.prototype.loadChunk = function(ci, cj, tiles) {
    var view = this._asm.shapeLayerView(ci * LOCAL_SIZE + cj, -1);
    console.assert(tiles.length == view.length,
            'expected ' + view.length + ' tiles, but got ' + tiles.length);

    for (var i = 0; i < tiles.length; ++i) {
        view[i] = BlockDef.by_id[tiles[i]].shape;
    }

    var base = new Vec(cj * CHUNK_SIZE,
                       ci * CHUNK_SIZE,
                       0);
    var size = new Vec(CHUNK_SIZE, CHUNK_SIZE, CHUNK_SIZE);
    this._asm.refreshShapeLayers(base, size);
};

Physics.prototype.addStructure = function(structure) {
    var template = structure.template;
    this._asm.setRegionShape(structure.pos, template.size, template.layer, template.shape);
};

Physics.prototype.removeStructure = function(structure) {
    var template = structure.template;
    this._asm.clearRegionShape(structure.pos, template.size, template.layer);
};

Physics.prototype.computeForecast = function(now, entity, target_velocity) {
    var start_pos = entity.position(now);
    // TODO: hardcoded constant based on entity size
    var size = new Vec(32, 32, 64);

    var result = this._asm.collide(start_pos, size, target_velocity);
    var end_pos = new Vec(result.x, result.y, result.z);
    var dur = result.t;

    // NB: keep this in sync with server/physics_.rs  Fragment::update
    if (dur > DURATION_MAX) {
        var offset = end_pos.sub(start_pos);
        end_pos = start_pos.add(offset.mulScalar(DURATION_MAX).divScalar(dur));
        dur = DURATION_MAX;
    } else if (dur == 0) {
        dur = DURATION_MAX;
    }

    var motion = new Motion(start_pos);
    motion.end_pos = end_pos;
    motion.start_time = now;
    motion.end_time = now + dur;

    // TODO: player speed handling shouldn't be here
    var speed = target_velocity.abs().max() / 50;
    var facing = target_velocity.sign();
    var idx = (3 * (facing.x + 1) + (facing.y + 1));
    var anim_dir = [5, 4, 3, 6, entity.animId(now) % 8, 2, 7, 0, 1][idx];
    motion.anim_id = anim_dir + speed * 8;

    return motion;
};

Physics.prototype.findCeiling = function(pos) {
    return this._asm.findCeiling(pos);
};


/** @constructor */
function Prediction(physics) {
    this.physics = physics;
    this.predicted = new Deque();
    this.inputs = new Deque();
    this.last_target_velocity = new Vec(0, 0, 0);
}
exports.Prediction = Prediction;

Prediction.prototype.predict = function(now, entity, target_velocity) {
    this.inputs.enqueue(new PredictionInput(now, target_velocity));
    this._predictNoInput(now, entity, target_velocity);
};

Prediction.prototype._predictNoInput = function(now, entity, target_velocity) {
    var m = this.physics.computeForecast(now, entity, target_velocity);
    this.predicted.enqueue(m);
    entity.queueMotion(m);
    this.last_target_velocity = target_velocity;
};

Prediction.prototype.refreshMotion = function(now, entity) {
    this.predict(now, entity, this.last_target_velocity);
};

Prediction.prototype.receivedMotion = function(m, entity) {
    // Flush all inputs earlier than `m.start_time`, under the assumption that
    // they have already taken effect.  (This may not be true if there was a
    // large unexpected delay sending the input to the server.)
    while (this.inputs.peek() != null && this.inputs.peek().time <= m.start_time) {
        this.inputs.dequeue();
    }

    var predicted = this.predicted.dequeue();
    if (predicted != null && motions_equal(m, predicted)) {
        // Received motion exactly matches the prediction.
        return;
    }

    // Motions are unequal.  Flush the entity's and the predictor's motion
    // queues, and replay from the inputs.
    entity.resetMotion(m);
    this.predicted = new Deque();

    var this_ = this;
    this.inputs.forEach(function(input) {
        this_._predictNoInput(input.time, entity, input.velocity);
    });
};

function motions_equal(m1, m2) {
    // TODO: include end once client physics is un-broken
    return m1.start_time == m2.start_time &&
           vecs_equal(m1.start_pos, m2.start_pos);
}

function vecs_equal(v1, v2) {
    var LOCAL_MASK = LOCAL_SIZE - 1;
    return (v1.x & LOCAL_MASK) == (v2.x & LOCAL_MASK) &&
           (v1.y & LOCAL_MASK) == (v2.y & LOCAL_MASK) &&
           (v1.z & LOCAL_MASK) == (v2.z & LOCAL_MASK);
}


/** @constructor */
function PredictionInput(time, velocity) {
    this.time = time;
    this.velocity = velocity;
}


/** @constructor */
function DummyPrediction() {
}
exports.DummyPrediction = DummyPrediction;

DummyPrediction.prototype.predict = function() {};
DummyPrediction.prototype.refreshMotion = function() {};

DummyPrediction.prototype.receivedMotion = function(m, entity) {
    entity.queueMotion(m);
};
