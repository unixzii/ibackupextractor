use std::time::Instant;

pub struct PerfTimer(Instant);

impl PerfTimer {
    pub fn new() -> Self {
        Self(Instant::now())
    }

    pub fn finish(self) {
        let msg = format!("finished in {}ms", self.0.elapsed().as_millis());
        println!("\n{}", console::style(msg).dim());
    }
}

impl Default for PerfTimer {
    fn default() -> Self {
        Self::new()
    }
}
