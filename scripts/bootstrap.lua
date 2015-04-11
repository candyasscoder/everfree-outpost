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
V2 = outpost_ffi.types.V2.table
World = outpost_ffi.types.World.table

Rng = outpost_ffi.types.Rng.table
GenChunk = outpost_ffi.types.GenChunk.table
ValuesMut = outpost_ffi.types.ValuesMut.table

ConstantField = outpost_ffi.types.ConstantField.table
RandomField = outpost_ffi.types.RandomField.table
FilterField = outpost_ffi.types.FilterField.table
BorderField = outpost_ffi.types.BorderField.table
DiamondSquare = outpost_ffi.types.DiamondSquare.table

IsoDiskSampler = outpost_ffi.types.IsoDiskSampler.table


require('outpost.userdata')
require('outpost.extra')
require('outpost.eval')
local action = require('outpost.action')
local command = require('outpost.command')

require('inventory')
local tools = require('tools')
require('structure_items')
local ward = require('ward')

require('chest')
require('anvil')
require('ward_item')
require('mallet')
require('hat')
require('teleporter')

require('terrain')


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

    local pos = s:pos()
    local w = s:world()
    s:destroy()
    s:world():create_structure(pos, 'stump')
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
command.help.where = '/where: Show coordinates of your current position'

local spawn_point = V3.new(32, 32, 0)

function command.handler.spawn(client, args)
    client:pawn():teleport(spawn_point)
end
command.help.spawn = '/spawn: Teleport to the spawn point'

function command.handler.sethome(client, args)
    local home = client:pawn():pos()
    client:extra().home_pos = home
    client:send_message('Set home to ' .. tostring(home))
end
command.help.sethome = {
    '/sethome: Set custom teleport destination',
    '/home: Teleport to custom destination'
}

function command.handler.home(client, args)
    local home = client:extra().home_pos or spawn_point
    client:pawn():teleport(home)
end
command.help.home = command.help.sethome


command.help.ignore = '/ignore <name>: Hide chat messages from named player'
command.help.unignore = '/unignore <name>: Stop hiding chat messages from <name>'


function outpost_ffi.callbacks.login(c)
    c:set_main_inventory(c:pawn():inventory('main'))
end


print('\n\nup and running')
