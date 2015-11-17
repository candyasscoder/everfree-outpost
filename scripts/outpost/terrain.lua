local gem_puzzle = require('outpost.gem_puzzle')

function outpost_ffi.callbacks.apply_structure_extra(s, k, v)
    if k == 'loot' then
        local pos = 1
        while pos < #v do
            local comma = v:find(',', pos)
            if comma == nil then
                break
            end

            local part = v:sub(pos, comma - 1)
            local sep = part:find(':')
            local item = part:sub(1, sep - 1)
            local count = part:sub(sep + 1)
            s:inventory('contents'):update(item, 0 + count)

            pos = comma + 1
        end
    else if k == 'gem_puzzle_slot' then
        gem_puzzle.apply_gem_puzzle_slot(s, k, v)
    else if k == 'gem_puzzle_door' then
        gem_puzzle.apply_gem_puzzle_door(s, k, v)
    end end end
end

