/** @constructor */
function RecipeDef_(id, info) {
    this.id = id;
    this.name = info['name'];
    this.ui_name = info['ui_name'] || info['name'];
    this.station = info['station'];
    this.inputs = info['inputs'];
    this.outputs = info['outputs'];
}

// Closure compiler doesn't like having static items on functions.
var RecipeDef = {};
exports.RecipeDef = RecipeDef;

RecipeDef.by_id = [];

RecipeDef.register = function(id, info) {
    if (info == null) {
        return;
    }

    var item = new RecipeDef_(id, info);
    while (RecipeDef.by_id.length <= item.id) {
        RecipeDef.by_id.push(null);
    }
    RecipeDef.by_id[item.id] = item;
};
