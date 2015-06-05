local action = require('outpost.action')
local util = require('outpost.util')

function action.use_item.shovel(c, s)
    local pawn = c:pawn()
    local plane = pawn:plane()
    local pos = util.hit_tile(pawn)
    local b = plane:get_block(pos)

    if b:startswith('grass/center/') then
        plane:set_interior(pos, 'farmland', 1)
    else if b:startswith('farmland/') then
        -- TODO: currently broken.  need a way to specify what to set the target tile back to.
        --plane:set_interior(pos, 'farmland', 0)
    end end
end
