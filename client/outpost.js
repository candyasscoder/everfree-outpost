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
        if (dx != 0 || dy != 0) {
            this._last_dir = { 'x': dx, 'y': dy };
        } else {
            dx = this._last_dir.x;
            dy = this._last_dir.y;
            speed = 0;
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

loader.addImage('pony_f_base', 'assets/sprites/maresprite.png');
loader.addImage('pony_f_eyes_blue', 'assets/sprites/type1blue.png');
loader.addImage('pony_f_horn', 'assets/sprites/marehorn.png');
loader.addImage('pony_f_wing_front', 'assets/sprites/frontwingmare.png');
loader.addImage('pony_f_wing_back', 'assets/sprites/backwingmare.png');
loader.addImage('pony_f_mane_1', 'assets/sprites/maremane1.png');
loader.addImage('pony_f_tail_1', 'assets/sprites/maretail1.png');

loader.addImage('tiles1', 'assets/tiles/PathAndObjects_0.png');

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

var tileSheet = new Sheet(assets.tiles1, 32, 32);
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

var grid = [];
for (var y = 0; y < canvas.height; y += tileSheet.item_height) {
    var row = [];
    for (var x = 0; x < canvas.width; x += tileSheet.item_width) {
        row.push(false);
    }
    grid.push(row);
}

function frame() {
    var now = Date.now();
    var pos = pony.position(now);
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    var tw = tileSheet.item_width;
    var th = tileSheet.item_height;
    var rows = Math.floor((canvas.height + th - 1) / th);
    var cols = Math.floor((canvas.width + tw - 1) / tw);
    function get(ii, jj) {
        if (ii < 0 || ii >= rows || jj < 0 || jj >= cols) {
            return false;
        }
        return grid[ii][jj] ? 1 : 0;
    }
    for (var i = 0; i < rows; ++i) {
        for (var j = 0; j < cols; ++j) {
            if (!get(i, j)) {
                tileSheet.drawInto(ctx, 11, 1, j * tw, i * th);
                continue;
            }

            // This algorithm operates on the grid points between tiles instead
            // of the tiles themselves.  A grid intersection is marked as road
            // if all four tiles that touch that intersection are road.  Then
            // we choose a tile for this location based on the values of the
            // four surrounding grid points.

            var n  = get(i - 1, j);
            var ne = get(i - 1, j + 1);
            var e  = get(i    , j + 1);
            var se = get(i + 1, j + 1);
            var s  = get(i + 1, j);
            var sw = get(i + 1, j - 1);
            var w  = get(i    , j - 1);
            var nw = get(i - 1, j - 1);

            // Flags to indicate road/grass for the grid point at each corner
            // of this tile.  The grid point is road if all four of its
            // surrounding tiles are road, but we already know the current tile
            // is road, so we only need to check three other tiles for each
            // case.
            var cnw = nw + n + w == 3;
            var cne = ne + n + e == 3;
            var csw = sw + s + w == 3;
            var cse = se + s + e == 3;

            // Number of corners that are road.
            var ct = cnw + cne + csw + cse;

            var ti = null;
            var tj = null;

            if (ct == 4) {
                ti = 1;
                tj = 1;
            } else if (ct == 3) {
                if (!cnw) {
                    ti = 4;
                    tj = 1;
                } else if (!cne) {
                    ti = 4;
                    tj = 0;
                } else if (!csw) {
                    ti = 3;
                    tj = 1;
                } else if (!cse) {
                    ti = 3;
                    tj = 0;
                } else {
                    console.log('impossible case for ct == 3', cnw, cne, csw, cse);
                }
            } else if (ct == 2) {
                // The first two cases handle grass in two nonadjacent corners.
                if (cnw && cse) {
                    // not yet implemented
                } else if (cne && csw) {
                    // not yet implemented

                // For the remaining cases, we are drawing a horizontal or
                // vertical edge.
                } else if (cnw && cne) {
                    ti = 2;
                    tj = 1;
                } else if (csw && cse) {
                    ti = 0;
                    tj = 1;
                } else if (cnw && csw) {
                    ti = 1;
                    tj = 2;
                } else if (cne && cse) {
                    ti = 1;
                    tj = 0;
                } else {
                    console.log('impossible case for ct == 2', cnw, cne, csw, cse);
                }
            } else if (ct == 1) {
                if (cnw) {
                    ti = 2;
                    tj = 2;
                } else if (cne) {
                    ti = 2;
                    tj = 0;
                } else if (csw) {
                    ti = 0;
                    tj = 2;
                } else if (cse) {
                    ti = 0;
                    tj = 0;
                } else {
                    console.log('impossible case for ct == 1', cnw, cne, csw, cse);
                }
            } else if (ct == 0) {
                // The current tile is road, but enough of the surrounding
                // tiles are grass that none of the corners are road.  Draw
                // plain grass.
                ti = 11;
                tj = 1;
            } else {
                console.log('impossible value for ct', ct, cnw, cne, csw, cse);
            }

            if (ti == null || tj == null) {
                ti = 1;
                tj = 1;
            }
            tileSheet.drawInto(ctx, ti, tj, j * tw, i * th);
        }
    }

    pony.drawInto(ctx, now);
}


var dirsHeld = {
    'Up': false,
    'Down': false,
    'Left': false,
    'Right': false,
    'Shift': false,
};

document.addEventListener('keydown', function(evt) {
    if (dirsHeld.hasOwnProperty(evt.key)) {
        evt.preventDefault();
        evt.stopPropagation();
        if (!evt.repeat) {
            dirsHeld[evt.key] = true;
            updateWalkDir();
        }
    } else if (evt.key == ' ') {
        stompGrass();
    }
});

document.addEventListener('keyup', function(evt) {
    if (dirsHeld.hasOwnProperty(evt.key)) {
        evt.preventDefault();
        evt.stopPropagation();
        dirsHeld[evt.key] = false;
        updateWalkDir();
    }
});

function updateWalkDir() {
    var dx = 0;
    var dy = 0;
    var speed = 1;

    if (dirsHeld['Left']) {
        dx -= 1;
    }
    if (dirsHeld['Right']) {
        dx += 1;
    }

    if (dirsHeld['Up']) {
        dy -= 1;
    }
    if (dirsHeld['Down']) {
        dy += 1;
    }

    if (dirsHeld['Shift']) {
        speed = 3;
    }

    pony.walk(Date.now(), speed, dx, dy);
}

function stompGrass() {
    var pos = pony.position(Date.now());
    var w = tileSheet.item_width;
    var h = tileSheet.item_height;
    var base_x = Math.floor((pos.x + w) / w);
    var base_y = Math.floor((pos.y + h) / h);
    for (var i = 0; i < 2; ++i) {
        for (var j = 0; j < 2; ++j) {
            var ii = i + base_y;
            var jj = j + base_x;
            if (ii >= 0 && ii < grid.length && jj >= 0 && jj < grid[ii].length) {
                grid[ii][jj] = true;
            }
        }
    }
}

})();
