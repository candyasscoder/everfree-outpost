local gem_puzzle = require('outpost.gem_puzzle')

function outpost_ffi.callbacks.apply_structure_extra(s, k, v)
    if k == 'loot' then
        local space = v:find(':')
        local item = v:sub(1, space - 1)
        local count = v:sub(space + 1)
        s:inventory('contents'):update(item, 0 + count)
    else if k == 'gem_puzzle_slot' then
        gem_puzzle.apply_gem_puzzle_slot(s, k, v)
    else if k == 'gem_puzzle_door' then
        gem_puzzle.apply_gem_puzzle_door(s, k, v)
    end end end
end

