// use std::env;
use std::fs;
use std::io::Read;

fn main() {
    // let path = &env::args().collect::<Vec<_>>()[1]; // set filepath to first arg
    let path = "hello.txt";
    let mut file = fs::File::open(path) // read file
        .expect("File could not be opened successfully");
    const BUFFER_LENGTH: usize = 32;
    let mut buffer = [0; BUFFER_LENGTH];
    let n  = file.read(&mut buffer).expect("File could not be read successfully");
    let mut eof = false;
    if n != BUFFER_LENGTH {
        eof = true;
    }

    println!("{}", buffer[0]);
}
