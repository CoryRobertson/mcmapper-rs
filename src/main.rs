


//! mcmapper-rs is a program that reads a minecraft world, flattens it, then generates an image from that grouping of blocks and saves it to the system.

extern crate core;

use image::imageops::FilterType;
use image::{imageops};
use rayon::prelude::*;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::SystemTime;
use std::{env, fs};
use mcmapper_rs::{find_bounding_box_for_map, get_region_files, get_texture_list, region_file_to_file_name, region_to_image, RegionImage, stitch_region_images};

mod timer;



fn main() {
    let args: Vec<String> = env::args().collect();

    // this might need improvement? Maybe prompt user using stdin?
    let world_path: String = match args.get(1) {
        None => {
            println!("No region folder path given, running with default of \"test/region\" ");
            if fs::read_dir("test/region").is_ok() {
                "test/region".to_string()
            } else {
                panic!(
                    "World path not found as command line args and no test world directory exists, please provide the program with a path to a minecraft world."
                );
            }
        }
        Some(path) => {
            if fs::read_dir(path).is_ok() {
                // path is valid directory
                path.to_string()
            } else {
                panic!("World path found in args, but is not valid");
            }
        }
    };

    // checking if output dir exists, if not try to create it, if it cant, then panic the program.
    match fs::read_dir("output") {
        Ok(_) => {}
        Err(err) => match fs::create_dir("output") {
            Ok(_) => {}
            Err(err2) => {
                panic!(
                    "Error, unable to read or create output directory. \n {} \n {}",
                    err, err2
                );
            }
        },
    }

    // check if assets folder is present, if not try to create it, and if that cant happen panic the program
    match fs::read_dir("assets") {
        Ok(_) => {}
        Err(err) => {
            match fs::create_dir("assets") {
                Ok(_) => {}
                Err(err2) => {
                    panic!("Unable to create assets folder, missing permissions? Try making assets folder and putting minecraft block textures in it. {}", err2);
                }
            }
            panic!("No assets folder present, please place minecraft assets in assets folder and run program again. {}", err);
        }
    }

    let list = get_region_files(&world_path);
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
                .expect("Unable to save region image to system. Missing permissions?"); // save the region image that was generated

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
    let start_stitch_time = SystemTime::now();

    let full_map_image = stitch_region_images(&region_images.lock().unwrap()); // generate the full map image from all the region images

    println!(
        "Stitch time: {:.2} seconds",
        SystemTime::now()
            .duration_since(start_stitch_time)
            .unwrap()
            .as_secs_f32()
    );

    println!("Cropping and saving full map image...");

    // crop the image to the bounding box we calculate for the full image
    let crop = find_bounding_box_for_map(&full_map_image);
    let cropped_full_map_image =
        imageops::crop_imm(&full_map_image, crop.0 .0, crop.0 .1, crop.1 .0, crop.1 .1).to_image();
    cropped_full_map_image
        .save("./output/cropped_all_regions_massive.png")
        .expect("Unable to save image to system, missing permissions?");

    println!("Scaling and saving newly cropped image...");

    let scaled_full_map_image = imageops::resize(
        &cropped_full_map_image,
        cropped_full_map_image.width() / 8,
        cropped_full_map_image.height() / 8,
        FilterType::Nearest,
    ); // scale the image down a good amount for distribution reasons.

    scaled_full_map_image
        .save("./output/cropped_all_regions_tenth.png")
        .expect("Unable to save image to system, missing permissions?");

    println!("Done!");
}

