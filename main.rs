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

fn sort_numbers(numbers: Vec<i32>) -> Vec<i32> {
    let mut sorted_numbers = numbers;
    sorted_numbers.sort();
    sorted_numbers
}

fn main() {
    println!("Enter comma-separated numbers (e.g. 1, 2, 3):");

    let numbers = read_numbers();
    let sorted_numbers = sort_numbers(numbers);

    println!("Sorted numbers: {:?}", sorted_numbers);
}
