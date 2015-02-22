-- Override print to output to stderr.  stdout is used for communication with
-- the server wrapper.
function print(...)
    s = ''
    for i = 1, select('#', ...) do
        x = select(i, ...)
        s = s .. tostring(x) .. '\t'
    end
    io.stderr:write(s .. '\n')
end

local function dump_rec(x, n)
    for k,v in pairs(x) do
        if type(v) == 'table' then
            print(n .. tostring(k) .. ':')
            dump_rec(v, n .. '  ')
        else
            print(n .. tostring(k) .. ': ' .. tostring(v))
        end
    end
end

local function dump(x)
    if type(x) == 'table' then
        dump_rec(x, '')
    else
        print(x)
    end
end

package.loaded.bootstrap = {
    dump = dump,
}


require('outpost.userdata')
require('outpost.extra')
require('outpost.eval')
local action = require('outpost.action')


local V3 = outpost_ffi.types.V3.table

function outpost_ffi.types.Entity.table.inventory(e, name)
    local extra = e:extra()
    local k = 'inventory_' .. name
    if extra[k] == nil then
        local i, err = e:world():create_inventory()
        i:attach_to_entity(e)
        extra[k] = i
    end
    return extra[k]
end

function outpost_ffi.types.Structure.table.inventory(s, name)
    local extra = s:extra()
    local k = 'inventory_' .. name
    if extra[k] == nil then
        local i, err = s:world():create_inventory()
        i:attach_to_structure(s)
        extra[k] = i
    end
    extra[k]:update('wood', 1)
    return extra[k]
end

function action.use.tree(c, s)
    local count = c:pawn():inventory('main'):update('wood', 2)
end

function action.use.rock(c, s)
    local count = c:pawn():inventory('main'):update('stone', 2)
end

function action.use.chest(c, s)
    c:open_container(c:pawn():inventory('main'),
                     s:inventory('contents'))
end

function action.use.anvil(c, s)
    c:open_crafting(s, c:pawn():inventory('main'))
end

function action.handler.inventory(c, arg)
    c:open_inventory(c:pawn():inventory('main'))
end

local function hit_tile(entity)
    local pos = entity:pos()
    -- TODO: hardcoded constants based on entity size and tile size
    local target = pos + V3.new(16, 16, 16) + entity:facing() * V3.new(32, 32, 32)
    return target:pixel_to_tile()
end

local function hit_structure(entity)
    return entity:world():find_structure_at_point(hit_tile(entity))
end

local function place_structure(name)
    return function(client, inv)
        local target_tile = hit_tile(client:pawn())
        -- TODO: HACK
        if name == 'anvil' or name == 'chest' then
            local x = target_tile:x()
            local y = target_tile:y()
            if x >= -64 and x < 64 and y >= -64 and x < 64 then
                return
            end
        end

        s, err = client:world():create_structure(target_tile, name)
        if s ~= nil then
            inv:update(name, -1)
            s:attach_to_chunk()
        end
    end
end

local function take_structure(name)
    return function(client, s)
        -- TODO: HACK
        if name == 'anvil' or name == 'chest' then
            local x = target_tile:x()
            local y = target_tile:y()
            if x >= -64 and x < 64 and y >= -64 and x < 64 then
                return
            end
        end

        local inv = client:pawn():inventory('main')

        if inv:count(name) < 255 then
            s:destroy()
            inv:update(name, 1)
        end
    end
end

action.use_item.anvil = place_structure('anvil')
action.use_item.chest = place_structure('chest')

for _, side in ipairs({'n', 's', 'w', 'e', 'nw', 'ne', 'sw', 'se'}) do
    name = 'house_wall/' .. side
    action.use_item[name] = place_structure(name)
    action.use[name] = take_structure(name)
end

action.use_item['house_floor'] = place_structure('house_floor')
action.use['house_floor'] = take_structure('house_floor')

function action.use_item.axe(client, inv)
    local s = hit_structure(client:pawn())
    if s == nil then
        return
    end

    local template = s:template()
    if template == 'tree' then
        s:replace('stump')
        inv:update('wood', 15)
    elseif template == 'stump' then
        s:destroy()
        inv:update('wood', 5)
    -- TODO: HACK
    elseif template == 'chest' then
        s:destroy()
        inv:update('chest', 1)
    end
end

function action.use_item.pick(client, inv)
    local s = hit_structure(client:pawn())
    if s == nil then
        return
    end

    local template = s:template()
    if template == 'rock' then
        s:destroy()
        inv:update('stone', 20)
    -- TODO: HACK
    elseif template == 'anvil' then
        s:destroy()
        inv:update('anvil', 1)
    end
end

print('\n\nup and running')
