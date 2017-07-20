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

use std::u32;

use std::time::Instant;

type Color = (u8, u8, u8);

type Location = (u32, u32);
fn location_distance(loc: &Location, oth_loc: &Location) -> f64 {
    (loc.0 as f64 - oth_loc.0 as f64).hypot(loc.1 as f64 - oth_loc.1 as f64)
}

fn offset_closest(
    location_offsets: &Vec<(i64, i64)>,
    unassigned_locations: &HashSet<Location>,
    target_cell: &Location,
) -> Location {
    location_offsets
        .iter()
        .filter_map(|offset| {
            let new0 = target_cell.0 as i64 + offset.0;
            let new1 = target_cell.1 as i64 + offset.1;
            if 0 <= new0 && new0 <= u32::MAX as i64 && 0 <= new1 && new1 <= u32::MAX as i64 {
                let location = (new0 as u32, new1 as u32);
                if unassigned_locations.contains(&location) {
                    Some(location)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .next()
        .expect("There's at least one left")
}

fn unassigned_closest(
    unassigned_locations: &HashSet<Location>,
    target_cell: &Location,
) -> Location {
    *unassigned_locations
        .iter()
        .min_by(|loc1, loc2| {
            location_distance(&target_cell, loc1)
                .partial_cmp(&location_distance(&target_cell, loc2))
                .expect("Not NaN")
        })
        .expect("There's at least one left")
}
fn main() {
    let size: u32 = match args().nth(1) {
        Some(num) => num.parse().unwrap(),
        None => panic!("Provide size as first arg"),
    };
    let debug_on = args().nth(2).is_some();
    assert!(size * size < 256);
    // Todo: support size = 16
    let color_range = (size * size) as u8;
    let color_multiplier = 255 / color_range;
    let side_length = size * size * size;
    let random_locs = size * 2;
    let mut colors: Vec<Color> = iproduct!(0..color_range, 0..color_range, 0..color_range)
        .collect();
    thread_rng().shuffle(&mut colors);
    let mut color_offsets: Vec<(i64, i64, i64)> = iproduct!(
        -(color_range as i64)..color_range as i64,
        -(color_range as i64)..color_range as i64,
        -(color_range as i64)..color_range as i64
    ).collect();
    color_offsets.sort_by_key(|offset| offset.0.pow(2) + offset.1.pow(2) + offset.2.pow(2));
    let mut unassigned_locations: HashSet<Location> = iproduct!(0..side_length, 0..side_length)
        .collect();
    let mut location_offsets: Vec<(i64, i64)> = iproduct!(
        -(side_length as i64)..side_length as i64,
        -(side_length as i64)..side_length as i64
    ).collect();
    location_offsets.sort_by(|offset1, offset2| {
        (offset1.0 as f64)
            .hypot(offset1.1 as f64)
            .partial_cmp(&(offset2.0 as f64).hypot(offset2.1 as f64))
            .expect("Not NaN")
    });
    assert_eq!(colors.len(), unassigned_locations.len());
    let mut assigned_colors: HashMap<Color, Location> = HashMap::new();
    let mut img = ImageBuffer::new(side_length, side_length);
    let mut time = Instant::now();
    let mut use_unassigned_instead_of_offset_in_a_row = 0;
    for (i, color) in colors.into_iter().enumerate() {
        if debug_on && i % 1000 == 0 {
            println!(
                "{} {} {} {:?} {}",
                i,
                unassigned_locations.len(),
                size.pow(6),
                time.elapsed(),
                use_unassigned_instead_of_offset_in_a_row
            );
            time = Instant::now();
        }
        let location = if i >= random_locs as usize {
            let closest_assigned_color = color_offsets
                .iter()
                .filter_map(|offset| {
                    let new0 = color.0 as i64 + offset.0;
                    let new1 = color.1 as i64 + offset.1;
                    let new2 = color.2 as i64 + offset.2;
                    if 0 <= new0 && new0 < 256 && 0 <= new1 && new1 < 256 && 0 <= new2 &&
                        new2 < 256
                    {
                        let color = (new0 as u8, new1 as u8, new2 as u8);
                        if assigned_colors.contains_key(&color) {
                            Some(color)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .next()
                .expect("It's not empty any more");
            let target_cell = assigned_colors
                .get(&closest_assigned_color)
                .expect("Just looked it up");
            if use_unassigned_instead_of_offset_in_a_row < 3 {
                let start_time = if i % 1000 == 0 {
                    Some(Instant::now())
                } else {
                    None
                };
                let res = offset_closest(&location_offsets, &unassigned_locations, target_cell);
                if let Some(start_time) = start_time {
                    let elapsed = start_time.elapsed();
                    let other_start = Instant::now();
                    unassigned_closest(&unassigned_locations, target_cell);
                    let other_elapsed = other_start.elapsed();
                    use_unassigned_instead_of_offset_in_a_row = if other_elapsed < elapsed {
                        use_unassigned_instead_of_offset_in_a_row + 1
                    } else {
                        0
                    }
                }
                res
            } else {
                unassigned_closest(&unassigned_locations, target_cell)
            }
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
    let fout = &mut File::create(&Path::new(&filename)).unwrap();
    image::ImageRgb8(img).save(fout, image::PNG).unwrap();
    println!("Saved to {}", &filename);
}
