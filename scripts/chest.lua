local action = require('outpost.action')
local structure_items = require('structure_items')
local tools = require('tools')
local ward = require('ward')

function action.use.chest(c, s)
    if not ward.check(c, s:pos()) then
        return
    end

    c:open_container(c:pawn():inventory('main'),
                     s:inventory('contents'))
end

function action.use_item.chest(c, inv)
    structure_items.use_item(c, inv, 'chest', 'chest')
end

function tools.handler.axe.chest(c, s, inv)
    -- TODO: do something with the chest contents
    structure_items.use_structure(c, s, 'chest')
end