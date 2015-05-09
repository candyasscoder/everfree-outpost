local action = require('outpost.action')

function action.use_item.hat(c, inv)
    inv:update('hat', -1)
    c:pawn():inventory('ability'):update('ability/remove_hat', 1)
    c:pawn():update_appearance(0x100, 0x100)
end

function action.use_ability.remove_hat(c, inv)
    c:pawn():inventory('ability'):update('ability/remove_hat', -1)
    c:pawn():inventory('main'):update('hat', 1)
    c:pawn():update_appearance(0x100, 0)
end
