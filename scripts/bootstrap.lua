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
local util = require('outpost.util')

-- No 'local' so it gets exposed to repl scripts
trigger = require('trigger')


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

    local plane = s:plane()
    local pos = s:pos()
    local w = s:world()
    s:destroy()
    s:world():create_structure(plane, pos, 'stump')
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


-- 'bookshelf' behavior
action.use['bookshelf/1'] = function(c, s)
    if not ward.check(c, s:pos()) then return end

    local inv = c:pawn():inventory('main')
    if inv:count('book') == 255 then
        return
    end

    s:replace('bookshelf/0')
    inv:update('book', 1)
end

action.use['bookshelf/2'] = function(c, s)
    if not ward.check(c, s:pos()) then return end

    local inv = c:pawn():inventory('main')
    if inv:count('book') == 255 then
        return
    end

    s:replace('bookshelf/1')
    inv:update('book', 1)
end

function action.use_item.book(c, inv)
    local s = util.hit_structure(c:pawn())
    if s == nil then return end

    local plane = s:plane()
    local pos = s:pos()
    local template = s:template()
    if template == 'bookshelf/0' then
        inv:update('book', -1)
        s:replace('bookshelf/1')
    else if template == 'bookshelf/1' then
        inv:update('book', -1)
        s:replace('bookshelf/2')
    end end
end


-- Commands
function command.handler.where(client, args)
    local pawn = client:pawn()
    local plane = pawn:plane()
    local pos = client:pawn():pos()
    client:send_message('Location: ' .. plane:name() ..
            ' (' .. plane:stable_id():id() .. '), ' ..
            pos:x() .. ', ' .. pos:y() .. ', ' .. pos:z())
end
command.help.where = '/where: Show coordinates of your current position'

local spawn_point = V3.new(32, 32, 0)
PLANE_FOREST = 'Everfree Forest'

function check_forest(client)
    if client:pawn():plane():name() ~= PLANE_FOREST then
        client:send_message("That doesn't work here.")
        return false
    else
        return true
    end
end

function command.handler.spawn(client, args)
    --if not check_forest(client) then return end
    --client:pawn():teleport(spawn_point)
    client:pawn():teleport_stable_plane(client:world():get_forest_plane(), spawn_point)
end
command.help.spawn = '/spawn: Teleport to the spawn point'

function command.handler.sethome(client, args)
    if not check_forest(client) then return end
    local home = client:pawn():pos()
    client:extra().home_pos = home
    client:send_message('Set home to ' .. tostring(home))
end
command.help.sethome = {
    '/sethome: Set custom teleport destination',
    '/home: Teleport to custom destination'
}

function command.handler.home(client, args)
    if not check_forest(client) then return end
    local home = client:extra().home_pos or spawn_point
    client:pawn():teleport(home)
end
command.help.home = command.help.sethome


no_op = function(...) end
command.handler.ignore = no_op
command.help.ignore = '/ignore <name>: Hide chat messages from named player'
command.handler.unignore = no_op
command.help.unignore = '/unignore <name>: Stop hiding chat messages from <name>'

command.handler.count = no_op
command.help.count = '/count: Show the number of players currently online'


function client_by_name(s)
    local w = World.get()
    for i = 0, 100 do
        local c = w:get_client(i)
        if c ~= nil and c:name() == s then
            return c
        end
    end
end

function command.su_handler.tp(client, args)
    x, y, z = args:match('([%d-]+) ([%d-]+) ([%d-]+)')
    if x ~= nil then
        client:pawn():teleport(V3.new(x + 0, y + 0, z + 0))
    else
        local other = client_by_name(args)
        if other == nil then
            client:send_message('No such player: ' .. args)
        else
            client:pawn():teleport_plane(other:pawn():plane(), other:pawn():pos())
        end
    end
end
command.help.tp = {
    "/tp <player>: Teleport to another player's location",
    '/tp <x> <y> <z>: Teleport to specific coordinates'
}

function command.su_handler.give(client, args)
    name, count = args:match('([^ ]+) ([%d-]+)')
    if name == nil then
        name = args
        count = 1
    end

    client:pawn():inventory('main'):update(name, count + 0)
end
command.help.give = '/give <item> [count]: Add items to your inventory'

function command.su_handler.place(client, args)
    local pawn = client:pawn()
    s, err = client:world():create_structure(pawn:plane(), util.hit_tile(pawn), args)
    if s == nil then
        client:send_message(err)
    end
end
command.help.place = '/place <structure>: Place a structure at your current location'

function command.su_handler.tribe(client, args)
    local value = {
        E = 0x00,
        P = 0x40,
        U = 0x80,
        A = 0xc0,
    }

    client:pawn():update_appearance(0xc0, value[args])
end
command.help.tribe = '/tribe [E|P|U|A]: Change the tribe of your character'


function outpost_ffi.callbacks.login(c)
    c:set_main_inventory(c:pawn():inventory('main'))
end


print('\n\nup and running')
