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
end

function action.handler.inventory(c)
    c:open_inventory(c:pawn():inventory('main'))
end


print('\n\nup and running')
