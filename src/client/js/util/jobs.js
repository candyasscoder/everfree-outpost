/** @constructor */
function BackgroundJobRunner() {
    // (jobs_cur, jobs_new) form a standard "queue from two stacks" data
    // structure.  New items are pushed into `jobs_new`; old items are popped
    // from `jobs_cur`.
    this.jobs_cur = [];
    this.jobs_new = [];
    // `subjobs` is a list of subjobs that were created by running the current
    // job.  When the current job finishes, `subjobs` will be reversed and
    // appended to `jobs_cur` (meaning subjobs automatically cut to the front
    // of the queue).
    this.subjobs = [];
    this.current_job_name = null;
    this.subjob_count = 0;

    this._message_pending = false;

    var this_ = this;
    window.addEventListener('message', function(evt) {
        if (evt.origin != window.location.origin || evt.data != 'run_jobs') {
            return;
        }
        this_._handleMessage();
    });
}
exports.BackgroundJobRunner = BackgroundJobRunner;

BackgroundJobRunner.prototype._sendMessage = function() {
    if (this._message_pending) {
        return;
    }
    this._message_pending = true;
    window.postMessage('run_jobs', window.location.origin);
};

BackgroundJobRunner.prototype._handleMessage = function() {
    this._message_pending = false;

    var had_job = this.runOne();
    if (had_job) {
        this._sendMessage();
    }
};

BackgroundJobRunner.prototype.job = function(name, cb) {
    var args = Array.prototype.slice.call(arguments, 2);
    this.jobs_new.push({ name: name, cb: cb, args: args });
    this._sendMessage();
};

BackgroundJobRunner.prototype.subjob = function(name, cb) {
    console.assert(this.current_job_name != null);
    var args = Array.prototype.slice.call(arguments, 2);
    var full_name = this.current_job_name + '/' + name;
    this.subjobs.push({ name: full_name, cb: cb, args: args });
    this._sendMessage();
};

BackgroundJobRunner.prototype.runOne = function() {
    if (this.jobs_cur.length == 0) {
        while (this.jobs_new.length > 0) {
            this.jobs_cur.push(this.jobs_new.pop());
        }
        if (this.jobs_cur.length == 0) {
            return false;
        }
    }

    var job = this.jobs_cur.pop();
    if (this.subjob_count > 0) {
        --this.subjob_count;
    }
    this.current_job_name = job.name;
    try {
        job.cb.apply(this, job.args);
    } finally {
        this.current_job_name = null;
        this.subjob_count += this.subjobs.length;
        while (this.subjobs.length > 0) {
            this.jobs_cur.push(this.subjobs.pop());
        }
    }
    return true;
};

BackgroundJobRunner.prototype.count = function() {
    var total = this.jobs_cur.length + this.jobs_new.length;
    return [total - this.subjob_count, this.subjob_count];
};
