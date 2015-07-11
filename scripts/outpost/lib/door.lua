local action = require('core.action')
local tools = require('outpost.lib.tools')
local structure_items = require('outpost.lib.structure_items')
local timer = require('outpost.ext.timer')

function make_door(item, base, tool)
    local t_open = base .. '/open'
    local t_closed = base .. '/closed'

    action.use[t_closed] = function(c, s)
        s:replace(t_open)
        s:set_timer(3000)
    end

    local function take(c, s, inv)
        return structure_items.use_structure(c, s, item)
    end
    tools.handler[tool][t_open] = take
    tools.handler[tool][t_closed] = take

    timer.handler[t_open] = function(s)
        s:replace(t_closed)
    end
end

return {
    make_door = make_door,
}
