local action = require('outpost.action')
local util = require('outpost.util')

function action.use_item.shovel(c, s)
    local pawn = c:pawn()
    local plane = pawn:plane()
    local pos = util.hit_tile(pawn)
    local b = plane:get_block(pos)

    if c:world():find_structure_at_point(plane, pos) ~= nil then
        -- Can't dig underneath structures.  In particular, this prevents the
        -- player from removing the farmland beneath a plant.
        return
    end

    if b:startswith('grass/center/') then
        plane:set_interior(pos, 'farmland')
    else if b:startswith('farmland/') then
        plane:clear_interior(pos, 'farmland', 'grass/center/v0')
    end end
end
