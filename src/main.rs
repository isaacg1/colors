extern crate image;
#[macro_use]
extern crate itertools;
extern crate rand;

use image::ImageBuffer;

use std::fs::File;
use std::path::Path;

use rand::{thread_rng, Rng, random};

use std::collections::{HashSet, HashMap};

use std::env::args;

type Color = (u8, u8, u8);
fn color_distance(color: &Color, oth_color: &Color) -> i64 {
    (color.0 as i64 - oth_color.0 as i64).pow(2) + (color.1 as i64 - oth_color.1 as i64).pow(2) +
        (color.2 as i64 - oth_color.2 as i64).pow(2)
}

type Location = (u32, u32);
fn location_distance(loc: &Location, oth_loc: &Location) -> f64 {
    (loc.0 as f64 - oth_loc.0 as f64).hypot(loc.1 as f64 - oth_loc.1 as f64)
}

fn main() {
    let size: u32 = match args().nth(1) {
        Some(num) => num.parse().unwrap(),
        None => panic!("Provide size as first arg"),
    };
    assert!(size * size < 256);
    // Todo: support size = 16
    let color_range = (size * size) as u8;
    let color_multiplier = 255 / color_range;
    let side_length = size * size * size;
    let random_locs = size * 2;
    let mut colors: Vec<Color> = iproduct!(0..color_range, 0..color_range, 0..color_range)
        .collect();
    thread_rng().shuffle(&mut colors);
    let mut unassigned_locations: HashSet<Location> = iproduct!(0..side_length, 0..side_length)
        .collect();
    assert_eq!(colors.len(), unassigned_locations.len());
    let mut assigned_colors: HashMap<Color, Location> = HashMap::new();
    let mut img = ImageBuffer::new(side_length, side_length);
    for (i, color) in colors.into_iter().enumerate() {
        let location = if i >= random_locs as usize {
            let closest_assigned_color = assigned_colors
                .keys()
                .min_by_key(|oth_color| color_distance(&color, oth_color))
                .expect("It's not empty any more");
            let target_cell = assigned_colors
                .get(closest_assigned_color)
                .expect("Just looked it up");
            *unassigned_locations
                .iter()
                .min_by(|loc1, loc2| {
                    location_distance(&target_cell, loc1)
                        .partial_cmp(&location_distance(&target_cell, loc2))
                        .expect("Not NaN")
                })
                .expect("There's at least one left")
        } else {
            *thread_rng()
                .choose(&unassigned_locations
                    .iter()
                    .cloned()
                    .collect::<Vec<Location>>())
                .expect("There's plenty_left")
        };
        unassigned_locations.remove(&location);
        assigned_colors.insert(color, location);
        let pixel = image::Rgb([
            color.0 * color_multiplier,
            color.1 * color_multiplier,
            color.2 * color_multiplier,
        ]);
        img.put_pixel(location.0, location.1, pixel);
    }
    let filename = format!("pic{}-{}.png", size, random::<u32>());
    let ref mut fout = File::create(&Path::new(&filename)).unwrap();
    image::ImageRgb8(img).save(fout, image::PNG).unwrap();
    println!("Saved to {}", &filename);
}
