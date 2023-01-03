

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


use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::fs;
use std::fs::File;
use std::io::BufWriter;
use std::num::ParseIntError;
use fastanvil::{Block, Chunk, CurrentJavaChunk};
use fastnbt::from_bytes;
use simple_anvil::region::Region;

fn main() {

    let file = File::open("test/region/r.0.0.mca").unwrap();
    let mut region = fastanvil::Region::from_stream(file).unwrap(); // region consist of 32x32 chunk squares
    let data = region.read_chunk(13, 3).unwrap().unwrap(); // accessing the data from a region using read chunk, reach chunk here should be a number from 0..32

    let chunk: CurrentJavaChunk = from_bytes(data.as_slice()).unwrap(); // chunk conversion from bytes? should probably just be one line from the data variable later

    // x coord is a range of 0..16 in the region, so is z coord, y coord is the real y coordinate of the block in the game.
    println!("{:?}",chunk.block(4,62,15).unwrap()); // this block should be an oak log if map is unchanged

    let list = get_region_files("test/region");

    for region_file in &list {
        println!("{}", region_file);
    }


    vec_to_image(&list);

}

fn vec_to_image(list: &Vec<RegionFile>) {

    // convert this to a for loop eventually
    let file = &list.get(6).unwrap().file;
    let mut region = fastanvil::Region::from_stream(file).unwrap();
    let data = region.read_chunk(13, 3).unwrap().unwrap();
    let chunk: CurrentJavaChunk = from_bytes(data.as_slice()).unwrap();
    let mut flattened_blocks: HashMap<(usize, usize),Block> = HashMap::new();
    // go through every coordinate in the given chunk, and find the highest block that is not air, add it to hash map.
    for x in 0..16 {
        for z in 0..16 {
            for y in (0..319).rev() { // go from top to bottom, cause top of map is most likely air and we stop when we find something.
                match chunk.block(x,y,z) {
                    Some(b) => {
                        if b.name().ne("minecraft:air") {
                            flattened_blocks.insert((x, z), b.clone());
                            break;
                        }
                    }
                    None => {}
                }
            }
        }
    }

    println!("len of flat blocks: {}", flattened_blocks.len());
    println!("{:?}", flattened_blocks.get(&(4 as usize,15 as usize)));
    // TODO: make this output an image based on this block, then make it stitch this image together with blocks

    let mut image_date: [u8 ; 1024] = [0 ; 1024];

    let file = File::create("./test.png").unwrap();
    let ref mut w = BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, 2, 1); // Width is 2 pixels and height is 1.
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_source_gamma(png::ScaledFloat::from_scaled(45455)); // 1.0 / 2.2, scaled by 100000
    encoder.set_source_gamma(png::ScaledFloat::new(1.0 / 2.2));     // 1.0 / 2.2, unscaled, but rounded
    let source_chromaticities = png::SourceChromaticities::new(     // Using unscaled instantiation here
                                                                    (0.31270, 0.32900),
                                                                    (0.64000, 0.33000),
                                                                    (0.30000, 0.60000),
                                                                    (0.15000, 0.06000)
    );
    encoder.set_source_chromaticities(source_chromaticities);
    let mut writer = encoder.write_header().unwrap();

    let data = [255, 0, 0, 255, 0, 0, 0, 255]; // An array containing a RGBA sequence. First pixel is red and second pixel is black.

    writer.write_image_data(&data).unwrap(); // Save
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

/// Get all region files contained within a directory.
fn get_region_files(path: &str) -> Vec<(RegionFile)> {
    let dir = fs::read_dir(path).unwrap();
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
