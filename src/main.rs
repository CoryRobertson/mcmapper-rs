

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
use fastanvil::{Block, Chunk, CurrentJavaChunk};
use fastnbt::from_bytes;
use image::{ImageBuffer, RgbaImage, RgbImage};

fn main() {

    let file = File::open("test/region/r.0.0.mca").unwrap();
    let mut region = fastanvil::Region::from_stream(file).unwrap(); // region consist of 32x32 chunk squares
    let data = region.read_chunk(13, 3).unwrap().unwrap(); // accessing the data from a region using read chunk, reach chunk here should be a number from 0..32

    let chunk: CurrentJavaChunk = from_bytes(data.as_slice()).unwrap(); // chunk conversion from bytes? should probably just be one line from the data variable later

    // x coord is a range of 0..16 in the region, so is z coord, y coord is the real y coordinate of the block in the game.
    println!("{:?}",chunk.block(4,62,15).unwrap()); // this block should be an oak log if map is unchanged

    let list = get_region_files("test/region");
    let texture_list = get_texture_list();
    // for region_file in &list {
    //     println!("{}", region_file);
    // }
    println!("Length of region file list: {}",list.len());

    println!("Minecraft sand texture: {:?}", texture_list.get("minecraft:sand"));

    region_to_image(&list, &texture_list);

    // for texture in texture_list {
    //     println!("Texture name: \"{}\", {:?}",texture.0, texture.1);
    // }




}

type TextureListMap = HashMap<String,Vec<u8>>;

/// Returns a list of all the filenames in the assets folder.
fn get_texture_list() -> TextureListMap {
    let dir = fs::read_dir("assets").unwrap();
    let list: Vec<String> = dir.into_iter()
        .map(|file_in_dir| {
            match file_in_dir {
                Ok(f) => { Some(f) }
                Err(_) => { None }
            }
        })
        .filter_map(|file_option| file_option)
        .filter_map( |file_entry| {
            match file_entry.file_name().to_str() {
                None => { None }
                Some(str) => { Some(str.to_string()) }
            }
        })
        .collect();

    let mut map: TextureListMap = HashMap::new();

    for file_name in list {
        let file = File::open(format!("assets/{}", file_name)).unwrap();
        let texture_name = file_name.split('.').next().unwrap(); // take the first thing that appears before the file extension
        let minecraft_texture_name = format!("minecraft:{}", texture_name);
        let image_data = read_texture_into_array(file);
        map.insert(minecraft_texture_name,image_data);
    }
    map
}

fn region_to_image(list: &Vec<RegionFile>, texture_list: &TextureListMap) {
    // convert this to a for loop eventually
    let file = &list.get(6).unwrap().file;
    let mut region = fastanvil::Region::from_stream(file).unwrap();
    let data = region.read_chunk(15, 0).unwrap().unwrap();
    let chunk: CurrentJavaChunk = from_bytes(data.as_slice()).unwrap();
    let mut flattened_blocks: HashMap<(usize, usize),Block> = HashMap::new();
    // go through every coordinate in the given chunk, and find the highest block that is not air, add it to hash map.
    // FIXME: i dont think this works the way it is intended?
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

    // let mut index = 0;
    // for block in &flattened_blocks {
    //     let x = block.0.0;
    //     let y = block.0.1;
    //     let mc_block = &block.1;
    //
    //
    //     print!("{},", mc_block.name());
    //
    //     if index % 16 == 0 {
    //         println!("");
    //     }
    //
    //     index += 1;
    // }

    println!("len of flat blocks: {}", flattened_blocks.len());
    println!("{:?}", flattened_blocks.get(&(4 as usize,15 as usize)));
    // TODO: make this output an image based on this block, then make it stitch this image together with blocks

    let mut image_data: [u8 ; 262144] = [0 ; 262144]; // 16^2 for area of a block times how many blocks we are storing, in this case one chunk. so 16^2*16^2 = 65536, times 4 for RGBA

    let mut index = 0;
    // println!("{:?}",texture_list);
    // panic!("");
    println!("flat block len: {}",flattened_blocks.len());
    for block in flattened_blocks {
        let x = block.0.0;
        let y = block.0.1;
        let mc_block = &block.1;
        let texture = match texture_list.get(mc_block.name()) {
            None => {
                println!("could not find texture for: {:?}, using default instead", mc_block);
                let out: [u8 ; 1024] = [0; 1024];
                out.to_vec()
            }
            Some(tex) => {
                // FIXME: eventually remove this clone and instead have the none result use a reference instead.
                //println!("found texture for: {:?}", mc_block);
                //println!("tex len: {}", tex.len());
                tex.clone()
            }
        };
        // image_data[index] = 1;
        for a in 0..1024 {
            // println!("{} : {}", a, texture.get(a).unwrap());
            image_data[index + a] = *texture.get(a).unwrap();
        }


        index += 1024;
    }

    // let file = File::create("./test.png").unwrap();
    // let ref mut w = BufWriter::new(file);
    // let mut encoder = png::Encoder::new(w, 256, 256); // Width is 2 pixels and height is 1.
    // encoder.set_color(png::ColorType::Rgba);
    // encoder.set_depth(png::BitDepth::Eight);
    // // encoder.set_source_gamma(png::ScaledFloat::from_scaled(45455)); // 1.0 / 2.2, scaled by 100000
    // let mut writer = encoder.write_header().unwrap();
    //
    // //println!("{:?}", image_data);
    // writer.write_image_data(&image_data).unwrap(); // Save

    // let img: RgbImage = ImageBuffer::new(256,256);
    image::save_buffer("text.png", &image_data, 256, 256, image::ColorType::Rgba8).unwrap();


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
fn get_region_files(path: &str) -> Vec<RegionFile> {
    let dir = fs::read_dir(path).unwrap();
    let list: Vec<RegionFile> = dir
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

fn read_texture_into_array(file: File) -> Vec<u8> {
    //println!("{:?}", file);
    let decoder = png::Decoder::new(file);
    let mut reader = decoder.read_info().unwrap();
    // Allocate the output buffer.
    let mut buf = vec![0; reader.output_buffer_size()];
    // Read the next frame. An APNG might contain multiple frames.
    let info = reader.next_frame(&mut buf).unwrap();
    // Grab the bytes of the image.
    let bytes = &buf[..info.buffer_size()];
    // Inspect more details of the last read frame.
    let in_animation = reader.info().frame_control.is_some();

    bytes.to_vec()
}