use std::io;

fn read_numbers() -> Vec<i32> {
    let mut input = String::new();

    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read input");

    input
        .trim()
        .split(',')
        .map(|s| s.trim().parse::<i32>().expect("Invalid number"))
        .collect()
}

fn main() {
    println!("Enter comma-separated numbers (e.g. 3, 1, 4):");

    let numbers = read_numbers();
    println!("{:?}", numbers);
}
