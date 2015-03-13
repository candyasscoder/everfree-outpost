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


-- Put some type tables in global scope
V3 = outpost_ffi.types.V3.table
World = outpost_ffi.types.World.table


require('outpost.userdata')
require('outpost.extra')
require('outpost.eval')
local action = require('outpost.action')
local command = require('outpost.command')

require('inventory')
local tools = require('tools')
require('structure_items')
require('chest')
require('anvil')
require('ward_item')
local ward = require('ward')
require('mallet')


function action.open_inventory(c)
    c:open_inventory(c:pawn():inventory('main'))
end


-- 'tree' behavior
function action.use.tree(c, s)
    local count = c:pawn():inventory('main'):update('wood', 2)
end

function tools.handler.axe.tree(c, s, inv)
    if not ward.check(c, s:pos()) then
        return
    end

    s:replace('stump')
    inv:update('wood', 15)
end

function tools.handler.axe.stump(c, s, inv)
    if not ward.check(c, s:pos()) then
        return
    end

    s:destroy()
    inv:update('wood', 5)
end


-- 'rock' behavior
function action.use.rock(c, s)
    local count = c:pawn():inventory('main'):update('stone', 2)
end

function tools.handler.pick.rock(c, s, inv)
    if not ward.check(c, s:pos()) then
        return
    end

    s:destroy()
    inv:update('stone', 20)
    print(math.random())
    if math.random() < 0.2 then
        print(inv:update('crystal', 1))
    end
end


-- Commands
function command.handler.where(client, args)
    local pos = client:pawn():pos()
    local x = pos:x()
    local y = pos:y()
    client:send_message('Location: ' .. tostring(x) .. ', ' .. tostring(y))
end

local spawn_point = V3.new(32, 32, 0)

function command.handler.spawn(client, args)
    client:pawn():teleport(spawn_point)
end

function command.handler.sethome(client, args)
    local home = client:pawn():pos()
    client:extra().home_pos = home
    client:send_message('Set home to ' .. tostring(home))
end

function command.handler.home(client, args)
    local home = client:extra().home_pos or spawn_point
    client:pawn():teleport(home)
end


print('\n\nup and running')
