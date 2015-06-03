local action = require('outpost.action')
local tools = require('lib.tools')
local structure_items = require('lib.structure_items')

function add_door(item, base, tool)
    local t_open = base .. '/open'
    local t_closed = base .. '/closed'

    action.use_item[item] = function(c, inv)
        structure_items.use_item(c, inv, item, t_closed)
    end

    action.use[t_closed] = function(c, s)
        s:replace(t_open)
        -- TODO: timer to reset to t_closed
    end

    local function take(c, s, inv)
        return structure_items.use_structure(c, s, item)
    end
    tools.handler[tool][t_open] = take
    tools.handler[tool][t_closed] = take
end

return {
    add_door = add_door,
}
