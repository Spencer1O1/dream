mod utils;

use utils::times_three_plus_two;

fn main() {
    println!("{}", times_three_plus_two(5_i32));
    println!("{}", times_three_plus_two(10_i32));
    println!("{}", times_three_plus_two(-2_i32));
    println!("{}", times_three_plus_two(2.5_f64));
}
