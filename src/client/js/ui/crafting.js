var Config = require('config').Config;
var ItemDef = require('data/items').ItemDef;
var RecipeDef = require('data/recipes').RecipeDef;
var ItemList = require('ui/inventory').ItemList;
var ItemRow = require('ui/inventory').ItemRow;
var InventoryTracker = require('inventory').InventoryTracker;
var util = require('util/misc');
var widget = require('ui/widget');


/** @constructor */
function CraftingUI(station_type, station_id, inv) {
    this.recipe_list = new RecipeList(station_type, inv);
    this.item_list = new ItemList(inv);
    this.station_id = station_id;
    this.inv = inv;

    this.input_div = util.element('div', ['recipe-item-list']);
    this.output_div = util.element('div', ['recipe-item-list']);
    this.arrow_div = util.element('div', ['recipe-item-arrow']);
    this.arrow_div.innerHTML = '&dArr;';

    var parts = util.templateParts('crafting', {
        'item_list': this.item_list.dom,
        'recipe_list': this.recipe_list.dom,
        'input_list': this.input_div,
        'output_list': this.output_div,
        'arrow': this.arrow_div,
    });

    var this_ = this;
    this.recipe_list.onchange = function() {
        this_._updateRecipeDisplay();
    };
    this_._updateRecipeDisplay();

    var list = new widget.SimpleList(
            parts['container'],
            [this.recipe_list, this.item_list],
            ['move_left', 'move_right']);

    widget.Form.call(this, list, parts['top']);

    widget.hookKey(this.recipe_list, 'select', function(evt) {
        if (evt.down) {
            this_._craft(evt.shiftKey ? 10 : 1);
        }
    });

    this.onaction = null;
}
CraftingUI.prototype = Object.create(widget.Form.prototype);
CraftingUI.prototype.constructor = CraftingUI;
exports.CraftingUI = CraftingUI;

CraftingUI.prototype._craft = function(mag) {
    if (this.onaction != null) {
        var recipe_id = this.recipe_list.selectedRecipe();
        if (recipe_id != -1) {
            var inventory_id = this.item_list.inventory_id;
            this.onaction(this.station_id, inventory_id, recipe_id, mag);
        }
    }
};

CraftingUI.prototype._updateRecipeDisplay = function() {
    while (this.input_div.firstChild) {
        this.input_div.removeChild(this.input_div.firstChild);
    }
    while (this.output_div.firstChild) {
        this.output_div.removeChild(this.output_div.firstChild);
    }

    var recipe = RecipeDef.by_id[this.recipe_list.selectedRecipe()];
    var craftable = true;

    for (var i = 0; i < recipe.inputs.length; ++i) {
        var item_id = recipe.inputs[i][0];
        var count = recipe.inputs[i][1];
        var item = ItemDef.by_id[item_id];
        var row = new ItemRow(item_id, count, item.ui_name, item.tile_x, item.tile_y);
        if (this.inv.count(item_id) < count) {
            row.dom.classList.add('disabled');
            craftable = false;
        }
        this.input_div.appendChild(row.dom);
    }

    for (var i = 0; i < recipe.outputs.length; ++i) {
        var item_id = recipe.outputs[i][0];
        var count = recipe.outputs[i][1];
        var item = ItemDef.by_id[item_id];
        var row = new ItemRow(item_id, count, item.ui_name, item.tile_x, item.tile_y);
        if (!craftable) {
            row.dom.classList.add('disabled');
        }
        this.output_div.appendChild(row.dom);
    }

    if (!craftable) {
        this.arrow_div.classList.add('disabled');
    } else {
        this.arrow_div.classList.remove('disabled');
    }
};


/** @constructor */
function RecipeList(station_type, inv) {
    var list_div = util.element('div', ['class=recipe-list']);
    var recipe_items = [];
    for (var i = 0; i < RecipeDef.by_id.length; ++i) {
        var recipe = RecipeDef.by_id[i];
        if (recipe != null && recipe.station == station_type) {
            var row = new RecipeRow(i, recipe.ui_name);
            recipe_items.push(row);
            list_div.appendChild(row.dom);
        }
    }

    widget.SimpleList.call(this, list_div, recipe_items);

    this.inv = inv;

    this._markCraftable();
    var this_ = this;
    inv.onUpdate(function(idx, old_item, new_item) {
        this_._markCraftable();
    });
}
RecipeList.prototype = Object.create(widget.SimpleList.prototype);
RecipeList.prototype.constructor = RecipeList;

RecipeList.prototype._markCraftable = function() {
    for (var i = 0; i < this.items.length; ++i) {
        var row = this.items[i];
        if (!canCraft(RecipeDef.by_id[row.id], this.inv)) {
            row.dom.classList.add('disabled');
        } else {
            row.dom.classList.remove('disabled');
        }
    }
};

RecipeList.prototype.selectedRecipe = function() {
    var sel = this.selection();
    if (sel == null) {
        return -1;
    } else {
        return sel.id;
    }
};


function canCraft(recipe, inv) {
    for (var i = 0; i < recipe.inputs.length; ++i) {
        var item_id = recipe.inputs[i][0];
        var count = recipe.inputs[i][1];
        if (inv.count(item_id) < count) {
            return false;
        }
    }
    return true;
}


/** @constructor */
function RecipeRow(id, name) {
    var parts = util.templateParts('recipe');
    parts['name'].textContent = name;

    widget.Element.call(this, parts['top']);

    this.id = id;
}
RecipeRow.prototype = Object.create(widget.Element.prototype);
RecipeRow.prototype.constructor = RecipeRow;
