local outpost_ffi = require('outpost_ffi')
local util = require('outpost.util')
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
    local handler = get_or_noop(action_handlers, action)
    handler(client, arg)
end

local structure_use_handlers = {}
function action_handlers.use(client, arg)
    local s = util.hit_structure(client:pawn())
    if s ~= nil then
        local handler = get_or_noop(structure_use_handlers, s:template())
        handler(client, s)
    end
end

local item_use_handlers = {}
function action_handlers.use_item(c, arg)
    local item_type = c:world():item_id_to_name(arg)
    if item_type == nil then
        return
    end

    local inv = c:pawn():inventory('main')
    if inv:count(item_type) == 0 then
        return
    end

    local handler = get_or_noop(item_use_handlers, item_type)
    handler(c, inv)
end

return {
    handler = action_handlers,
    use = structure_use_handlers,
    use_item = item_use_handlers,
}
