local action = require('core.action')
local structure_items = require('outpost.lib.structure_items')
local tools = require('outpost.lib.tools')


local TELEPORT_SETUP_DIALOG_ID = 1
local TELEPORT_DEST_DIALOG_ID = 2

function action.use.teleporter(c, s, args)
    if args == nil then
        local dests = ExtraArg.list()
        for k,_ in pairs(net_get_table(s:extra().network)) do
            dests:push(ExtraArg.str(k))
        end

        local parts = ExtraArg.map()
        parts:set('dests', dests)

        c:get_interact_args(TELEPORT_DEST_DIALOG_ID, parts)
    else
        local network = s:extra().network
        local dest_name = args:get('dest'):as_str()

        local dest_pos = net_get_dest(network, dest_name)
        c:pawn():teleport(dest_pos)
    end
end

function action.use_item.teleporter(c, inv, args)
    if not check_forest(c) then return end

    if args == nil then
        local id = c:world():item_name_to_id('teleporter')
        local parts = ExtraArg.map()
        c:get_use_item_args(id, TELEPORT_SETUP_DIALOG_ID, parts)
    else
        local name = args:get('name'):as_str()
        local network = args:get('network'):as_str()

        ok, err = net_register(network, name, c:pawn():pos())
        if not ok then
            c:send_message(err)
            return
        end

        local s = structure_items.use_item(c, inv, 'teleporter', 'teleporter')
        if s == nil then
            net_deregister(network, name)
            return
        end

        s:extra().name = name
        s:extra().network = network
    end
end

function tools.handler.pick.teleporter(c, s, inv)
    -- `s` will be destroyed if `use_structure` succeeds, so get its name and
    -- network before that happens.
    local name = s:extra().name
    local network = s:extra().network
    if structure_items.use_structure(c, s, 'teleporter') then
        net_deregister(network, name)
    end
end


function net_register(net, name, pos)
    local table = World.get():extra().teleport_networks

    if table == nil then
        table = {}
        World.get():extra().teleport_networks = table
    end

    if table[net] == nil then
        table[net] = {}
    end
    local net_table = table[net]

    if net_table[name] ~= nil then
        return false, 'That name is already in use.'
    else
        net_table[name] = pos
        return true
    end
end

function is_empty(t)
    for k,v in pairs(t) do
        return false
    end
    return true
end

function net_deregister(net, name)
    local table = World.get():extra().teleport_networks
    local net_table = table[net]
    net_table[name] = nil
    if is_empty(net_table) then
        table[net] = nil
    end
end

function net_get_table(net)
    return World.get():extra().teleport_networks[net]
end

function net_get_dest(net, name)
    local net_table = net_get_table(net)
    if net_table == nil then
        return nil
    end
    return net_table[name]
end


function action.use.dungeon_entrance(c, s)
    if s:extra().target_plane == nil then
        local p = s:world():create_plane('Dungeon')
        p:extra().exit_pos = c:pawn():pos()
        s:extra().target_plane = p:stable_id()
    end

    local entrance_pos = V3.new(128, 128, 14) * V3.new(32, 32, 32)
    c:pawn():teleport_stable_plane(s:extra().target_plane, entrance_pos)
end


function action.use.dungeon_exit(c, s)
    local p = c:pawn():plane()
    c:pawn():teleport_stable_plane(c:world():get_forest_plane(), p:extra().exit_pos)
end

