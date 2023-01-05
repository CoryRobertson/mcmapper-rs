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

use fastanvil::{Block, Chunk, CurrentJavaChunk};
use fastnbt::from_bytes;
use image::{DynamicImage, GenericImageView, ImageBuffer, Pixel, RgbImage};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::fs;
use std::fs::File;

fn main() {
    let file = File::open("test/region/r.0.0.mca").unwrap();
    let mut region = fastanvil::Region::from_stream(file).unwrap(); // region consist of 32x32 chunk squares
    let data = region.read_chunk(13, 3).unwrap().unwrap(); // accessing the data from a region using read chunk, reach chunk here should be a number from 0..32

    let chunk: CurrentJavaChunk = from_bytes(data.as_slice()).unwrap(); // chunk conversion from bytes? should probably just be one line from the data variable later

    // x coord is a range of 0..16 in the region, so is z coord, y coord is the real y coordinate of the block in the game.
    println!("{:?}", chunk.block(4, 62, 15).unwrap()); // this block should be an oak log if map is unchanged

    let list = get_region_files("test/region");
    let texture_list = get_texture_list();
    // for region_file in &list {
    //     println!("{}", region_file);
    // }
    println!("Length of region file list: {}", list.len());

    println!(
        "Minecraft sand texture: {:?}",
        texture_list.get("minecraft:sand")
    );

    region_to_image(list.get(6).unwrap(), &texture_list);
}

type TextureListMap = HashMap<String, DynamicImage>;

/// Returns a list of all the filenames in the assets folder.
fn get_texture_list() -> TextureListMap {
    let dir = fs::read_dir("assets").unwrap();
    let list: Vec<String> = dir
        .into_iter()
        .map(|file_in_dir| match file_in_dir {
            Ok(f) => Some(f),
            Err(_) => None,
        })
        .filter_map(|file_option| file_option)
        .filter_map(|file_entry| match file_entry.file_name().to_str() {
            None => None,
            Some(str) => Some(str.to_string()),
        })
        .collect();

    let mut map: TextureListMap = HashMap::new();

    for file_name in list {
        let path = format!("assets/{}", file_name);
        // let file = File::open(path).unwrap();
        let texture_name = file_name.split('.').next().unwrap(); // take the first thing that appears before the file extension
        let minecraft_texture_name = format!("minecraft:{}", texture_name);
        let image_data = read_texture_from_texture_name(path);

        map.insert(minecraft_texture_name, image_data);
    }
    map
}

fn region_to_image(region_selected: &RegionFile, texture_list: &TextureListMap) {
    // convert this to a for loop eventually
    // let region_selected = &list.get(6).unwrap();
    let file = &region_selected.file;
    let region_coords = &region_selected.coordinate;
    let file_name = format!("r.{}-{}.png", region_coords.0, region_coords.1);

    let mut region = fastanvil::Region::from_stream(file).unwrap();
    let mut images_of_chunks: Vec<(RgbImage, usize, usize)> = vec![]; // image,x,y

    for chunk_x in 0..16 {
        for chunk_y in 0..16 {
            let data = region.read_chunk(chunk_x, chunk_y).unwrap().unwrap();
            let chunk: CurrentJavaChunk = from_bytes(data.as_slice()).unwrap();
            images_of_chunks.push(chunk_to_image(
                chunk,
                chunk_x,
                chunk_y,
                texture_list,
                region_coords,
            ));
        }
    }

    let mut img = ImageBuffer::new(4096, 4096); // 4096 = 16 chunk images * 16 chunks total

    for chunk in images_of_chunks {
        let block_x = chunk.1 * 256;
        let block_y = chunk.2 * 256;
        let pixels = chunk.0.enumerate_pixels();

        for pixel in pixels {
            let color = pixel.2.to_rgb();
            let x = pixel.0 as usize;
            let y = pixel.1 as usize;
            img.put_pixel((block_x + x) as u32, (block_y + y) as u32, color);
        }
    }

    img.save(file_name).unwrap();
    // chunk_to_image(chunk,15,0, texture_list, region_coords);
}

/// convert a chunk to an image, the chunk x and chunk y are purely for file naming only.
fn chunk_to_image(
    chunk: CurrentJavaChunk,
    chunk_x: usize,
    chunk_y: usize,
    texture_list: &TextureListMap,
    _region_coords: &ChunkCoordinate,
) -> (RgbImage, usize, usize) {
    let mut flattened_blocks: HashMap<(usize, usize), Block> = HashMap::new();
    // let file_name = format!("./output/r.{}_{}-{}.png",region_coords,chunk_x,chunk_y);
    // go through every coordinate in the given chunk, and find the highest block that is not air, add it to hash map.
    for x in 0..16 {
        for z in 0..16 {
            for y in (0..319).rev() {
                // go from top to bottom, cause top of map is most likely air and we stop when we find something.
                match chunk.block(x, y, z) {
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

    // println!("len of flat blocks: {}", flattened_blocks.len());
    // println!("{:?}", flattened_blocks.get(&(4 as usize,15 as usize)));
    let mut img: RgbImage = ImageBuffer::new(256, 256);
    // println!("flat block len: {}",flattened_blocks.len());
    for block in flattened_blocks {
        let block_x = block.0 .0 * 16;
        let block_y = block.0 .1 * 16;
        let mc_block = &block.1;
        let texture = match texture_list.get(mc_block.name()) {
            None => {
                panic!("ERROR BLOCK TEXTURE NOT FOUND FOR BLOCK: {:?}", mc_block);
            }
            Some(tex) => tex,
        };
        for pixel in texture.pixels() {
            let color = pixel.2.to_rgb();
            let x = pixel.0 as usize;
            let y = pixel.1 as usize;
            img.put_pixel((block_x + x) as u32, (block_y + y) as u32, color);
        }
    }
    //img.save(file_name).unwrap();
    (img, chunk_x, chunk_y)
}

#[derive(Debug)]
struct ChunkCoordinate(i32, i32);

#[derive(Debug)]
struct RegionFile {
    coordinate: ChunkCoordinate,
    file: File,
}

impl Display for ChunkCoordinate {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{},{}", self.0, self.1)
    }
}

impl Display for RegionFile {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Coordinate: {}, File: {:?}", self.coordinate, self.file)
    }
}

/// Get all region files contained within a directory.
fn get_region_files(path: &str) -> Vec<RegionFile> {
    let dir = fs::read_dir(path).unwrap();
    let list: Vec<RegionFile> = dir
        .into_iter()
        .map(|file_in_dir| match file_in_dir {
            Ok(f) => Some(f),
            Err(_) => None,
        })
        .filter_map(|file_option| file_option)
        .filter_map(|file_dir_entry| {
            let coords: Vec<i32> = file_dir_entry
                .file_name()
                .to_str()
                .unwrap()
                .split('.')
                .filter_map(|token| match token.parse::<i32>() {
                    Ok(n) => Some(n),
                    Err(_) => None,
                })
                .collect();

            let coord: ChunkCoordinate =
                ChunkCoordinate(*coords.get(0).unwrap(), *coords.get(1).unwrap());

            // println!("{:?}",coords);

            match File::open(file_dir_entry.path()) {
                Ok(f) => Some(RegionFile {
                    coordinate: coord,
                    file: f,
                }),
                Err(_) => None,
            }
        })
        .collect();

    list
}

fn read_texture_from_texture_name(file: String) -> DynamicImage {
    image::open(file).unwrap()
}
