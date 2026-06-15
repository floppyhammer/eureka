use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::SystemTime;

pub struct Time {
    startup_time: SystemTime,

    last_frame_time: SystemTime,
    /// Frame time.
    delta: f64,
    /// Approximate FPS over the last second.
    fps: f32,

    last_time_updated_fps: SystemTime,

    // Profiling stats in nanoseconds
    pub logic_time: Arc<AtomicU64>,
    pub render_cpu_time: Arc<AtomicU64>,
    pub gpu_time: Arc<AtomicU64>,
}

impl Time {
    pub fn new() -> Self {
        Self {
            startup_time: SystemTime::now(),
            last_frame_time: SystemTime::now(),
            delta: 0.0,
            fps: 0.0,
            last_time_updated_fps: SystemTime::now(),
            logic_time: Arc::new(AtomicU64::new(0)),
            render_cpu_time: Arc::new(AtomicU64::new(0)),
            gpu_time: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn tick(&mut self) {
        let now = SystemTime::now();

        match self.last_frame_time.elapsed() {
            Ok(elapsed) => {
                self.delta = elapsed.as_secs_f64();
            }
            Err(e) => {
                println!("Error: {:?}", e);
            }
        }

        if self.last_time_updated_fps.elapsed().unwrap().as_secs_f64() > 1.0 {
            self.last_time_updated_fps = now;
            self.fps = 1.0 / self.delta as f32;

            let logic_ms = self.logic_time.load(Ordering::Relaxed) as f64 / 1_000_000.0;
            let render_cpu_ms = self.render_cpu_time.load(Ordering::Relaxed) as f64 / 1_000_000.0;
            let gpu_ms = self.gpu_time.load(Ordering::Relaxed) as f64 / 1_000_000.0;

            println!(
                "FPS: {:>6.2} | CPU Logic: {:>6.2}ms | CPU Render: {:>6.2}ms | GPU: {:>6.2}ms",
                self.fps, logic_ms, render_cpu_ms, gpu_ms
            );
        }

        self.last_frame_time = now;
    }

    pub fn get_delta(&self) -> f64 {
        return self.delta;
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
        self.fps
    }
}
