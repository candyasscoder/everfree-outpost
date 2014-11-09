/** @constructor */
function AssetLoader() {
    this.assets = {}
    this.pending = 0;
    this.loaded = 0;
}
exports.AssetLoader = AssetLoader;

AssetLoader.prototype.addImage = function(name, url, callback) {
    var img = new Image();

    var this_ = this;
    img.onload = function() {
        if (callback != null) {
            callback(img);
        }
        this_._handleAssetLoad();
    };

    img.src = url;
    if (name != null) {
        this.assets[name] = img;
    }
    this._addPendingAsset();
};

AssetLoader.prototype.addJson = function(name, url, callback) {
    this._addXhr(name, url, 'json', callback);
};

AssetLoader.prototype._addXhr = function(name, url, type, callback) {
    var elt = null;
    if (name != null) {
        elt = document.getElementById('asset-' + name);
    }

    if (elt == null) {
        var xhr = new XMLHttpRequest();
        xhr.open('GET', url, true);

        xhr.responseType = type;

        var this_ = this;
        xhr.onreadystatechange = function() {
            if (this.readyState == XMLHttpRequest.DONE) {
                if (name != null) {
                    this_.assets[name] = this.response;
                }
                if (callback != null) {
                    callback(this.response);
                }
                this_._handleAssetLoad();
            }
        };

        xhr.send();
        this._addPendingAsset();
    } else {
        var value = elt.textContent;
        if (type == 'json') {
            value = JSON.parse(value);
        }

        if (name != null) {
            this.assets[name] = value;
        }
        if (callback != null) {
            callback(value);
        }
    }
};

AssetLoader.prototype.addText = function(name, url, callback) {
    this._addXhr(name, url, 'text', callback);
};

AssetLoader.prototype._addPendingAsset = function() {
    this.pending += 1;
    this._handleProgress();
};

AssetLoader.prototype._handleAssetLoad = function() {
    this.pending -= 1;
    this.loaded += 1;
    this._handleProgress();
    if (this.pending == 0 && typeof this.onload == 'function') {
        this.onload();
    }
};

AssetLoader.prototype._handleProgress = function() {
    if (typeof this.onprogress == 'function') {
        this.onprogress(this.loaded, this.pending + this.loaded);
    }
};
