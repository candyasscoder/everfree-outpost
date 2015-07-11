local action = require('core.action')
local structure_items = require('outpost.lib.structure_items')
local tools = require('outpost.lib.tools')
local ward = require('outpost.lib.ward')

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


function action.use.cabinets(c, s)
    if not ward.check(c, s:pos()) then
        return
    end

    c:open_container(c:pawn():inventory('main'),
                     s:inventory('contents'))
end

function action.use_item.cabinets(c, inv)
    -- nB: `attachment_map[cabinets]` is set in `object.structures`.
    structure_items.use_attachment_item(c, inv, 'cabinets', 'cabinets')
end

function tools.handler.axe.cabinets(c, s, inv)
    -- TODO: do something with the contents
    structure_items.use_structure(c, s, 'cabinets')
end
