pub struct ProgressIndicator {
    val: u64,
    max: u64,
    width: u16,
}

impl ProgressIndicator {
    pub fn new(max: u64, width: u16) -> ProgressIndicator {
        ProgressIndicator { val: 0, max, width }
    }

    pub fn progress(&mut self, amount: u64) {
        if (self.val + amount) > self.max {
            self.val = self.max;
        } else {
            self.val += amount;
        }
    }

    pub fn draw(&self) {
        let percent = f32::round(self.val as f32 / self.max as f32 * 100.0);
        let filled_length = (self.val as f32 / self.max as f32 * self.width as f32).round() as u16;
        let empty_length = self.width.saturating_sub(filled_length);

        let filled_bar = "█".repeat(filled_length as usize);
        let empty_bar = "░".repeat(empty_length as usize);

        print!("\x1b[2K\r{}{} {}%", filled_bar, empty_bar, percent);
        use std::io::Write;
        std::io::stdout().flush().unwrap();

        if self.val >= self.max {
            println!(); // Move to the next line when complete
        }
    }
}
