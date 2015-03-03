var simple = require('graphics/draw/simple');
var SpriteBase = require('graphics/renderer').SpriteBase;
var Animation = require('graphics/sheet').Animation;
var Vec = require('util/vec').Vec;

var $ = function(x) { return document.getElementById(x); }



var anim_dirs = [
    // Start facing in +x, then cycle toward +y (clockwise, since y points
    // downward).
    {idx: 2, flip: false},
    {idx: 3, flip: false},
    {idx: 4, flip: false},
    {idx: 3, flip: true},
    {idx: 2, flip: true},
    {idx: 1, flip: true},
    {idx: 0, flip: false},
    {idx: 1, flip: false},
];

var pony_anims = new Array(4 * anim_dirs.length);
for (var i = 0; i < anim_dirs.length; ++i) {
    var idx = anim_dirs[i].idx;
    var flip = anim_dirs[i].flip;

    pony_anims[i] = {
        i: 0,
        j: idx,
        len: 1,
        fps: 1,
        flip: flip,
    };

    for (var speed = 1; speed < 4; ++speed) {
        pony_anims[speed * anim_dirs.length + i] = {
            i: speed,
            j: idx * 6,
            len: 6,
            fps: 6 + 2 * speed,
            flip: flip,
        };
    }
}


function load_preset(dir, speed) {
    var anim = pony_anims[speed * 8 + dir];
    $('anim_i').value = anim.i;
    $('anim_j').value = anim.j;
    $('anim_len').value = anim.len;
    $('anim_fps').value = anim.fps;
    $('anim_flip').checked = anim.flip;


    var dir_x = [1, 1, 0, -1, -1, -1, 0, 1][dir];
    var dir_y = [0, 1, 1, 1, 0, -1, -1, -1][dir];

    $('move_speed_x').value = speed * 50 * dir_x;
    $('move_speed_y').value = speed * 50 * dir_y;
    update_anim();
}

$('preset_use').onclick = function() {
    load_preset(+$('preset_dir').value,
                +$('preset_speed').value);
};


$('anim_i').onchange = update_anim;
$('anim_j').onchange = update_anim;
$('anim_len').onchange = update_anim;
$('anim_fps').onchange = update_anim;
$('anim_flip').onchange = update_anim;
$('move_toggle').onchange = update_anim;
$('move_speed_x').onchange = update_anim;
$('move_speed_y').onchange = update_anim;


var ctx = $('canvas').getContext('2d');
var active_anim = new Animation();
var move_speed_x = 0;
var move_speed_y = 0;
var sheet = new Image();
sheet.src = 'assets/sprites/maresprite.png';
var sprite_base = null;
var renderer = new simple.Simple2D();


function update_anim() {
    var i = +$('anim_i').value;
    var j = +$('anim_j').value;
    var len = +$('anim_len').value;
    var fps = +$('anim_fps').value;
    var flip = +$('anim_flip').checked;
    active_anim.animate(i, j, len, fps, flip, 0);

    move_speed_x = +$('move_speed_x').value * +$('move_toggle').checked;
    move_speed_y = +$('move_speed_y').value * +$('move_toggle').checked;

    var extra = new simple.SimpleExtra(sheet);
    sprite_base = new SpriteBase(96, 96, 48, 90, extra);
}
update_anim();

$('upload').onchange = function() {
    var r = new FileReader();
    r.onload = function(e) { sheet.src = r.result; };
    r.readAsDataURL($('upload').files[0]);
};


function frame() {
    var now = Date.now();

    var SIZE = 250;

    var offset_x = Math.floor(move_speed_x * now / 1000) % SIZE;
    var offset_y = Math.floor(move_speed_y * now / 1000) % SIZE;

    var x = (SIZE * 1.5 + offset_x) % SIZE;
    var y = (SIZE * 1.5 + offset_y) % SIZE;

    var sprite = sprite_base.instantiate();
    active_anim.updateSprite(now, sprite);
    sprite.setPos(new Vec(x, y, 0));

    ctx.mozImageSmoothingEnabled = false;
    ctx.webkitImageSmoothingEnabled = false;
    ctx.imageSmoothingEnabled = false;
    ctx.fillStyle = '#2f8136';
    ctx.fillRect(0, 0, 500, 500);

    ctx.save();
    ctx.scale(2, 2);
    renderer.drawInto(ctx, [0, 0], sprite);
    ctx.restore();
}

function frameWrapper() {
    frame();
    window.requestAnimationFrame(frameWrapper);
}
window.requestAnimationFrame(frameWrapper);

console.log(simple);
