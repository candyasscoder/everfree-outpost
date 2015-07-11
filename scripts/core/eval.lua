local outpost_ffi = require('outpost_ffi')


function outpost_ffi.callbacks.eval(w, code)
    func, err = loadstring(code, '<repl>')

    if not func then
        return 'error parsing code: ' .. err .. '\n >>> '
    end

    local out_buf = ''
    local real_env = getfenv(0)
    local eval_env = {
        _real_env = real_env,
        print = function(...)
            s = ''
            for i = 1, select('#', ...) do
                x = select(i, ...)
                s = s .. tostring(x) .. '\t'
            end
            out_buf = out_buf .. s .. '\n'
        end,
        w = w,
    }
    setmetatable(eval_env, { __index = real_env })
    setfenv(func, eval_env)

    ok, msg = pcall(func)

    if ok then
        if msg ~= nil then
            out_buf = out_buf .. tostring(msg) .. '\n >>> '
        else
            out_buf = out_buf .. '\n >>> '
        end
    else
        out_buf = out_buf .. 'error: ' .. tostring(msg) .. '\n >>> '
    end

    return out_buf
end
