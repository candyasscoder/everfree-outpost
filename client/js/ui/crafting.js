var Config = require('config').Config;
var ItemDef = require('data/items').ItemDef;
var RecipeDef = require('data/recipes').RecipeDef;
var SelectionList = require('ui/sortedlist').SelectionList;
var ItemList = require('ui/inventory').ItemList;
var fromTemplate = require('util/misc').fromTemplate;
var InventoryTracker = require('inventory').InventoryTracker;
var chain = require('util/misc').chain;


/** @constructor */
function CraftingUI(station_type, station_id, inv) {
    this.recipe_list = new RecipeList(station_type, inv);
    this.item_list = new ItemList(inv);
    this.station_id = station_id;

    this.container = fromTemplate('crafting', {
        'item_list': this.item_list.container,
        'recipe_list': this.recipe_list.container,
    });

    this.dialog = null;

    this.inv_active = false;
    this.recipe_list.container.classList.add('active');

    this.onaction = null;
    this.onclose = null;
}
exports.CraftingUI = CraftingUI;

CraftingUI.prototype._activate = function(new_inv_active) {
    this._getActive().container.classList.remove('active');
    this.inv_active = new_inv_active;
    this._getActive().container.classList.add('active');
};

CraftingUI.prototype._getActive = function() {
    if (!this.inv_active) {
        return this.recipe_list;
    } else {
        return this.item_list;
    }
};

CraftingUI.prototype._handleKeyEvent = function(down, evt) {
    if (!down) {
        return;
    }

    var binding = Config.keybindings.get()[evt.keyCode];

    var mag = evt.shiftKey ? 10 : 1;

    switch (binding) {
        case 'move_up':
            this._getActive().step(-1 * mag);
            break;
        case 'move_down':
            this._getActive().step(1 * mag);
            break;

        case 'move_left':
            if (this.inv_active) {
                this._activate(false);
            }
            break;
        case 'move_right':
            if (!this.inv_active) {
                this._activate(true);
            }
            break;

        case 'interact':
            if (this.onaction != null) {
                var recipe_id = this.recipe_list.selectedRecipe();
                if (recipe_id != -1) {
                    var inventory_id = this.item_list.inventory_id;
                    this.onaction(this.station_id, inventory_id, recipe_id, mag);
                }
            }
            break;

        case 'cancel':
            this.dialog.hide();
            break;
    }
};

CraftingUI.prototype.handleOpen = function(dialog) {
    var this_ = this;
    this.dialog = dialog;
    dialog.keyboard.pushHandler(function(d, e) { return this_._handleKeyEvent(d, e); });
};

CraftingUI.prototype.handleClose = function(dialog) {
    this.dialog = null;
    dialog.keyboard.popHandler();

    if (this.onclose != null) {
        this.onclose();
    }
};


/** @constructor */
function RecipeList(station_type, inv) {
    this.list = new SelectionList('recipe-list');
    this.container = this.list.container;
    this.inv = inv;

    this.onchange = null;

    var this_ = this;
    this.list.onchange = function(row) {
        if (row == null) {
            if (this_.onchange != null) {
                this_.onchange(-1);
            }
            return;
        }

        if (this_.onchange != null) {
            this_.onchange(row.id);
        }

        this_._scrollToSelection();
    };

    var init = [];
    for (var i = 0; i < RecipeDef.by_id.length; ++i) {
        var recipe = RecipeDef.by_id[i];
        if (recipe != null && recipe.station == station_type) {
            init.push({
                id: i,
                old_count: 0,
                new_count: 1,
            });
        }
    }
    this.update(init);

    this._markCraftable();
    inv.onUpdate(function(updates) {
        this_._markCraftable();
    });
}
exports.RecipeList = RecipeList;

RecipeList.prototype._scrollToSelection = function() {
    // TODO: this is copied from ItemList, factor it out somewhere
    var sel = this.list.selection();
    if (sel == null) {
        return;
    }

    var item_bounds = sel.container.getBoundingClientRect();
    var parent_bounds = this.container.getBoundingClientRect();
    var target_top = parent_bounds.top + parent_bounds.height / 2 - item_bounds.height / 2;
    // Adjust scrollTop to move 'item_bounds.top' to 'target_top'.
    var delta = target_top - item_bounds.top;
    // Use -= like in ItemList
    this.container.scrollTop -= delta;
};

RecipeList.prototype._markCraftable = function() {
    for (var i = 0; i < this.list.length(); ++i) {
        var row = this.list.get(i);
        if (!canCraft(RecipeDef.by_id[row.id], this.inv)) {
            row.container.classList.add('disabled');
        } else {
            row.container.classList.remove('disabled');
        }
    }
};

RecipeList.prototype.select = function(id) {
    this.list.select(id);
};

RecipeList.prototype.step = function(offset) {
    this.list.step(offset);
};

RecipeList.prototype.update = function(updates) {
    this.list.update(updates, function(up, row) {
        if (up.new_count == 0) {
            return null;
        } else if (up.old_count == 0) {
            var id = up.id;
            var def = RecipeDef.by_id[id];
            return new RecipeRow(id, def.ui_name);
        } else {
            return row;
        }
    });
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
            console.log("can't craft", recipe.name, " - not enough", item_id, inv.count(item_id), count);
            return false;
        }
    }
    return true;
}


/** @constructor */
function RecipeRow(id, name) {
    this.container = document.createElement('div');
    this.container.classList.add('recipe');

    var nameDiv = document.createElement('div');
    nameDiv.classList.add('recipe-name');
    nameDiv.textContent = name;
    this.container.appendChild(nameDiv);

    this.id = id;
}
