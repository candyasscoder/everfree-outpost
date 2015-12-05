local autorotate = require('outpost.lib.autorotate')
local mallet = require('outpost.lib.mallet')
local structure_items = require('outpost.lib.structure_items')
local door = require('outpost.lib.door')

local add_structure_item = structure_items.add_structure_item
local add_attachment_item = structure_items.add_attachment_item
local mallet_cycle = mallet.mallet_cycle




add_structure_item('fence', 'fence/edge/horiz')
add_structure_item('fence_tee', 'fence/tee/e')
add_structure_item('fence_post', 'fence/end/fancy/e')

mallet_cycle('fence/', {
    'edge/horiz', 'edge/vert',
    'corner/nw', 'corner/ne', 'corner/se', 'corner/sw',
})
mallet_cycle('fence/', { 'tee/e', 'tee/s', 'tee/w', 'tee/n', 'cross' })
mallet_cycle('fence/end/fancy/', { 'e', 'w' })


autorotate.add_simple_wall_item('interior_wall', 'interior_wall', false)
autorotate.add_simple_wall_item('interior_door', 'interior_wall', true)
door.make_door('interior_door', 'interior_wall/door', 'axe')
mallet_cycle('interior_wall/', {
    'edge/horiz', 'edge/vert',
    'corner/nw', 'corner/ne', 'corner/se', 'corner/sw', 
    'tee/n', 'tee/e', 'tee/s', 'tee/w',
    'cross',
})

autorotate.add_simple_wall_item('brick_wall', 'brick_wall', false)
autorotate.add_simple_wall_item('brick_door', 'brick_wall', true)
door.make_door('brick_door', 'brick_wall/door', 'pick')
mallet_cycle('brick_wall/', {
    'edge/horiz', 'edge/vert',
    'corner/nw', 'corner/ne', 'corner/se', 'corner/sw', 
    'tee/n', 'tee/e', 'tee/s', 'tee/w',
    'cross',
})

autorotate.add_simple_wall_item('wood_wall', 'wood_wall', false)
autorotate.add_simple_wall_item('wood_door', 'wood_wall', true)
door.make_door('wood_door', 'wood_wall/door', 'axe')
mallet_cycle('wood_wall/', {
    'edge/horiz', 'window/v0', 'edge/vert',
    'corner/nw', 'corner/ne', 'corner/se', 'corner/sw', 
    'tee/n', 'tee/e', 'tee/s', 'tee/w',
    'cross',
})

autorotate.add_simple_wall_item('stone_wall', 'stone_wall', false)
autorotate.add_simple_wall_item('stone_door', 'stone_wall', true)
door.make_door('stone_door', 'stone_wall/door', 'pick')
mallet_cycle('stone_wall/', {
    'edge/horiz', 'window/v0', 'window/v1', 'edge/vert',
    'corner/nw', 'corner/ne', 'corner/se', 'corner/sw', 
    'tee/n', 'tee/e', 'tee/s', 'tee/w',
    'cross',
})

autorotate.add_simple_wall_item('ruined_wall', 'ruined_wall', false)
add_structure_item('ruined_door', 'ruined_wall/door/open')
mallet_cycle('ruined_wall/', {
    'edge/horiz', 'window/v0', 'window/v1', 'edge/vert',
    'corner/nw', 'corner/ne', 'corner/se', 'corner/sw', 
    'tee/n', 'tee/e', 'tee/s', 'tee/w',
    'cross',
})

autorotate.add_simple_wall_item('cottage_wall', 'cottage_wall', false)
autorotate.add_simple_wall_item('cottage_door', 'cottage_wall', true)
door.make_door('cottage_door', 'cottage_wall/door', 'axe')
mallet_cycle('cottage_wall/', {
    'edge/horiz',
    'variant/v0', 'variant/v1', 'variant/v2',
    'window/v0', 'window/v1',
    'edge/vert',
    'corner/nw', 'corner/ne', 'corner/se', 'corner/sw', 
    'tee/n', 'tee/e', 'tee/s', 'tee/w',
    'cross',
})


local terrain_cycle = {
    'center/v0',
    'edge/n', 'corner/outer/ne', 'edge/e', 'corner/outer/se',
    'edge/s', 'corner/outer/sw', 'edge/w', 'corner/outer/nw',
    'corner/inner/nw', 'corner/inner/ne', 'corner/inner/se', 'corner/inner/sw',
}

autorotate.add_floor_item('house_floor', 'wood_floor')
mallet_cycle('wood_floor/', terrain_cycle)
autorotate.add_floor_item('road', 'road')
mallet_cycle('road/', terrain_cycle)

add_structure_item('statue', 'statue/e')
mallet_cycle('statue/', { 'e', 's', 'w', 'n' })

add_structure_item('bed')
add_structure_item('table')
add_structure_item('trophy')
add_structure_item('fountain')
add_structure_item('torch')
add_structure_item('stair', 'stair/n')



local horiz_walls = {
    ['house_wall/edge/horiz/in'] = true,
    ['house_wall/edge/horiz/out'] = true,
    ['house_wall/tee/n/in'] = true,
    ['house_wall/tee/n/out'] = true,
    ['wood_wall/edge/horiz'] = true,
    ['wood_wall/tee/n'] = true,
    ['stone_wall/edge/horiz'] = true,
    ['stone_wall/tee/n'] = true,
    ['cottage_wall/edge/horiz'] = true,
    ['cottage_wall/variant/v0'] = true,
    ['cottage_wall/variant/v1'] = true,
    ['cottage_wall/variant/v2'] = true,
    ['cottage_wall/tee/n'] = true,
}

-- NB: Other `cabinets` setup is in `object.chest`.
structure_items.attachment_map['cabinets'] = horiz_walls
structure_items.attachment_map['bookshelf/0'] = horiz_walls

add_attachment_item('bookshelf', 'bookshelf/0')


add_structure_item('wood_pillar', 'pillar/wood')
add_structure_item('stone_pillar', 'pillar/stone')
