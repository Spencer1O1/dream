mod utils;

fn main() {
    println!("{}", utils::times_three_plus_two(5.0));
    println!("{}", utils::times_three_plus_two(10.0));
    println!("{}", utils::times_three_plus_two(-2.0));
    println!("{}", utils::times_three_plus_two(2.5));
}
