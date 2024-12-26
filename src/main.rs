fn main() {
    // let path = &env::args().collect::<Vec<_>>()[1]; // set filepath to first arg
    let path = "hello.txt.gz";
    rolling_inflate::run(path);
}
