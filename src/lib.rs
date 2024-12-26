use bitvec::prelude::*;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use flate2::write::DeflateDecoder;

fn read_bytes(mut cursor: u64, buffer_length: usize, mut fileref: &mut File, eof: &mut bool) -> (u64, Vec<u8>) {
    let mut buffer = vec![0u8; buffer_length];
    let n = fileref.read(&mut buffer).expect("File could not be read successfully"); // reads part of file into buffer
    cursor += n as u64;
    // println!("\nbytes have been read:\ncursor: {cursor}\nn: {n}\nbuffer_length: {buffer_length}");
    if n != buffer_length {
        *eof = true; // this would only happen if the entire buffer is not filled
    }
    return (cursor, buffer);
}

fn read_bits(mut cursor: u64, buffer_length: usize, mut fileref: &mut File, eof: &mut bool) -> (u64, BitVec<u8, Lsb0>) {
    let mut buffer = vec![0u8; buffer_length];
    let n  = fileref.read(&mut buffer).expect("File could not be read successfully"); // reads part of file into buffer
    cursor += n as u64;
    // println!("\nbits have been read:\ncursor: {cursor}\nn: {n}\nbuffer_length: {buffer_length}");
    if n != buffer_length {
        *eof = true; // this would only happen if the entire buffer is not filled
    }
    return (cursor, BitVec::from_slice(&buffer));
}

fn append_bits(cursor: u64, buffer_length: usize, fileref: &mut File, eof: &mut bool, mut bits: &mut BitVec<u8, Lsb0>) -> u64 {
    let mut output = read_bits(cursor, buffer_length, fileref, eof);
    bits.append(&mut output.1);
    return output.0;
}

fn remove_front_bits(n: u16, bits: &mut BitVec<u8, Lsb0>) {
    for _ in 0..n {
        bits.remove(0);
    }
}

fn scan_static_literal(bits: &mut BitVec<u8, Lsb0>) {
    let n: u16;
    bits.force_align();
    let byte: u8 = bits.as_raw_slice()[0];
    println!("byte = {:08b}", byte);
    match byte {
        ..0b00110000 => n = 7,
        ..0b11001000 => n = 8,
        _ => n = 9,
    }
    println!("n = {n}");
    remove_front_bits(n, bits);
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
//     (cursor, bits) = read_bits(cursor, 4, fileref, eof);
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
    (cursor, bytes) = read_bytes(cursor, 3, &mut file, &mut eof);
    if bytes[0] != 0x1F || bytes[1] != 0x8B {
        panic!("File is not a gzip file")
    } else if bytes[2] != 8 {
        panic!("File is not compressed with deflate")
    }

    (cursor, bits) = read_bits(cursor, 1, &mut file, &mut eof);
    let ftext = bits[0];    // flag can be set if file seems to be plaintext (at compressors discretion)
    let fhcrc = bits[1];    // flag set if CRC16 for file header is present before the compressed blocks
    let fextra = bits[2];   // flag set if some bytes of extra field are present (at compressors discretion)
    let fname = bits[3];    // flag set if filename is present before the compressed blocks
    let fcomment = bits[4]; // flag set if comment is present before the compressed blocks
    println!("fcomment = {fcomment}\nfname = {fname}\nfextra = {fextra}\nfhcrc = {fhcrc}\nftext = {ftext}");

    (cursor, _) = read_bytes(cursor, 6, &mut file, &mut eof); // yeah i dont know what to do with these bytes ._.

    let _ = file.seek(SeekFrom::End(-4)); // this is bad!!!!!!!!!!!!!!!!
    (_, bytes) = read_bytes(cursor, 4, &mut file, &mut eof);
    let _ = file.seek(SeekFrom::Start(cursor)); // also bad!!!!!!!!!! errors should be handled
    let isize: u32 = u32::from_le_bytes(bytes.as_slice().try_into().unwrap());
    println!("isize = {}", isize);

    let mut name: String;
    let mut comment: String;
    if fextra {
        (cursor, bytes) = read_bytes(cursor, 2, &mut file, &mut eof);
        let xlen: u16 = bytes[1] as u16 | ((bytes[2] as u16) << 8);
        let _ = read_bytes(cursor, xlen as usize, &mut file, &mut eof);
        // i dont know what to do with these either ._.
    }
    if fname {
        let mut name_bytes: Vec<u8> = Vec::new();
        (cursor, bytes) = read_bytes(cursor, 1, &mut file, &mut eof);
        while bytes[0] != 0x00 {
            name_bytes.push(bytes[0]);
            (cursor, bytes) = read_bytes(cursor, 1, &mut file, &mut eof);
        }
        name = String::from_utf8(name_bytes).unwrap();
        println!("name: {}", name);
    }
    if fcomment {
        let mut comment_bytes: Vec<u8> = Vec::new();
        (cursor, bytes) = read_bytes(cursor, 1, &mut file, &mut eof);
        while bytes[0] != 0x00 {
            comment_bytes.push(bytes[0]);
            (cursor, bytes) = read_bytes(cursor, 1, &mut file, &mut eof);
        }
        comment = String::from_utf8(comment_bytes).unwrap();
        println!("comment: {}", comment);
    }
    if fhcrc {
        let _bytes = read_bytes(cursor, 2, &mut file, &mut eof);
        // uhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhh ._.
    }

    // read header of first block
    (cursor, bits) = read_bits(cursor, 1, &mut file, &mut eof);
    let bfinal = bits[0];
    let btypea = bits[1];
    let btypeb = bits[2];
    println!("bfinal = {}", bfinal);
    match (btypea, btypeb) { // find block type
        (false, false) => println!("block has type 0b00: uncompressed"),
        (false, true) => println!("block has type 0b01: static huffman compressed"),
        (true, false) => println!("block has type 0b10: dynamic huffman compressed"),
        (true, true) => panic!("block has type 0b11: reserved (error)")
    }
    remove_front_bits(3, &mut bits);

    while bits.len() < 9 { cursor = append_bits(cursor, 1, &mut file, &mut eof, &mut bits); }
    print_bits(&bits);
    scan_static_literal(&mut bits);
    while bits.len() < 9 { cursor = append_bits(cursor, 1, &mut file, &mut eof, &mut bits); }
    print_bits(&bits);

    // (cursor, bytes) = read_bytes(cursor, 259, &mut file, &mut eof);
    // println!("{}", String::from_utf8(inflate_block(&bytes)).expect("Data could not be converted to text"));

}