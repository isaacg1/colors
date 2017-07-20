extern crate image;
#[macro_use]
extern crate itertools;
extern crate rand;

use image::ImageBuffer;

use std::fs::File;
use std::path::Path;

use rand::{thread_rng, Rng};

use std::collections::{HashSet, HashMap};

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
    let size = 2;
    let color_range = (size * size) as u8;
    let side_length = size * size * size;
    let mut colors: Vec<Color> = iproduct!(0..color_range, 0..color_range, 0..color_range)
        .collect();
    thread_rng().shuffle(&mut colors);
    let mut unassigned_locations: HashSet<Location> = iproduct!(0..side_length, 0..side_length)
        .collect();
    let mut assigned_colors: HashMap<Color, Location> = HashMap::new();
    for color in colors {
        let closest_assigned_color = assigned_colors
            .keys()
            .min_by_key(|oth_color| color_distance(&color, oth_color));
        let location = if let Some(closest_assigned_color) = closest_assigned_color {
            let target_cell = assigned_colors
                .get(closest_assigned_color)
                .expect("Just looked it up");
            *unassigned_locations.iter().min_by(|loc1, loc2| {
                location_distance(&target_cell, loc1)
                    .partial_cmp(&location_distance(&target_cell, loc2))
                    .expect("Not NaN")
            }).expect("There's at least one left")
        } else {
            *thread_rng().choose(&unassigned_locations
                .iter()
                .cloned()
                .collect::<Vec<Location>>()).expect("There's plenty_left")
        };
        unassigned_locations.remove(&location);
        assigned_colors.insert(color, location);
    }
    let img = ImageBuffer::new(side_length, side_length);
    let ref mut fout = File::create(&Path::new(&format!("pic{}.png", size))).unwrap();
    image::ImageRgb8(img).save(fout, image::PNG).unwrap();
}
