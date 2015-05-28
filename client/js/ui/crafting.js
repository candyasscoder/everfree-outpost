var Config = require('config').Config;
var ItemDef = require('data/items').ItemDef;
var RecipeDef = require('data/recipes').RecipeDef;
var ItemList = require('ui/inventory').ItemList;
var InventoryTracker = require('inventory').InventoryTracker;
var util = require('util/misc');
var widget = require('ui/widget');


/** @constructor */
function CraftingUI(station_type, station_id, inv) {
    this.recipe_list = new RecipeList(station_type, inv);
    this.item_list = new ItemList(inv);
    this.station_id = station_id;

    var parts = util.templateParts('crafting', {
        'item_list': this.item_list.dom,
        'recipe_list': this.recipe_list.dom,
    });

    var list = new widget.SimpleList(
            parts['container'],
            [this.recipe_list, this.item_list],
            ['move_left', 'move_right']);

    widget.Form.call(this, list, parts['top']);

    var this_ = this;
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
    inv.onUpdate(function(updates) {
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
    var recipeDiv = util.element('div', ['recipe']);
    var nameDiv = util.element('div', ['recipe-name'], recipeDiv);
    nameDiv.textContent = name;

    widget.Element.call(this, recipeDiv);

    this.id = id;
}
RecipeRow.prototype = Object.create(widget.Element.prototype);
RecipeRow.prototype.constructor = RecipeRow;
