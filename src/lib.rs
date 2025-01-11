use bitvec::prelude::*;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use flate2::write::DeflateDecoder;

fn read_bytes(mut cursorref: &mut u64, buffer_length: usize, mut fileref: &mut File, eofref: &mut bool) -> Vec<u8> {
    let mut buffer = vec![0u8; buffer_length];
    let n = fileref.read(&mut buffer).expect("File could not be read successfully"); // reads part of file into buffer
    *cursorref += n as u64;
    // println!("\nbytes have been read:\ncursor: {cursor}\nn: {n}\nbuffer_length: {buffer_length}");
    if n != buffer_length {
        *eofref = true; // this would only happen if the entire buffer is not filled
    }
    return buffer;
}

fn read_bits(mut cursorref: &mut u64, buffer_length: usize, mut fileref: &mut File, eofref: &mut bool) -> BitVec<u8, Lsb0> {
    let mut buffer = vec![0u8; buffer_length];
    let n  = fileref.read(&mut buffer).expect("File could not be read successfully"); // reads part of file into buffer
    *cursorref += n as u64;
    // println!("\nbits have been read:\ncursor: {cursor}\nn: {n}\nbuffer_length: {buffer_length}");
    if n != buffer_length {
        *eofref = true; // this would only happen if the entire buffer is not filled
    }
    return BitVec::from_slice(&buffer);
}

fn append_bits(cursorref: &mut u64, buffer_length: usize, fileref: &mut File, eofref: &mut bool, mut bitsref: &mut BitVec<u8>) {
    bitsref.append(&mut read_bits(cursorref, buffer_length, fileref, eofref));
}

fn remove_front_bits(n: u8, bitsref: &mut BitVec<u8>) {
    println!("removing {n} bits");
    for _ in 0..n {
        bitsref.remove(0);
    }
}

fn first_byte(bitsref: &BitVec<u8>) -> u8{
    let mut byte: u8 = 0;
    for i in 0..8 {
        byte |= (bitsref[7-i] as u8) << i;
    }
    return byte;
}

fn scan_uncompressed(mut cursorref: &mut u64, fileref: &mut File, eofref: &mut bool) {
    let len_bytes = read_bits(cursorref, 2, fileref, eofref).into_vec();
    let len: u16 = (len_bytes[1] as u16) << 8 | len_bytes[0] as u16;
    *cursorref += len as u64;
    let _ = (*fileref).seek(SeekFrom::Start(*cursorref));
}

fn static_code_to_literal(code: u8) -> u16 {
    match code {
        ..=0b00101111 => (code >> 1) as u16 + 256,
        ..=0b10111111 => code as u16 - 48,
        ..=0b11000101 => code as u16 - 196 + 280,
        ..=0b11000111 => panic!("what the fuckest (illegal deflate literal 286/287)"),
        _ => todo!()
    }
}

fn static_length_distance_pair(bitsref: &mut BitVec<u8, Lsb0>) -> u8 {
    println!("length-distance pair detected");
    print_bits(bitsref);
    remove_front_bits(length_extra_bits(static_code_to_literal(first_byte(bitsref) >> 1)), bitsref); // remove length bits
    remove_front_bits(length_extra_bits(static_code_to_literal(first_byte(bitsref) >> 3)), bitsref); // remove distance bits
    return 0;
}

fn length_extra_bits(literal: u16) -> u8 {
    match literal {
        ..=256 => panic!("what the fuck"),
        ..=264 => 7 + 0,
        ..=268 => 7 + 1,
        ..=272 => 7 + 2,
        ..=276 => 7 + 3,
        ..=279 => 7 + 4,
        ..=280 => 8 + 4,
        ..=284 => 8 + 5,
        ..=285 => 8 + 0,
        286.. => panic!("what the even fuck")
    }
}

