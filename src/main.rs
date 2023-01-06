use fastanvil::{Block, Chunk, CurrentJavaChunk};
use fastnbt::from_bytes;
use image::imageops::FilterType;
use image::{imageops, DynamicImage, GenericImageView, ImageBuffer, Pixel, RgbImage};
use rayon::prelude::*;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::fs;
use std::fs::File;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

struct BoundingBox((u32,u32),(u32,u32));

/// Given an image, finds the smallest square shape crop that removes only rgb[0,0,0] pixels.
/// Due to the image crate, the output is a bounding box where the first two numbers are x and y to start, but the second two are width and height, not x2 and y2.
fn find_bounding_box_for_map(image: &RgbImage) -> BoundingBox {
    let width = image.width();
    let height = image.height();

    // calculate the lowest x coord bound
    let mut lower_x_bound = 0;
    'outer: for x in 0..width {
        for y in 0..height {
            if image.get_pixel(x,y).0.ne(&[0,0,0]) {
                lower_x_bound = x;
                break 'outer;
            }
        }
    }

    // calculate the highest x coord bound
    let mut upper_x_bound= width;
    'outer: for x in (0..width).rev() {
        for y in (0..height).rev() {
            if image.get_pixel(x,y).0.ne(&[0,0,0]) {
                upper_x_bound = x;
                break 'outer;
            }
        }
    }

    // calculate lowest y coord bound
    let mut lower_y_bound= 0;
    'outer: for y in 0..height {
        for x in 0..width {
            if image.get_pixel(x,y).0.ne(&[0,0,0]) {
                lower_y_bound = y;
                break 'outer;
            }
        }
    }

    // calculate the highest y coord bound
    let mut upper_y_bound= height;
    'outer: for y in (0..height).rev() {
        for x in (0..width).rev() {
            if image.get_pixel(x,y).0.ne(&[0,0,0]) {
                upper_y_bound = y;
                break 'outer;
            }
        }
    }

    BoundingBox((lower_x_bound,lower_y_bound),(upper_x_bound - lower_x_bound,upper_y_bound - lower_y_bound))
}

fn main() {

    // TODO: eventually prompt user for all these things instead of just expecting things to be in the right folder.

    let list = get_region_files("test/region");
    println!("Discovering texture files");
    let texture_list = get_texture_list();
    for region_file in &list {
        println!("Region file found: {}", region_file);
    }
    println!("Length of region file list: {}", list.len());

    let region_images: Mutex<Vec<RegionImage>> = Mutex::new(vec![]); // vector full of all the images that are generated from the region files
    let threads_finished: AtomicU32 = AtomicU32::new(1); // number of threads that are finished
    let number_of_regions = list.len() as u32; // number of regions to calculate images for.

    list.into_par_iter()
        .enumerate()
        .for_each(|(index, region)| {
            println!("Thread {} started.\n", index);
            let region_image = region_to_image(&region, &texture_list); // generate the image of a region
            let file_name = region_file_to_file_name(&region); // get the file name that the region should have

            region_image
                .save(format!("./output/{}", file_name))
                .unwrap(); // save the region image that was generated

            region_images.lock().unwrap().push(RegionImage {
                coordinate: region.coordinate,
                image: region_image,
            }); // add the image of the region to the region images vector so we can stitch them all together
            println!("Thread {} ended.\n", index);
            println!(
                "Progress: {}/{}.\n",
                threads_finished.load(Ordering::Relaxed),
                number_of_regions
            ); // print out the progress of how many threads are done versus not done.

            threads_finished.fetch_add(1, Ordering::Relaxed); // add to the number of threads that have concluded
        });

    println!("Stitching regions...");
    let full_map_image = stitch_region_images(&region_images.lock().unwrap()); // generate the full map image from all the region images
    full_map_image
        .save("./output/all_regions_massive.png")
        .unwrap(); // save the full map image to the system
    let full_map_width = full_map_image.width();
    let full_map_height = full_map_image.height();

    println!("Finished stitching images, scaling image now...");

    let scaled_full_map_image = imageops::resize(
        &full_map_image,
        full_map_width / 10,
        full_map_height / 10,
        FilterType::Nearest,
    ); // scale the image down a good amount for distribution reasons.

    scaled_full_map_image
        .save("./output/all_regions_tenth.png")
        .unwrap(); // save the scaled image down.


    println!("Cropping full map image...");

    // crop the image to the bounding box we calculate for the full image
    let crop = find_bounding_box_for_map(&full_map_image);
    let cropped_full_map_image = imageops::crop_imm(&full_map_image,crop.0.0,crop.0.1,crop.1.0,crop.1.1).to_image();
    cropped_full_map_image.save("./output/cropped_all_regions_massive.png").unwrap();

    println!("Scaling newly cropped image...");

    let scaled_full_map_image = imageops::resize(
        &cropped_full_map_image,
        cropped_full_map_image.width() / 10,
        cropped_full_map_image.height() / 10,
        FilterType::Nearest,
    ); // scale the image down a good amount for distribution reasons.

    scaled_full_map_image.save("./output/cropped_all_regions_tenth.png").unwrap();

    println!("Done!");
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

        if image_data.height() > 16 || image_data.width() > 16 { // if the texture loaded is larger than expected, we resize it
            let resized_image_data = imageops::resize(&image_data,16,16,FilterType::Nearest);
            map.insert(minecraft_texture_name, DynamicImage::from(resized_image_data));
        }
        else { // if its the expected size or smaller, re just load the image into the hash map.
            map.insert(minecraft_texture_name, image_data);
        }

    }

    map.insert("minecraft:error".to_string(),read_texture_from_texture_name("error.png".to_string()));

    map
}


