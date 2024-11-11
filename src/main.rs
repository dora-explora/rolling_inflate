// use std::env;
use std::fs::File;
use std::io::Read;
use bitvec::prelude::*;

fn read_bytes(mut cursor: u64, buffer_length: usize, mut file: &mut File, mut eof: bool) -> Vec<u8> {
    let mut buffer = vec![0u8; buffer_length];
    let n  = file.read(&mut buffer).expect("File could not be read successfully"); // reads part of file into buffer
    cursor += n as u64;
    if n != buffer_length {
        eof = true; // this would only happen if the entire buffer is not filled
    }
    return buffer;
}

fn read_bits(mut cursor: u64, buffer_length: usize, mut file: &mut File, mut eof: bool) -> BitVec<u8, Lsb0> {
    let mut buffer = vec![0u8; buffer_length];
    let n  = file.read(&mut buffer).expect("File could not be read successfully"); // reads part of file into buffer
    cursor += n as u64;
    if n != buffer_length {
        eof = true; // this would only happen if the entire buffer is not filled
    }
    let bits: BitVec<u8, Lsb0> = BitVec::from_slice(&buffer);
    return bits;
}

fn deflate_block() {

}

fn main() {
    // let path = &env::args().collect::<Vec<_>>()[1]; // set filepath to first arg
    let path = "test.txt.gz";
    let mut file = File::open(path) // open file
        .expect("File could not be opened successfully");
    let eof = false; // true if encountered the end of the file
    let cursor: u64 = 0;
    let mut buffer: Vec<u8>;
    let mut bits: BitVec<u8, Lsb0>;
    buffer = read_bytes(cursor, 3, &mut file, eof);

    if buffer[0] != 0x1F || buffer[1] != 0x8B {
        panic!("File is not a gzip file")
    } else if buffer[2] != 8 {
        panic!("File is not compressed with deflate")
    }

    bits = read_bits(cursor, 1, &mut file, eof);
    let ftext = bits[0];
    let fhcrc = bits[1];
    let fextra = bits[2];
    let fname = bits[3];
    let fcomment = bits[4];
    println!("fcomment = {fcomment}\nfname = {fname}\nfextra = {fextra}\nfhcrc = {fhcrc}\nftext = {ftext}");

    _ = read_bytes(cursor, 6, &mut file, eof); // yeah i dont know what to do with these bytes ._.

    if fextra {
        buffer = read_bytes(cursor, 2, &mut file, eof);
        let xlen: u16 = buffer[1] as u16 | ((buffer[2] as u16) << 8);
        _ = read_bytes(cursor, xlen as usize, &mut file, eof);
        // i dont know what to do with these either ._.
    }
    if fname {
        let mut name_bytes: Vec<u8> = Vec::new();
        buffer = read_bytes(cursor, 1, &mut file, eof);
        while buffer[0] != 0x00 {
            name_bytes.push(buffer[0]);
            buffer = read_bytes(cursor, 1, &mut file, eof);
        }
        let name = String::from_utf8(name_bytes).unwrap();
        println!("name: {}", name);
    }
    if fcomment {
        let mut comment_bytes: Vec<u8> = Vec::new();
        buffer = read_bytes(cursor, 1, &mut file, eof);
        while buffer[0] != 0x00 {
            comment_bytes.push(buffer[0]);
            buffer = read_bytes(cursor, 1, &mut file, eof);
        }
        let comment = String::from_utf8(comment_bytes).unwrap();
        println!("comment: {}", comment);
    }
    if fhcrc {
        _ = read_bytes(cursor, 2, &mut file, eof);
        // uhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhh ._.
    }

    bits = read_bits(cursor, 5, &mut file, eof);
    let bfinal = bits.remove(0);
    let btypea = bits.remove(0);
    let btypeb = bits.remove(0);
    println!("bfinal = {}", bfinal);
    match (btypea, btypeb) {
        (false, false) => println!("block has type 0b00: uncompressed"),
        (false, true) => println!("block has type 0b01: static huffman compressed"),
        (true, false) => println!("block has type 0b10: dynamic huffman compressed"),
        (true, true) => panic!("block has type 0b11: reserved (error)")
    }

    // let nlen_bytes = bits.split_off(16).into_vec();
    // let len_bytes = bits.into_vec();
    // println!("{:b}", len_bytes[0])
    // println!(" len: {len:b}\nnlen: {nlen:b}");
}
