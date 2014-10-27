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
}
exports.BackgroundJobRunner = BackgroundJobRunner;

BackgroundJobRunner.prototype.job = function(name, cb) {
    var args = Array.prototype.slice.call(arguments, 2);
    this.jobs_new.push({ name: name, cb: cb, args: args });
};

BackgroundJobRunner.prototype.subjob = function(name, cb) {
    console.assert(this.current_job_name != null);
    var args = Array.prototype.slice.call(arguments, 2);
    var full_name = this.current_job_name + '/' + name;
    this.subjobs.push({ name: full_name, cb: cb, args: args });
};

BackgroundJobRunner.prototype.run = function(start, max_dur) {
    var end = start + max_dur;
    var count = 0;
    do {
        var had_job = this.run_one();
        if (had_job) {
            ++count;
        }
    } while (had_job && Date.now() < end);
};

BackgroundJobRunner.prototype.run_one = function() {
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
        var start = Date.now();
        job.cb.apply(this, job.args);
        var end = Date.now();
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
