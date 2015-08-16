

function outpost_ffi.callbacks.apply_structure_extra(s, k, v)
    if k == 'loot' then
        local space = v:find(':')
        local item = v:sub(1, space - 1)
        local count = v:sub(space + 1)
        s:inventory('contents'):update(item, 0 + count)
    end
end


