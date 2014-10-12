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

var sheet = null;

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

var start_time = Date.now();
loader.onload = function() {
    sheet = new Sheet(bake_sprite_sheet(), 96, 96);

    document.body.removeChild($('banner-bg'));
    start_time = Date.now();
    startAnimation();
};

ctx.fillStyle = '#8cf';

function frame() {
    var delta = Date.now() - start_time;
    var frame = Math.floor(delta / 100) % 6;
    ctx.clearRect(100, 100, sheet.item_width, sheet.item_height);
    sheet.drawInto(ctx, 3, 12 + frame, 100, 100);
}

})();
