use std::f64::consts::PI;

pub struct FirstOrderFilter {
    b0: f64,
    b1: f64,
    a1: f64,
    prev_x: f64,
    prev_y: f64,
}

impl FirstOrderFilter {
    pub fn high_pass(sample_rate: f64, cutoff_frequency: f64) -> Self {
        let c = sample_rate / PI / cutoff_frequency;
        let a0i = 1.0 / (1.0 + c);

        FirstOrderFilter {
            b0: c * a0i,
            b1: -c * a0i,
            a1: (1.0 - c) * a0i,
            prev_x: 0.0,
            prev_y: 0.0,
        }
    }

    pub fn low_pass(sample_rate: f64, cutoff_frequency: f64) -> Self {
        let c = sample_rate / PI / cutoff_frequency;
        let a0i = 1.0 / (1.0 + c);

        FirstOrderFilter {
            b0: a0i,
            b1: a0i,
            a1: (1.0 - c) * a0i,
            prev_x: 0.0,
            prev_y: 0.0,
        }
    }

    pub fn tick(&mut self, x: f64) -> f64 {
        let y = self.b0 * x + self.b1 * self.prev_x - self.a1 * self.prev_y;
        self.prev_y = y;
        self.prev_x = x;
        y
    }
}
