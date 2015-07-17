/** @constructor */
function ExtraDefsClass() {
    this.anim_dir_table = null;
    this.default_anim = null;
    this.physics_anim_table = null;
}

ExtraDefsClass.prototype.init = function(info) {
    this.anim_dir_table = info['anim_dir_table'];
    this.default_anim = info['default_anim'];
    this.physics_anim_table = info['physics_anim_table'];
};

var ExtraDefs = new ExtraDefsClass();
exports.ExtraDefs = ExtraDefs;
