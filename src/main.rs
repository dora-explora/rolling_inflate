use std::env;
use std::fs;

fn main() {
    let path = &env::args().collect::<Vec<_>>()[1]; // set filepath to first arg
    let contents = fs::read_to_string(path) // read file
        .expect("File could not be read successfully");

    println!("{}", contents);
}
