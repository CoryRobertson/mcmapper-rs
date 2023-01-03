

/*
// --fastanvil solution--

 let file = File::open("test/region/r.0.0.mca").unwrap();
    let mut region = fastanvil::Region::from_stream(file).unwrap();
    let data = region.read_chunk(13, 3).unwrap().unwrap();

    let chunk: CurrentJavaChunk = from_bytes(data.as_slice()).unwrap();

    println!("{:?}",chunk.block(4,62,15).unwrap());

*/

/*
// --simpleanvil solution--

let region = Region::from_file("test/region/r.0.0.mca".parse().unwrap());
    let chunk = region.get_chunk(13,3).unwrap();
    println!("{}", chunk.get_block(4,62,15));

 */


use std::fmt::{Display, Formatter};
use std::fs;
use std::fs::File;
use std::num::ParseIntError;
use fastanvil::{Chunk, CurrentJavaChunk};
use fastnbt::from_bytes;
use simple_anvil::region::Region;

fn main() {

    let file = File::open("test/region/r.0.0.mca").unwrap();
    let mut region = fastanvil::Region::from_stream(file).unwrap();
    let data = region.read_chunk(13, 3).unwrap().unwrap();

    let chunk: CurrentJavaChunk = from_bytes(data.as_slice()).unwrap();

    println!("{:?}",chunk.block(4,62,15).unwrap());

    let list = get_region_files();

    for region_file in list {
        println!("{}", region_file);
    }


}

#[derive(Debug)]
struct ChunkCoordinate(i32,i32);

#[derive(Debug)]
struct RegionFile {
    coordinate: ChunkCoordinate,
    file: File,
}

impl Display for ChunkCoordinate {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{},{}", self.0, self.1)
    }
}

impl Display for RegionFile {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"Coordinate: {}, File: {:?}",self.coordinate, self.file)
    }
}

fn get_region_files() -> Vec<(RegionFile)> {
    let dir = fs::read_dir("test/region").unwrap();
    let list: Vec<(RegionFile)> = dir
        .into_iter()
        .map(|file_in_dir| {
            match file_in_dir {
                Ok(f) => { Some(f) }
                Err(_) => { None }
            }
        })
        .filter_map(|file_option| file_option)
        .filter_map(|file_dir_entry| {

            let coords: Vec<i32> = file_dir_entry.file_name().to_str().unwrap().split('.')
                .filter_map(|token| {
                    match token.parse::<i32>() {
                        Ok(n) => { Some(n) }
                        Err(_) => { None }
                    }
                }).collect();

            let coord: ChunkCoordinate = ChunkCoordinate(*coords.get(0).unwrap(), *coords.get(1).unwrap());

            // println!("{:?}",coords);

            match File::open(file_dir_entry.path()) {
                Ok(f) => { Some(RegionFile{ coordinate: coord, file: f }) }
                Err(_) => { None }
            }
        })
        .collect();

    list
}
