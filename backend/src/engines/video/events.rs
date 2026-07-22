use std::time::Instant;

#[derive(Debug, Clone)]
pub struct EventMeta {
    pub timestamp: Instant,
}

impl EventMeta {
    pub fn now() -> Self {
        Self { timestamp: Instant::now() }
    }
}

#[derive(Debug, Clone)]
pub enum VideoEvent {
    // 生命周期
    ChromiumStarted {
        pid: u32,
        meta: EventMeta,
    },
    CdpConnected {
        ws_url: String,
        meta: EventMeta,
    },
    PageLoaded {
        url: String,
        final_url: Option<String>,
        meta: EventMeta,
    },
    HooksInjected {
        hook_count: u32,
        meta: EventMeta,
    },

    // 播放器发现
    VideoElementDiscovered {
        selector: String,
        count: u32,
        meta: EventMeta,
    },
    PlayerIdentified {
        platform: String,
        player_type: String,
        meta: EventMeta,
    },

    // 播放生命周期
    PlayStarted {
        player_id: Option<String>,
        video_src: Option<String>,
        meta: EventMeta,
    },
    PlayPaused {
        player_id: Option<String>,
        meta: EventMeta,
    },
    PlayResumed {
        player_id: Option<String>,
        meta: EventMeta,
    },
    PlayEnded {
        player_id: Option<String>,
        meta: EventMeta,
    },
    Seek {
        from_sec: f64,
        to_sec: f64,
        meta: EventMeta,
    },

    // 缓冲
    BufferStarted {
        player_id: Option<String>,
        meta: EventMeta,
    },
    BufferEnded {
        player_id: Option<String>,
        duration_ms: f64,
        meta: EventMeta,
    },

    // 质量变化（CDP Media 域）
    ResolutionChanged {
        width: u32,
        height: u32,
        meta: EventMeta,
    },
    BitrateChanged {
        video_kbps: f64,
        audio_kbps: f64,
        meta: EventMeta,
    },
    DroppedFramesChanged {
        dropped: u64,
        decoded: u64,
        meta: EventMeta,
    },
    FpsChanged {
        fps: f64,
        meta: EventMeta,
    },
    CodecDetected {
        video_codec: String,
        audio_codec: String,
        mime_type: String,
        meta: EventMeta,
    },

    // 网络分片
    SegmentLoaded {
        url: String,
        host: String,
        size_bytes: u64,
        meta: EventMeta,
    },
    CdnDetected {
        host: String,
        cdn_node: String,
        meta: EventMeta,
    },
    BytesReceived {
        bytes: u64,
        meta: EventMeta,
    },

    // 错误
    VideoError {
        error_type: String,
        message: String,
        meta: EventMeta,
    },
    JsConsoleError {
        text: String,
        meta: EventMeta,
    },

    // 最终指标
    MetricsFinalized {
        play_detected: bool,
        first_play_time_ms: f64,
        total_buffer_count: u32,
        total_buffer_time_ms: f64,
        meta: EventMeta,
    },
}
