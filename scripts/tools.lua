local action = require('outpost.action')
local util = require('outpost.util')

local handlers = {}

local function add_tool(name)
    handlers[name] = {}

    action.use_item[name] = function(c, inv)
        local s = util.hit_structure(c:pawn())
        local template = '_'
        if s ~= nil then
            template = s:template()
        end

        local handler = handlers[name][template]
        if handler ~= nil then
            handler(c, s, inv)
        end
    end
end

add_tool('axe')
add_tool('pick')

return {
    -- Hooks for handling particular tool-structure interactions.  Define a
    -- function such as 'tools.handler.axe.chest' to be called when the 'axe'
    -- item is used on the 'chest' structure.  (The structure type may also be
    -- '_', to handle using the tool on no structure.)
    handler = handlers,
}