fn distance_extra_bits(literal: u16) -> u8 {
    match literal {
        ..=3  => 5 + 0,
        ..=5  => 5 + 1,
        ..=7  => 5 + 2,
        ..=9  => 5 + 3,
        ..=11 => 5 + 4,
        ..=13 => 5 + 5,
        ..=15 => 5 + 6,
        ..=17 => 5 + 7,
        ..=19 => 5 + 8,
        ..=21 => 5 + 9,
        ..=23 => 5 + 10,
        ..=25 => 5 + 11,
        ..=27 => 5 + 12,
        ..=29 => 5 + 13,
        30.. => panic!("what the evener fuck")
    }
}

fn scan_static_code(bitsref: &mut BitVec<u8, Lsb0>, mut eob: &mut bool) {
    let mut n: u8 = 7;
    let byte: u8 = first_byte(bitsref);
    println!("byte: {byte:08b}");
    match byte {
        ..=0b00000001 => *eob = true,
        ..=0b00101111 => n = static_length_distance_pair(bitsref),
        ..=0b10111111 => n = 8,
        ..=0b11000111 => n = static_length_distance_pair(bitsref),
        _ => n = 9,
    }
    remove_front_bits(n, bitsref);
}

fn scan_static(cursorref: &mut u64, fileref: &mut File, eofref: &mut bool) {
    let mut eob: bool = false; // end of block
    let mut bits: BitVec<u8, Lsb0> = BitVec::new();
    while !eob {
        while bits.len() < 30 { append_bits(cursorref, 1, fileref, eofref, &mut bits); }
        scan_static_code(&mut bits, &mut eob);
    }
}

fn scan(mut blocks: &mut Vec<u64>, cursorref: &mut u64, fileref: &mut File, eofref: &mut bool) {
    let mut bfinal: bool = false;
    let mut btypea: bool;
    let mut btypeb: bool;
    while !bfinal {
        let bits = read_bits(cursorref, 1, fileref, eofref);
        bfinal = bits[0];
        btypea = bits[1];
        btypeb = bits[2];
        match (btypea, btypeb) { // find block type
            (false, false) => scan_uncompressed(cursorref, fileref, eofref),
            (false, true) => scan_static(cursorref, fileref, eofref),
            (true, false) => println!("block has type 0b10: dynamic huffman compressed"),
            (true, true) => panic!("block has type 0b11: reserved (error)")
        }
        blocks.push(cursorref.clone());
    }
}

fn print_bits(bitsref: &BitVec<u8, Lsb0>) {
    for i in 0..bitsref.len() {
        if i % 8 == 0 && i != 0{
            print!(".{:b}", bitsref[i] as u8);
        } else {
            print!("{:b}", bitsref[i] as u8);
        }
    }
    println!();
}

fn print_bytes(bytesref: &Vec<u8>) {
    for byte in bytesref {
        print!("|{:08b}", byte);
    }
    println!();
}

// fn inflate_uncompressed(mut cursorref: u64, fileref: &mut File, eofref: &mut bool) -> (u64, Vec<u8>) {
//     // inflate uncompressed block
//     let mut bits: BitVec<u8, Lsb0> = BitVec::new();
//     bits = read_bits(cursorref, 4, fileref, eofref);
//     bits.split_off(16).into_vec();
//     let len_bytes = bits.into_vec();
//     let len: u16 = (len_bytes[1] as u16) << 8 | len_bytes[0] as u16;
//     return read_bytes(cursorref, len as usize, fileref, eofref);
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

    cursor += 6;
    let _ = file.seek(SeekFrom::Start(cursor)); // this is bad!!!!!!!!!!!!!!!!

    let _ = file.seek(SeekFrom::End(-4)); // still bad!!!!!!!!!!!!!!!!
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
        let _ = read_bytes(&mut cursor, 2, &mut file, &mut eof);
        // uhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhh ._.
    }

    let mut blocks: Vec<u64> = vec![cursor]; // stores the first bytes of all deflate blocks
    scan(&mut blocks, &mut cursor, &mut file, &mut eof);

    println!("{:?}", blocks);

    // bytes = read_bytes(cursor, 259, &mut file, &mut eof);
    // println!("{}", String::from_utf8(inflate_block(&bytes)).expect("Data could not be converted to text"));

}