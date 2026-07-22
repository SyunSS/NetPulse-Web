/// 视频轮询监控 JS — 首帧/卡顿/播放时长检测
pub fn video_monitor_inject_js() -> &'static str {
    r#"(function(){
    window.__videoMonitor = { firstFrameTime: 0, stutterCount: 0, stutterDuration: 0,
        stutterStart: 0, lastCt: 0, playStartTime: 0, hist: [], vw: 0, vh: 0,
        vdur: 0, ended: false, lastPollTime: Date.now(), firstPlayTimeMs: null, monitorStart: Date.now() };
})()"#
}

pub fn video_poll_js() -> &'static str {
    r#"JSON.stringify((function(){
        var v = document.querySelector('video');
        var m = window.__videoMonitor || {};
        if (!v) return JSON.stringify({alive:false});
        var ct = v.currentTime, t = Date.now();

        // 首帧
        if (m.firstFrameTime === 0 && !v.paused && ct > 0) {
            m.firstFrameTime = t;
            m.firstPlayTimeMs = (m.firstPlayTimeMs != null) ? m.firstPlayTimeMs : (t - (m.monitorStart || m.lastPollTime));
            m.vw = v.videoWidth; m.vh = v.videoHeight; m.vdur = v.duration;
            m.playStartTime = t; m.lastCt = ct;
        }

        // 播放开始
        if (m.playStartTime === 0 && !v.paused && ct > (m.lastCt||0) + 0.1) {
            m.playStartTime = t; m.lastCt = ct;
        }

        // 卡顿: ct 没进展
        if (m.playStartTime > 0 && !v.paused && ct > 0 &&
            Math.abs(ct - (m.lastCt||ct)) < 0.1 && m.stutterStart === 0) {
            m.stutterCount++; m.stutterStart = t;
        }

        // 卡顿结束
        if (m.stutterStart > 0 && ct > (m.lastCt||ct) + 0.3) {
            m.stutterDuration += t - m.stutterStart; m.stutterStart = 0;
        }

        m.lastCt = ct; m.lastPollTime = t;
        if (v.ended) m.ended = true;

        return JSON.stringify({
            alive:true, ct:ct, paused:v.paused, ended:v.ended, rs:v.readyState,
            ff:m.firstFrameTime, sc:m.stutterCount, sd:m.stutterDuration,
            ps:m.playStartTime, vw:m.vw, vh:m.vh, vdur:m.vdur,
            firstPlayMs: m.firstPlayTimeMs
        });
    })())"#
}
