(function() {

var $ = document.getElementById.bind(document);


var canvas = document.createElement('canvas');
document.body.appendChild(canvas);
canvas.width = canvas.clientWidth;
canvas.height = canvas.clientHeight;

var ctx = canvas.getContext('2d');

var animating = false;

function frameWrapper() {
    frame();
    if (animating) {
        window.requestAnimationFrame(frameWrapper);
    }
}

function startAnimation() {
    animating = true;
    window.requestAnimationFrame(frameWrapper);
}

function stopAnimation() {
    animating = false;
}


function Sheet(image, item_width, item_height) {
    this.image = image;
    this.item_width = item_width;
    this.item_height = item_height;
}

Sheet.prototype = {
    'drawInto': function(ctx, i, j, x, y) {
        ctx.drawImage(this.image,
                j * this.item_width,
                i * this.item_height,
                this.item_width,
                this.item_height,
                x,
                y,
                this.item_width,
                this.item_height);
    },
};


function LayeredSheet(images, item_width, item_height) {
    this.images = images;
    this.item_width = item_width;
    this.item_height = item_height;
}

LayeredSheet.prototype = {
    'drawInto': function(ctx, i, j, x, y) {
        for (var idx = 0; idx < this.images.length; ++idx) {
            ctx.drawImage(this.images[idx],
                    j * this.item_width,
                    i * this.item_height,
                    this.item_width,
                    this.item_height,
                    x,
                    y,
                    this.item_width,
                    this.item_height);
        }
    },
};


function AssetLoader() {
    this.assets = {}
    this.pending = 0;
    this.loaded = 0;
}

AssetLoader.prototype = {
    'addImage': function(name, url) {
        var img = new Image();

        var this_ = this;
        img.onload = function() { this_._handleAssetLoad(); };

        img.src = url;
        this._addPendingAsset(name, img);
    },

    '_addPendingAsset': function(name, asset) {
        this.assets[name] = asset;
        this.pending += 1;
    },

    '_handleAssetLoad': function() {
        this.pending -= 1;
        this.loaded += 1;
        if (typeof this.onprogress == 'function') {
            this.onprogress(this.loaded / (this.pending + this.loaded));
        }
        if (this.pending == 0 && typeof this.onload == 'function') {
            this.onload();
        }
    },
};


function Entity(sheet, x, y) {
    this.sheet = sheet;
    this._motion = {
        'last_x': x,
        'last_y': y,
        'velocity_x': 0,
        'velocity_y': 0,
        'start': 0,
    };
    this._anim = null;
}

Entity.prototype = {
    'animate': function(i, j, len, fps, flip, now) {
        this._anim = {
            'i': i,
            'j': j,
            'len': len,
            'fps': fps,
            'flip': flip,
            'start': now,
        };
    },

    'move': function(vx, vy, now) {
        var pos = this.position(now);
        this._motion = {
            'last_x': pos.x,
            'last_y': pos.y,
            'velocity_x': vx,
            'velocity_y': vy,
            'start': now,
        };
    },

    'position': function(now) {
        var motion = this._motion;
        var delta = now - motion.start;
        var x = motion.last_x + Math.floor(delta * motion.velocity_x / 1000);
        var y = motion.last_y + Math.floor(delta * motion.velocity_y / 1000);
        return { 'x': x, 'y': y }
    },

    'drawInto': function(ctx, now) {
        var pos = this.position(now);
        var x = pos.x;
        var y = pos.y;

        var anim = this._anim;
        if (anim.flip) {
            ctx.scale(-1, 1);
            x = -x - this.sheet.item_width;
        }
        var frame = Math.floor((now - anim.start) * anim.fps / 1000) % anim.len;
        this.sheet.drawInto(ctx, anim.i, anim.j + frame, x, y);
        if (anim.flip) {
            ctx.scale(-1, 1);
        }
    },
};


function Pony(sheet, x, y) {
    this._entity = new Entity(sheet, x, y);
    this._entity.animate(0, 2, 1, 1, false, 0);
    this._last_dir = { 'x': 1, 'y': 0 };
}

Pony.prototype = {
    'walk': function(now, speed, dx, dy) {
        if (dx != null && dy != null) {
            this._last_dir = { 'x': dx, 'y': dy };
        } else {
            dx = this._last_dir.x;
            dy = this._last_dir.y;
        }

        var entity = this._entity;
        var flip = dx < 0;
        // Direction, in [0..4].  0 = north, 2 = east, 4 = south.  For western
        // directions, we use [1..3] but also set `flip`.
        var dir = (2 - Math.abs(dx)) * dy + 2;

        if (speed == 0) {
            entity.animate(0, dir, 1, 1, flip, now);
        } else {
            entity.animate(speed, 6 * dir, 6, 6 + 2 * speed, flip, now);
        }

        var pixel_speed = 30 * speed;
        entity.move(dx * pixel_speed, dy * pixel_speed, now);
    },

    'position': function(now) {
        return this._entity.position(now);
    },

    'drawInto': function(ctx, now) {
        this._entity.drawInto(ctx, now);
    },
};



var loader = new AssetLoader();
loader.addImage('pony_f_base', 'assets/maresprite.png');
loader.addImage('pony_f_eyes_blue', 'assets/type1blue.png');
loader.addImage('pony_f_horn', 'assets/marehorn.png');
loader.addImage('pony_f_wing_front', 'assets/frontwingmare.png');
loader.addImage('pony_f_wing_back', 'assets/backwingmare.png');
loader.addImage('pony_f_mane_1', 'assets/maremane1.png');
loader.addImage('pony_f_tail_1', 'assets/maretail1.png');
var assets = loader.assets;
window.assets = assets;

function bake_sprite_sheet() {
    var width = assets.pony_f_base.width;
    var height = assets.pony_f_base.height;

    var temp_canvas = document.createElement('canvas');
    temp_canvas.width = width;
    temp_canvas.height = height;
    var temp_ctx = temp_canvas.getContext('2d');

    var canvas = document.createElement('canvas');
    canvas.width = width;
    canvas.height = height;
    var ctx = canvas.getContext('2d');

    function copy(img) {
        ctx.drawImage(img, 0, 0);
    }

    function tinted(img, color) {
        temp_ctx.globalCompositeOperation = 'copy';
        temp_ctx.drawImage(img, 0, 0);

        temp_ctx.globalCompositeOperation = 'source-in';
        temp_ctx.fillStyle = color;
        temp_ctx.fillRect(0, 0, width, height);

        temp_ctx.globalCompositeOperation = 'multiply';
        temp_ctx.drawImage(img, 0, 0);

        ctx.drawImage(temp_canvas, 0, 0);
    }

    var coat_color = '#c8f';
    var hair_color = '#84c';
    tinted(assets.pony_f_wing_back, coat_color);
    tinted(assets.pony_f_base, coat_color);
    copy(assets.pony_f_eyes_blue);
    tinted(assets.pony_f_mane_1, hair_color);
    tinted(assets.pony_f_tail_1, hair_color);
    tinted(assets.pony_f_horn, coat_color);
    tinted(assets.pony_f_wing_front, coat_color);

    return canvas;
}

var sheet;
var pony;

var start_time = Date.now();
loader.onload = function() {
    sheet = new Sheet(bake_sprite_sheet(), 96, 96);
    pony = new Pony(sheet, 100, 100);
    window.pony = pony;

    document.body.removeChild($('banner-bg'));
    start_time = Date.now();
    startAnimation();
};

ctx.fillStyle = '#888';

function frame() {
    var now = Date.now();
    var pos = pony.position(now);
    ctx.clearRect(pos.x, pos.y, sheet.item_width, sheet.item_height);
    pony.drawInto(ctx, now);
}

})();
