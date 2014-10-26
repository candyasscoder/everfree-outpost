/** @constructor */
function Vec(x, y, z) {
    this.x = x | 0;
    this.y = y | 0;
    this.z = z | 0;
}
exports.Vec = Vec;

Vec.prototype.clone = function() {
    return new Vec(this.x, this.y, this.z);
};

Vec.prototype.add = function(other) {
    return new Vec(this.x + other.x, this.y + other.y, this.z + other.z);
};

Vec.prototype.addScalar = function(c) {
    return new Vec(this.x + c, this.y + c, this.z + c);
};

Vec.prototype.sub = function(other) {
    return new Vec(this.x - other.x, this.y - other.y, this.z - other.z);
};

Vec.prototype.subScalar = function(c) {
    return new Vec(this.x - c, this.y - c, this.z - c);
};

Vec.prototype.mul = function(other) {
    return new Vec((this.x * other.x)|0, (this.y * other.y)|0, (this.z * other.z)|0);
};

Vec.prototype.mulScalar = function(c) {
    return new Vec((this.x * c)|0, (this.y * c)|0, (this.z * c)|0);
};

Vec.prototype.div = function(other) {
    return new Vec((this.x / other.x)|0, (this.y / other.y)|0, (this.z / other.z)|0);
};

Vec.prototype.divScalar = function(c) {
    return new Vec((this.x / c)|0, (this.y / c)|0, (this.z / c)|0);
};

Vec.prototype.sign = function() {
    return new Vec(Math.sign(this.x), Math.sign(this.y), Math.sign(this.z));
};

Vec.prototype.isPositive = function() {
    return new Vec(this.x > 0 ? 1 : 0, this.y > 0 ? 1 : 0, this.z > 0 ? 1 : 0);
};

Vec.prototype.isNegative = function() {
    return new Vec(this.x < 0 ? 1 : 0, this.y < 0 ? 1 : 0, this.z < 0 ? 1 : 0);
};

Vec.prototype.isZero = function() {
    return new Vec(this.x == 0 ? 1 : 0, this.y == 0 ? 1 : 0, this.z == 0 ? 1 : 0);
};

Vec.prototype.choose = function(a, b) {
    return new Vec(
            this.x ? a.x : b.x,
            this.y ? a.y : b.y,
            this.z ? a.z : b.z);
};

Vec.prototype.clamp = function(min, max) {
    return new Vec(
            Math.min(max, Math.max(min, this.x)),
            Math.min(max, Math.max(min, this.y)),
            Math.min(max, Math.max(min, this.z)));
};

Vec.prototype.map = function(f) {
    return new Vec(f(this.x), f(this.y), f(this.z));
};

Vec.prototype.forEach = function(f) {
    f(this.x);
    f(this.y);
    f(this.z);
};

Vec.prototype.zip = function(a, f) {
    return new Vec(
            f(this.x, a.x),
            f(this.y, a.y),
            f(this.z, a.z));
};

Vec.prototype.zip3 = function(a, b, f) {
    return new Vec(
            f(this.x, a.x, b.x),
            f(this.y, a.y, b.y),
            f(this.z, a.z, b.z));
};

Vec.prototype.zip4 = function(a, b, c, f) {
    return new Vec(
            f(this.x, a.x, b.x, c.x),
            f(this.y, a.y, b.y, c.y),
            f(this.z, a.z, b.z, c.z));
};

Vec.prototype.get = function(i) {
    if (i == 0) {
        return this.x;
    } else if (i == 1) {
        return this.y;
    } else if (i == 2) {
        return this.z;
    } else {
        throw 'Vec.get: bad index';
    }
};

Vec.prototype.toString = function() {
    return [this.x, this.y, this.z].join(',');
};
