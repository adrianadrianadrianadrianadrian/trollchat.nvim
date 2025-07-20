local M = {}
M.enabled = false
M.toggle = function()
    M.enabled = not M.enabled
end

local function send_buffer_to_chat()
    local content = table.concat(vim.api.nvim_buf_get_lines(0, 0, -1, false), '\n')
    local buffer_name = string.gsub(string.gsub(vim.api.nvim_buf_get_name(0), '/', ''), '%.', '')
    local server_path = "'localhost:2000/" .. buffer_name .. "'"
    local cmd = "curl -X POST " .. server_path .. " -d'" .. content .. "'"
    os.execute(cmd)
end

vim.api.nvim_create_autocmd({ "BufWritePost" }, {
    callback = function()
        if M.enabled then
            send_buffer_to_chat()
        end
    end
})

return M
