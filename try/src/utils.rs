pub trait TimesThreePlusTwo: Sized {
    fn times_three_plus_two(self) -> Self;
}

impl TimesThreePlusTwo for i32 {
    fn times_three_plus_two(self) -> Self {
        3 * self + 2
    }
}

impl TimesThreePlusTwo for f64 {
    fn times_three_plus_two(self) -> Self {
        3.0 * self + 2.0
    }
}

pub fn times_three_plus_two<T: TimesThreePlusTwo>(x: T) -> T {
    x.times_three_plus_two()
}
