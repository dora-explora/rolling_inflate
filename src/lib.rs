use bitvec::prelude::*;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use flate2::write::DeflateDecoder;

fn read_bytes(mut cursor: &mut u64, buffer_length: usize, mut fileref: &mut File, eof: &mut bool) -> Vec<u8> {
    let mut buffer = vec![0u8; buffer_length];
    let n = fileref.read(&mut buffer).expect("File could not be read successfully"); // reads part of file into buffer
    *cursor += n as u64;
    // println!("\nbytes have been read:\ncursor: {cursor}\nn: {n}\nbuffer_length: {buffer_length}");
    if n != buffer_length {
        *eof = true; // this would only happen if the entire buffer is not filled
    }
    return buffer;
}

fn read_bits(mut cursor: &mut u64, buffer_length: usize, mut fileref: &mut File, eof: &mut bool) -> BitVec<u8, Lsb0> {
    let mut buffer = vec![0u8; buffer_length];
    let n  = fileref.read(&mut buffer).expect("File could not be read successfully"); // reads part of file into buffer
    *cursor += n as u64;
    // println!("\nbits have been read:\ncursor: {cursor}\nn: {n}\nbuffer_length: {buffer_length}");
    if n != buffer_length {
        *eof = true; // this would only happen if the entire buffer is not filled
    }
    return BitVec::from_slice(&buffer);
}

fn append_bits(cursor: &mut u64, buffer_length: usize, fileref: &mut File, eof: &mut bool, mut bits: &mut BitVec<u8>) {
    bits.append(&mut read_bits(cursor, buffer_length, fileref, eof));
}

fn remove_front_bits(n: u8, bits: &mut BitVec<u8>) {
    for _ in 0..n {
        bits.remove(0);
    }
}

fn first_byte(bits: &BitVec<u8>) -> u8{
    let mut byte: u8 = 0;
    for i in 0..8 {
        byte |= (bits[7-i] as u8) << i;
    }
    return byte;
}


fn scan_uncompressed(mut cursor: &mut u64, fileref: &mut File, eof: &mut bool) {
    *cursor -= 1;
    let _ = (*fileref).seek(SeekFrom::Start(*cursor));
    let len_bytes = read_bits(cursor, 2, fileref, eof).into_vec();
    let len: u16 = (len_bytes[1] as u16) << 8 | len_bytes[0] as u16;
    *cursor += len as u64;
}

fn scan_static_literal(bits: &mut BitVec<u8, Lsb0>, mut eob: &mut bool) {
    let mut n: u8 = 7;
    let byte: u8 = first_byte(bits);
    match byte {
        ..0b00000010 => *eob = true,
        ..0b00110000 => n = 7,
        ..0b11001000 => n = 8,
        _ => n = 9,
    }
    remove_front_bits(n, bits);
}

fn scan_static(cursor: &mut u64, fileref: &mut File, eof: &mut bool) {
    let mut eob: bool = false; // end of block
    let mut bits: BitVec<u8, Lsb0> = BitVec::new();
    while !eob {
        while bits.len() < 9 { append_bits(cursor, 1, fileref, eof, &mut bits); }
        scan_static_literal(&mut bits, &mut eob);
    }
}

fn scan(mut blocks: &mut Vec<u64>, cursor: &mut u64, fileref: &mut File, eof: &mut bool) {
    let mut bfinal: bool = false;
    let mut btypea: bool;
    let mut btypeb: bool;
    while !bfinal {
        let bits = read_bits(cursor, 1, fileref, eof);
        bfinal = bits[0];
        btypea = bits[1];
        btypeb = bits[2];
        match (btypea, btypeb) { // find block type
            (false, false) => scan_uncompressed(cursor, fileref, eof),
            (false, true) => scan_static(cursor, fileref, eof),
            (true, false) => println!("block has type 0b10: dynamic huffman compressed"),
            (true, true) => panic!("block has type 0b11: reserved (error)")
        }
        blocks.push(cursor.clone());
    }
}

