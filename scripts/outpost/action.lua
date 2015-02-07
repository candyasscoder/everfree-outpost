local outpost_ffi = require('outpost_ffi')
local V3 = outpost_ffi.types.V3.table


-- Callback for user actions.

local function noop(...) end

local function get_or_noop(t, k)
    local result = t[k]
    if result == nil then
        return noop
    else
        return result
    end
end

local action_handlers = {}
function outpost_ffi.callbacks.action(client, action, arg)
    print('arg = ', arg)
    local handler = get_or_noop(action_handlers, action)
    handler(client)
end

local structure_use_handlers = {}
function action_handlers.use(client)
    local entity = client:pawn()
    local pos = entity:pos()
    -- TODO: hardcoded constants based on entity size and tile size
    local target = pos + V3.new(16, 16, 16) + entity:facing() * V3.new(32, 32, 32)
    local target_tile = target:pixel_to_tile()

    local s = client:world():find_structure_at_point(target_tile)
    if s ~= nil then
        local handler = get_or_noop(structure_use_handlers, s:template())
        handler(client, s)
    end
end

return {
    handler = action_handlers,
    use = structure_use_handlers,
}