/// Stitches region images together in a not super intelligent way.
fn stitch_region_images(list: &Vec<RegionImage>) -> RgbImage {
    let region_image_size = 8192;

    let min_modifier_x = &list.iter().map(|ri| ri.coordinate.0).min().unwrap();
    let min_modifier_y = &list.iter().map(|ri| ri.coordinate.1).min().unwrap();
    let max_modifier_x = &list.iter().map(|ri| ri.coordinate.0).max().unwrap();
    let max_modifier_y = &list.iter().map(|ri| ri.coordinate.1).max().unwrap();

    let min_coefficient_x = (min_modifier_x * region_image_size).abs();
    let min_coefficient_y = (min_modifier_y * region_image_size).abs();

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

/// convert a chunk to an image, the chunk x and chunk y are purely for file naming and image placement in the region file..
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
                        if b.name().ne("minecraft:air") && b.name().ne("minecraft:cave_air") {
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
                // this function slows down the program a good amount in terms of chunk rendering, worth the cost for the easier output though.
                match search_texture_map(&texture_list,mc_block.name()) {
                    None => {
                        // #[cfg(debug_assertions)]
                        // println!("no texture for: {:?}, using error texture: \n", mc_block);
                        // panic!("unable to find texture, even after searched texture: {:?}", mc_block);
                        texture_list.get("minecraft:error").unwrap()
                    }
                    Some(tex) => {
                        // panic!("unable to find texture, found searched texture: {:?}", mc_block);
                        tex
                    }
                }
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

/// Takes in a list of textures and a search name, and returns either nothing if the texture was not found, or the texture that was found.
fn search_texture_map<'a>(list: &'a TextureListMap, search_name: &str) -> Option<&'a DynamicImage> {
    for (name,texture) in list {
        // if we happen to fine a name of a block that has extra text after, e.g. we are searching for oak_stairs but we find dark_oak_stairs, this should find it and be good enough.
        if name.contains(search_name) {
            return Some(texture);
        }
        // if the first search doesnt work, we can shorten the name and remove its modifiers e.g. "dark_oak_stairs" becomes "dark", very approximate but it works for simplicity sake.
        // TODO: eventually improve this search by instead seeing if it can find a texture that contains the most portions of search name when split by '_', this will require a large search function.
        //  Unsure if this is worth the runtime costs.
        match search_name.split("_").next() {
            None => {}
            Some(short_name) => {
                if name.contains(short_name) {
                    return Some(texture);
                }
            }
        }
    }
    return None;
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