fn print_bits(bits: &BitVec<u8, Lsb0>) {
    for i in 0..bits.len() {
        if i % 8 == 0 && i != 0{
            print!(".{:b}", bits[i] as u8);
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

// fn inflate_uncompressed(mut cursor: u64, fileref: &mut File, eof: &mut bool) -> (u64, Vec<u8>) {
//     // inflate uncompressed block
//     let mut bits: BitVec<u8, Lsb0> = BitVec::new();
//     bits = read_bits(cursor, 4, fileref, eof);
//     bits.split_off(16).into_vec();
//     let len_bytes = bits.into_vec();
//     let len: u16 = (len_bytes[1] as u16) << 8 | len_bytes[0] as u16;
//     return read_bytes(cursor, len as usize, fileref, eof);
// }

fn inflate_block(bufferref: &Vec<u8>) -> Vec<u8> {
    let mut inflater = DeflateDecoder::new(Vec::new());
    match inflater.write_all(bufferref) {
        Ok(ok) => ok,
        Err(error) => panic!("Error attempting to decode a block: {error}"),
    };
    match inflater.finish() {
        Ok(out) => out,
        Err(error) => panic!("Error decoding a block: {error}")
    }
}

pub fn run(path: &str) {
    let mut file = File::open(path) // open file
        .expect("File could not be opened successfully");
    let mut eof = false; // true if encountered the end of the file
    let mut cursor: u64 = 0;
    let mut bytes: Vec<u8>;
    let mut bits: BitVec<u8, Lsb0>;

    // read gzip header
    bytes = read_bytes(&mut cursor, 3, &mut file, &mut eof);
    if bytes[0] != 0x1F || bytes[1] != 0x8B {
        panic!("File is not a gzip file")
    } else if bytes[2] != 8 {
        panic!("File is not compressed with deflate")
    }

    bits = read_bits(&mut cursor, 1, &mut file, &mut eof);
    let ftext = bits[0];    // flag can be set if file seems to be plaintext (at compressors discretion)
    let fhcrc = bits[1];    // flag set if CRC16 for file header is present before the compressed blocks
    let fextra = bits[2];   // flag set if some bytes of extra field are present (at compressors discretion)
    let fname = bits[3];    // flag set if filename is present before the compressed blocks
    let fcomment = bits[4]; // flag set if comment is present before the compressed blocks
    println!("fcomment = {fcomment}\nfname = {fname}\nfextra = {fextra}\nfhcrc = {fhcrc}\nftext = {ftext}");

    _ = read_bytes(&mut cursor, 6, &mut file, &mut eof); // yeah i dont know what to do with these bytes ._.

    let _ = file.seek(SeekFrom::End(-4)); // this is bad!!!!!!!!!!!!!!!!
    bytes = read_bytes(&mut cursor, 4, &mut file, &mut eof);
    cursor -= 4;
    let _ = file.seek(SeekFrom::Start(cursor)); // also bad!!!!!!!!!! errors should be handled
    let isize: u32 = u32::from_le_bytes(bytes.as_slice().try_into().unwrap());
    println!("isize = {}", isize);

    let mut name: String;
    let mut comment: String;
    if fextra {
        bytes = read_bytes(&mut cursor, 2, &mut file, &mut eof);
        let xlen: u16 = bytes[1] as u16 | ((bytes[2] as u16) << 8);
        let _ = read_bytes(&mut cursor, xlen as usize, &mut file, &mut eof);
        // i dont know what to do with these either ._.
    }
    if fname {
        let mut name_bytes: Vec<u8> = Vec::new();
        bytes = read_bytes(&mut cursor, 1, &mut file, &mut  eof);
        while bytes[0] != 0x00 {
            name_bytes.push(bytes[0]);
            bytes = read_bytes(&mut cursor, 1, &mut file, &mut eof);
        }
        name = String::from_utf8(name_bytes).unwrap();
        println!("name: {}", name);
    }
    if fcomment {
        let mut comment_bytes: Vec<u8> = Vec::new();
        bytes = read_bytes(&mut cursor, 1, &mut file, &mut eof);
        while bytes[0] != 0x00 {
            comment_bytes.push(bytes[0]);
            bytes = read_bytes(&mut cursor, 1, &mut file, &mut eof);
        }
        comment = String::from_utf8(comment_bytes).unwrap();
        println!("comment: {}", comment);
    }
    if fhcrc {
        let _bytes = read_bytes(&mut cursor, 2, &mut file, &mut eof);
        // uhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhh ._.
    }

    let mut blocks: Vec<u64> = vec![cursor]; // stores the first bytes of all deflate blocks
    scan(&mut blocks, &mut cursor, &mut file, &mut eof);

    println!("{:?}", blocks);

    // bytes = read_bytes(cursor, 259, &mut file, &mut eof);
    // println!("{}", String::from_utf8(inflate_block(&bytes)).expect("Data could not be converted to text"));

}