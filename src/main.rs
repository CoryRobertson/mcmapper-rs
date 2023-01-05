use fastanvil::{Block, Chunk, CurrentJavaChunk};
use fastnbt::from_bytes;
use image::imageops::FilterType;
use image::{imageops, DynamicImage, GenericImageView, ImageBuffer, Pixel, RgbImage};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::fs;
use std::fs::File;

fn main() {
    let list = get_region_files("test/region");
    let texture_list = get_texture_list();
    for region_file in &list {
        println!("{}", region_file);
    }
    println!("Length of region file list: {}", list.len());

    let mut region_images: Vec<RegionImage> = vec![];
    let mut index = 1;
    for region in &list {
        let region_image = region_to_image(region, &texture_list);
        let file_name = region_file_to_file_name(region);

        region_image
            .save(format!("./output/{}", file_name))
            .unwrap();
        region_images.push(RegionImage {
            coordinate: region.coordinate,
            image: region_image,
        });
        println!("Regions processed: {}, out of: {}", index, list.len());
        index += 1;
    }

    println!("Stitching regions");
    let full_map_image = stitch_region_images(region_images);
    full_map_image
        .save("./output/all_regions_massive.png")
        .unwrap();
    let full_map_width = full_map_image.width();
    let full_map_height = full_map_image.height();

    println!("Finished stitching images, scaling image now...");

    let scaled_full_map_image = imageops::resize(
        &full_map_image,
        full_map_width / 10,
        full_map_height / 10,
        FilterType::Nearest,
    );

    scaled_full_map_image
        .save("./output/all_regions_tenth.png")
        .unwrap();
}

fn region_file_to_file_name(region: &RegionFile) -> String {
    format!("r.{}-{}.png", region.coordinate.0, region.coordinate.1)
}

/// type def for the hash map of textures and string
type TextureListMap = HashMap<String, DynamicImage>;

/// Returns a list of all the filenames in the assets folder hash mapped to the image data respective to that file name.
fn get_texture_list() -> TextureListMap {
    let dir = fs::read_dir("assets").unwrap();
    let list: Vec<String> = dir
        .into_iter()
        .filter_map(|file_in_dir| match file_in_dir {
            Ok(f) => Some(f),
            Err(_) => None,
        })
        .filter_map(|file_entry| file_entry.file_name().to_str().map(|str| str.to_string()))
        .collect();

    let mut map: TextureListMap = HashMap::new();

    for file_name in list {
        let path = format!("assets/{}", file_name);
        let texture_name = file_name.split('.').next().unwrap(); // take the first thing that appears before the file extension
        let minecraft_texture_name = format!("minecraft:{}", texture_name);
        let image_data = read_texture_from_texture_name(path);

        map.insert(minecraft_texture_name, image_data);
    }
    map
}

/// Stitches region images together in a not super intelligent way.
fn stitch_region_images(list: Vec<RegionImage>) -> RgbImage {
    let region_image_size = 8192;

    let min_modifier_x = &list.iter().map(|ri| ri.coordinate.0).min().unwrap();
    let min_modifier_y = &list.iter().map(|ri| ri.coordinate.1).min().unwrap();
    let max_modifier_x = &list.iter().map(|ri| ri.coordinate.0).max().unwrap();
    let max_modifier_y = &list.iter().map(|ri| ri.coordinate.1).max().unwrap();

    let min_coefficient_x = (min_modifier_x * region_image_size).abs();
    let min_coefficient_y = (min_modifier_y * region_image_size).abs();

    // TODO: eventually calculate this width and height more intelligently. At the moment the program produces a larger image than is needed by a lot.
    let width = region_image_size * (min_modifier_x.abs() + 1 + max_modifier_x.abs()) as i32;
    let height = region_image_size * (min_modifier_y.abs() + 1 + max_modifier_y.abs()) as i32;

    let mut img: RgbImage = ImageBuffer::new(width as u32, height as u32);

    for region in list {
        let region_x = ((region.coordinate.0 * region_image_size) + min_coefficient_x) as usize;
        let region_y = ((region.coordinate.1 * region_image_size) + min_coefficient_y) as usize;
        let pixels = region.image.enumerate_pixels();

        for pixel in pixels {
            let color = pixel.2.to_rgb();
            let x = pixel.0 as usize;
            let y = pixel.1 as usize;
            let pixel_x = (region_x + x) as u32;
            let pixel_y = (region_y + y) as u32;
            img.put_pixel(pixel_x, pixel_y, color);
        }
    }
    img
}

