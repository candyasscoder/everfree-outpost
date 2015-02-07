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


function action.use.tree(c, s)
    s:replace('stump')

    local extra = c:extra()
    extra.trees_kicked = (extra.trees_kicked or 0) + 1
    print("kicked " .. extra.trees_kicked .. " trees")
end


print('\n\nup and running')
