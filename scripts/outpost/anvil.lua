local action = require('core.action')
local structure_items = require('outpost.lib.structure_items')
local tools = require('outpost.lib.tools')

function action.use.anvil(c, s)
    c:open_crafting(s, c:pawn():inventory('main'))
end

function action.use_item.anvil(c, inv)
    structure_items.use_item(c, inv, 'anvil', 'anvil')
end

function tools.handler.pick.anvil(c, s, inv)
    structure_items.use_structure(c, s, 'anvil')
end
