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

function action.use.tree(c, s)
    s:replace('stump')

    local extra = c:extra()
    extra.trees_kicked = (extra.trees_kicked or 0) + 1
    print("kicked " .. extra.trees_kicked .. " trees")

    local count = c:pawn():inventory('main'):update('wood', 5)
    print('got ' .. count .. ' wood')
    c:pawn():inventory('main'):update('stick', 3)
    c:pawn():inventory('main'):update('stone', 1)
    c:pawn():inventory('main'):update('anvil', 1)
    c:pawn():inventory('main'):update('chest', 1)
end

function action.handler.inventory(c, arg)
    c:open_inventory(c:pawn():inventory('main'))
end

local function place_structure(name)
    return function(client, inv)
        local entity = client:pawn()
        local pos = entity:pos()
        -- TODO: hardcoded constants based on entity size and tile size
        local target = pos + V3.new(16, 16, 16) + entity:facing() * V3.new(32, 32, 32)
        local target_tile = target:pixel_to_tile()

        s, err = client:world():create_structure(target_tile, name)
        if s ~= nil then
            inv:update(name, -1)
        end
    end
end

action.use_item.anvil = place_structure('anvil')
action.use_item.chest = place_structure('chest')


print('\n\nup and running')
