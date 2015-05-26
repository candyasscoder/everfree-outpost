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

local structure_use_handlers = {}
function outpost_ffi.callbacks.interact(client, args)
    local s = util.hit_structure(client:pawn())
    if s ~= nil then
        local handler = get_or_noop(structure_use_handlers, s:template())
        handler(client, s, args)
    end
end

local item_use_handlers = {}
function outpost_ffi.callbacks.use_item(c, item_id, args)
    local item_type = c:world():item_id_to_name(item_id)
    if item_type == nil then
        return
    end

    local inv = c:pawn():inventory('main')
    if inv:count(item_type) == 0 then
        return
    end

    local handler = get_or_noop(item_use_handlers, item_type)
    handler(c, inv, args)
end

local ability_use_handlers = {}
function outpost_ffi.callbacks.use_ability(c, item_id, args)
    local item_type = c:world():item_id_to_name(item_id)
    if item_type == nil then
        return
    end

    local inv = c:pawn():inventory('ability')
    if inv:count(item_type) == 0 then
        return
    end

    local prefix = 'ability/'
    if item_type:sub(1, #prefix) ~= prefix then
        print('tried to use non-ability: ' .. item_type)
        return
    end
    name = item_type:sub(#prefix + 1)

    local handler = get_or_noop(ability_use_handlers, name)
    handler(c, inv, args)
end

local M = {
    use = structure_use_handlers,
    use_item = item_use_handlers,
    use_ability = ability_use_handlers,
    open_inventory = nil,
}

function outpost_ffi.callbacks.open_inventory(client)
    M.open_inventory(client)
end

return M
