var Config = require('config').Config;

function calcScale(px) {
    var target = 1024;
    if (px < target) {
        return -Math.round(target / px);
    } else {
        return Math.round(px / target);
    }
}

function resizeCanvas(ac, w, h) {
    var scale = Config.scale_world.get() || calcScale(Math.max(w, h));
    var factor = scale > 0 ? scale : 1 / -scale;

    var virt_w = Math.ceil(w / factor);
    var virt_h = Math.ceil(h / factor);
    var phys_w = virt_w * scale;
    var phys_h = virt_h * scale;

    ac.resize(phys_w, phys_h, virt_w, virt_h);

    document.body.dataset.worldScale = factor;
}

function resizeUI(ui_div, w, h) {
    var scale = Config.scale_ui.get() || calcScale(Math.max(w, h));
    var factor = scale > 0 ? scale : 1 / -scale;

    var virt_w = Math.ceil(w / factor);
    var virt_h = Math.ceil(h / factor);

    ui_div.style.width = virt_w + 'px';
    ui_div.style.height = virt_h + 'px';
    ui_div.style.transform = 'scale(' + factor + ')';

    document.body.dataset.uiScale = factor;
}

function handleResize(anim_canvas, ui_div, w, h) {
    resizeCanvas(anim_canvas, w, h);
    resizeUI(ui_div, w, h);
}
exports.handleResize = handleResize;
