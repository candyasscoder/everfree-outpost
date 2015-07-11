local action = require('core.action')

function action.use_item.hat(c, inv)
    if c:pawn():inventory('ability'):count('ability/remove_hat') > 0 then
        return
    end

    inv:update('hat', -1)
    c:pawn():inventory('ability'):update('ability/remove_hat', 1)
    c:pawn():update_appearance(0x100, 0x100)
end

function action.use_ability.remove_hat(c, inv)
    c:pawn():inventory('ability'):update('ability/remove_hat', -1)
    c:pawn():inventory('main'):update('hat', 1)
    c:pawn():update_appearance(0x100, 0)
end
