use std::time::{Duration, SystemTime};

pub struct Engine {
    startup_time: SystemTime,

    last_frame_time: SystemTime,
    /// Frame time.
    delta: f64,
    /// Approximate FPS over the last second.
    fps: f32,

    last_time_updated_fps: SystemTime,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            startup_time: SystemTime::now(),
            last_frame_time: SystemTime::now(),
            delta: 0.0,
            fps: 0.0,
            last_time_updated_fps: SystemTime::now(),
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