/// Converts a region file into an image and returns it, returns a black image of nothing if the region is not read correctly.
fn region_to_image(region_selected: &RegionFile, texture_list: &TextureListMap) -> RgbImage {
    let file = &region_selected.file;
    let region_coords = &region_selected.coordinate;

    let mut region = fastanvil::Region::from_stream(file).unwrap();
    let mut images_of_chunks: Vec<(RgbImage, usize, usize)> = vec![]; // image,x,y

    for chunk_x in 0..32 {
        for chunk_y in 0..32 {
            let data = match region.read_chunk(chunk_x, chunk_y) {
                Ok(r) => match r {
                    None => {
                        return ImageBuffer::new(8192, 8192); // if the region cant be read for any reason we return a black region image, could be done better i bet?
                    }
                    Some(vec) => vec,
                },
                Err(_) => {
                    return ImageBuffer::new(8192, 8192); // if the region cant be read for any reason we return a black region image, could be done better i bet?
                }
            };

            let chunk_result = from_bytes(data.as_slice());

            if chunk_result.is_err() {
                return ImageBuffer::new(8192, 8192);
            }

            let chunk: CurrentJavaChunk = chunk_result.unwrap();
            images_of_chunks.push(chunk_to_image(
                chunk,
                chunk_x,
                chunk_y,
                texture_list,
                region_coords,
            ));
        }
    }

    let mut img: RgbImage = ImageBuffer::new(8192, 8192); // 4096 = 16 chunk images * 16 chunks total

    for chunk in images_of_chunks {
        let block_x = chunk.1 * 256;
        let block_y = chunk.2 * 256;
        let pixels = chunk.0.enumerate_pixels();

        for pixel in pixels {
            let color = pixel.2.to_rgb();
            let x = pixel.0 as usize;
            let y = pixel.1 as usize;
            let pixel_x = (block_x + x) as u32;
            let pixel_y = (block_y + y) as u32;
            img.put_pixel(pixel_x, pixel_y, color);
        }
    }
    img
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

    // loop to flatten the chunk and only take blocks that are seeing the sky.
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

    let mut img: RgbImage = ImageBuffer::new(256, 256);
    for block in flattened_blocks {
        let block_x = block.0 .0 * 16;
        let block_y = block.0 .1 * 16;
        let mc_block = &block.1;
        let texture = match texture_list.get(mc_block.name()) {
            None => {
                // panic!("ERROR BLOCK TEXTURE NOT FOUND FOR BLOCK: {:?}", mc_block);
                //println!("no texture for: {:?}, using anvil", mc_block);
                texture_list.get("minecraft:anvil").unwrap()
            }
            Some(tex) => tex,
        };
        // loop to take the respective block textures and place them in the place the blocks occur in.
        for pixel in texture.pixels() {
            let color = pixel.2.to_rgb();
            let x = pixel.0 as usize;
            let y = pixel.1 as usize;
            let pixel_x = (block_x + x) as u32;
            let pixel_y = (block_y + y) as u32;

            if pixel_x >= 256 || pixel_y >= 256 {
                panic!("pixel x or y was >= 256: {:?}", mc_block);
            }

            img.put_pixel(pixel_x, pixel_y, color);
        }
    }
    (img, chunk_x, chunk_y)
}

#[derive(Debug, Copy, Clone)]
/// A struct to contain the region coordinate of a region file. e.g. r.-1.2.mca becomes ChunkCoordinate(-1,2)
struct ChunkCoordinate(i32, i32);

#[derive(Debug)]
/// A struct to contain a region file header and its respective chunk coordinate.
struct RegionFile {
    coordinate: ChunkCoordinate,
    file: File,
}

#[derive(Debug)]
/// A struct to contain a region image and its respective chunk coordinate.
struct RegionImage {
    coordinate: ChunkCoordinate,
    image: RgbImage,
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

/// Get all region files contained within a directory, output a vector full of the file handles and their region coordinates.
/// e.g. r.0.-1.mca becomes a file header to that file, and a chunk coordinate of 0,-1
fn get_region_files(path: &str) -> Vec<RegionFile> {
    let dir = fs::read_dir(path).unwrap();
    let list: Vec<RegionFile> = dir
        .into_iter()
        .filter_map(|file_in_dir| match file_in_dir {
            Ok(f) => Some(f),
            Err(_) => None,
        })
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
                ChunkCoordinate(*coords.first().unwrap(), *coords.get(1).unwrap());

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
