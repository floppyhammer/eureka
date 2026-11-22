use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::SystemTime;

pub struct Time {
    startup_time: SystemTime,

    last_logic_tick_time: SystemTime,
    /// Frame delta time in seconds.
    delta: f64,

    /// Logic FPS over the last second.
    logic_fps: f32,
    /// Render FPS over the last second.
    render_fps: f32,

    logic_frame_counter: u32,
    pub render_frame_count: Arc<AtomicU64>,
    last_render_frame_count_snapshot: u64,

    last_profile_log_time: SystemTime,
    profiler_string: String,

    // Profiling stats in nanoseconds
    pub logic_time: Arc<AtomicU64>,
    pub render_cpu_time: Arc<AtomicU64>,
    pub gpu_time: Arc<AtomicU64>,
}

impl Time {
    pub fn new() -> Self {
        let now = SystemTime::now();
        let default_profiler_str = "Logic FPS:   0.00 | Render FPS:   0.00 | CPU Logic:   0.00ms | CPU Render:   0.00ms | GPU:   0.00ms".to_string();
        Self {
            startup_time: now,
            last_logic_tick_time: now,
            delta: 0.0,
            logic_fps: 0.0,
            render_fps: 0.0,
            logic_frame_counter: 0,
            render_frame_count: Arc::new(AtomicU64::new(0)),
            last_render_frame_count_snapshot: 0,
            last_profile_log_time: now,
            profiler_string: default_profiler_str,
            logic_time: Arc::new(AtomicU64::new(0)),
            render_cpu_time: Arc::new(AtomicU64::new(0)),
            gpu_time: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn tick(&mut self) {
        let now = SystemTime::now();

        match self.last_logic_tick_time.elapsed() {
            Ok(elapsed) => {
                self.delta = elapsed.as_secs_f64();
            }
            Err(e) => {
                println!("Error: {:?}", e);
            }
        }
        self.last_logic_tick_time = now;
        self.logic_frame_counter += 1;

        if let Ok(elapsed) = self.last_profile_log_time.elapsed() {
            let elapsed_secs = elapsed.as_secs_f64();
            if elapsed_secs >= 0.5 {
                let current_render_frames = self.render_frame_count.load(Ordering::Relaxed);
                let render_frames_delta =
                    current_render_frames.saturating_sub(self.last_render_frame_count_snapshot);
                self.last_render_frame_count_snapshot = current_render_frames;

                self.logic_fps = (self.logic_frame_counter as f64 / elapsed_secs) as f32;
                self.render_fps = (render_frames_delta as f64 / elapsed_secs) as f32;

                self.logic_frame_counter = 0;
                self.last_profile_log_time = now;

                let logic_ms = self.logic_time.load(Ordering::Relaxed) as f64 / 1_000_000.0;
                let render_cpu_ms = self.render_cpu_time.load(Ordering::Relaxed) as f64 / 1_000_000.0;
                let gpu_ms = self.gpu_time.load(Ordering::Relaxed) as f64 / 1_000_000.0;

                self.profiler_string = format!(
                    "Logic FPS: {:>6.2} | Render FPS: {:>6.2} | CPU Logic: {:>6.2}ms | CPU Render: {:>6.2}ms | GPU: {:>6.2}ms",
                    self.logic_fps, self.render_fps, logic_ms, render_cpu_ms, gpu_ms
                );

                println!("{}", self.profiler_string);
            }
        }
    }

    pub fn get_delta(&self) -> f64 {
        self.delta
    }

    pub fn get_elapsed(&self) -> f64 {
        match self.startup_time.elapsed() {
            Ok(elapsed) => elapsed.as_secs_f64(),
            Err(e) => {
                println!("Error: {:?}", e);
                0.0
            }
        }
    }

    pub fn get_fps(&self) -> f32 {
        self.render_fps
    }

    pub fn get_logic_fps(&self) -> f32 {
        self.logic_fps
    }

    pub fn get_render_fps(&self) -> f32 {
        self.render_fps
    }

    /// 获取最新的性能分析 Profiler 日志文本字符串
    pub fn get_log_string(&self) -> &str {
        &self.profiler_string
    }

    /// 获取最新的性能分析 Profiler 日志文本字符串
    pub fn get_profiler_string(&self) -> &str {
        &self.profiler_string
    }
}
