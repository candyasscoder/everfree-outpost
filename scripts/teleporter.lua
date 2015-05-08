local action = require('outpost.action')
local structure_items = require('structure_items')
local tools = require('tools')

function action.use.teleporter(c, s)
    c:pawn():teleport(s:extra().destination)
end

function action.use_item.teleporter(c, inv)
    if not check_forest(c) then return end

    local home = c:extra().home_pos
    if home == nil then
        c:send_message('Must /sethome before placing teleporter')
        return
    end
    local s = structure_items.use_item(c, inv, 'teleporter', 'teleporter')
    s:extra().destination = home
end

function tools.handler.pick.teleporter(c, s, inv)
    structure_items.use_structure(c, s, 'teleporter')
end


function action.use.dungeon_entrance(c, s)
    if s:extra().target_plane == nil then
        s:extra().target_plane = s:world():create_plane('Dungeon'):stable_id()
    end

    c:pawn():teleport_stable_plane(s:extra().target_plane, V3.new(256, 256, 0))
end


function action.use.dungeon_exit(c, s)
    c:pawn():teleport_stable_plane(c:world():get_forest_plane(), V3.new(32, 32, 0))
end

