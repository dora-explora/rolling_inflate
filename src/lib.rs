use bitvec::prelude::*;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

fn read_bytes(mut cursor: u64, buffer_length: usize, mut file: &mut File, mut eof: bool) -> (u64, Vec<u8>) {
    let mut buffer = vec![0u8; buffer_length];
    let n = file.read(&mut buffer).expect("File could not be read successfully"); // reads part of file into buffer
    cursor += n as u64;
    // println!("\nbytes have been read:\ncursor: {cursor}\nn: {n}\nbuffer_length: {buffer_length}");
    if n != buffer_length {
        eof = true; // this would only happen if the entire buffer is not filled
    }
    (cursor, buffer)
}

fn read_bits(mut cursor: u64, buffer_length: usize, mut file: &mut File, mut eof: bool) -> (u64, BitVec<u8, Lsb0>) {
    let mut buffer = vec![0u8; buffer_length];
    let n  = file.read(&mut buffer).expect("File could not be read successfully"); // reads part of file into buffer
    cursor += n as u64;
    // println!("\nbits have been read:\ncursor: {cursor}\nn: {n}\nbuffer_length: {buffer_length}");
    if n != buffer_length {
        eof = true; // this would only happen if the entire buffer is not filled
    }
    (cursor, BitVec::from_slice(&buffer))
}

fn print_bits(bits: &BitVec<u8, Lsb0>) {
    for i in 0..bits.len() {
        if i % 8 == 0{
            print!("|{:b}", bits[i] as u8);
        } else {
            print!("{:b}", bits[i] as u8);
        }
    }
    println!();
}

fn print_bytes(bytes: &Vec<u8>) {
    for byte in bytes {
        print!("|{:08b}", byte);
    }
    println!();
}

fn deflate_block() {

}

pub fn run(path: &str) {
    let mut file = File::open(path) // open file
        .expect("File could not be opened successfully");
    let eof = false; // true if encountered the end of the file
    let mut cursor: u64 = 0;
    let mut buffer: Vec<u8>;
    let mut bits: BitVec<u8, Lsb0>;
    let mut output: Vec<u8> = Vec::new();

    // read gzip header
    (cursor, buffer) = read_bytes(cursor, 3, &mut file, eof);
    if buffer[0] != 0x1F || buffer[1] != 0x8B {
        panic!("File is not a gzip file")
    } else if buffer[2] != 8 {
        panic!("File is not compressed with deflate")
    }

    (cursor, bits) = read_bits(cursor, 1, &mut file, eof);
    let ftext = bits[0];    // flag can be set if file seems to be plaintext (at compressors discretion)
    let fhcrc = bits[1];    // flag set if CRC16 for file header is present before the compressed blocks
    let fextra = bits[2];   // flag set if some bytes of extra field are present (at compressors discretion)
    let fname = bits[3];    // flag set if filename is present before the compressed blocks
    let fcomment = bits[4]; // flag set if comment is present before the compressed blocks
    println!("fcomment = {fcomment}\nfname = {fname}\nfextra = {fextra}\nfhcrc = {fhcrc}\nftext = {ftext}");

    (cursor, _) = read_bytes(cursor, 6, &mut file, eof); // yeah i dont know what to do with these bytes ._.

    let mut name: String;
    let mut comment: String;
    if fextra {
        (cursor, buffer) = read_bytes(cursor, 2, &mut file, eof);
        let xlen: u16 = buffer[1] as u16 | ((buffer[2] as u16) << 8);
        let _ = read_bytes(cursor, xlen as usize, &mut file, eof);
        // i dont know what to do with these either ._.
    }
    if fname {
        let mut name_bytes: Vec<u8> = Vec::new();
        (cursor, buffer) = read_bytes(cursor, 1, &mut file, eof);
        while buffer[0] != 0x00 {
            name_bytes.push(buffer[0]);
            (cursor, buffer) = read_bytes(cursor, 1, &mut file, eof);
        }
        name = String::from_utf8(name_bytes).unwrap();
        println!("name: {}", name);
    }
    if fcomment {
        let mut comment_bytes: Vec<u8> = Vec::new();
        (cursor, buffer) = read_bytes(cursor, 1, &mut file, eof);
        while buffer[0] != 0x00 {
            comment_bytes.push(buffer[0]);
            (cursor, buffer) = read_bytes(cursor, 1, &mut file, eof);
        }
        comment = String::from_utf8(comment_bytes).unwrap();
        println!("comment: {}", comment);
    }
    if fhcrc {
        let _bytes = read_bytes(cursor, 2, &mut file, eof);
        // uhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhh ._.
    }

    // read header of first member
    (cursor, bits) = read_bits(cursor, 1, &mut file, eof);
    let bfinal = bits.remove(0);
    let btypea = bits.remove(0);
    let btypeb = bits.remove(0);
    println!("bfinal = {}", bfinal);
    match (btypea, btypeb) { // find block type
        (false, false) => println!("block has type 0b00: uncompressed"),
        (false, true) => println!("block has type 0b01: static huffman compressed"),
        (true, false) => println!("block has type 0b10: dynamic huffman compressed"),
        (true, true) => panic!("block has type 0b11: reserved (error)")
    }

    let _ = file.seek(SeekFrom::End(-4)); // this is bad!!!!!!!!!!!!!!!!
    (_, buffer) = read_bytes(cursor, 4, &mut file, eof);
    let _ = file.seek(SeekFrom::Start(cursor));
    print_bytes(&buffer);
    let isize: u32 = u32::from_le_bytes(buffer.as_slice().try_into().unwrap());

    // decode uncompressed block
    (cursor, bits) = read_bits(cursor, 4, &mut file, eof);
    bits.split_off(16).into_vec();
    let len_bytes = bits.into_vec();
    let len: u16 = (len_bytes[1] as u16) << 8 | len_bytes[0] as u16;
    (cursor, output) = read_bytes(cursor, len as usize, &mut file, eof);
    println!("len: {len}");
}