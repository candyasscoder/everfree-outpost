var Config = require('config').Config;
var ItemDef = require('data/items').ItemDef;
var RecipeDef = require('data/recipes').RecipeDef;
var SelectionList = require('ui/sortedlist').SelectionList;
var ItemList = require('ui/inventory').ItemList;
var fromTemplate = require('util/misc').fromTemplate;
var InventoryTracker = require('inventory').InventoryTracker;
var chain = require('util/misc').chain;
var widget = require('ui/widget');


/** @constructor */
function CraftingUI(station_type, station_id, inv) {
    this.recipe_list = new RecipeList(station_type, inv);
    this.item_list = new ItemList(inv);
    this.station_id = station_id;

    this.dom = fromTemplate('crafting', {
        'item_list': this.item_list.dom,
        'recipe_list': this.recipe_list.dom,
    });

    var this_ = this;
    this.focus = new widget.FocusTracker(
            [this.recipe_list, this.item_list],
            ['move_left', 'move_right']);
    this.keys = new widget.ActionKeyHandler(
            'select',
            function(evt) { this_._craft(evt.shiftKey ? 10 : 1); },
            this.focus);

    this.dialog = null;

    this.onaction = null;
    this.onclose = null;
}
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

CraftingUI.prototype.handleOpen = function(dialog) {
    this.dialog = dialog;
};

CraftingUI.prototype.handleClose = function(dialog) {
    if (this.onclose != null) {
        this.onclose();
    }
};


/** @constructor */
function RecipeList(station_type, inv) {
    var recipe_items = [];
    for (var i = 0; i < RecipeDef.by_id.length; ++i) {
        var recipe = RecipeDef.by_id[i];
        if (recipe != null && recipe.station == station_type) {
            recipe_items.push(new RecipeRow(i, recipe.ui_name));
        }
    }
    this.items = recipe_items;

    this.list = new widget.SimpleList('recipe-list', recipe_items);
    this.dom = this.list.dom;
    this.keys = this.list.keys;

    this.inv = inv;

    var this_ = this;

    this.onchange = null;
    this.list.onchange = function(idx) {
        if (this_.onchange != null) {
            this_.onchange(idx);
        }
    };

    this._markCraftable();
    inv.onUpdate(function(updates) {
        this_._markCraftable();
    });
}
exports.RecipeList = RecipeList;

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
    var sel = this.list.selection();
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
    this.dom = document.createElement('div');
    this.dom.classList.add('recipe');

    var nameDiv = document.createElement('div');
    nameDiv.classList.add('recipe-name');
    nameDiv.textContent = name;
    this.dom.appendChild(nameDiv);

    this.id = id;
}
