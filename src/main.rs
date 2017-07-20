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

fn main() {
    let size: u32 = match args().nth(1) {
        Some(num) => num.parse().unwrap(),
        None => panic!("Provide size as first arg"),
    };
    let debug_frequency: Option<usize> = args().nth(2).map(|freq| freq.parse().unwrap());
    assert!(size * size < 256);
    // Todo: support size = 16
    let color_range = (size * size) as u8;
    let color_multiplier = 255f64 / color_range as f64;
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
    let mut frontiers: Vec<HashSet<Location>> = (0..random_locs).map(|_| HashSet::new()).collect();
    let mut assigned_region: HashMap<Location, usize> = HashMap::new();
    assert_eq!(colors.len(), unassigned_locations.len());
    let mut assigned_colors: HashMap<Color, (Location, usize)> = HashMap::new();
    let mut img = ImageBuffer::new(side_length, side_length);
    let mut time = Instant::now();
    for (i, color) in colors.into_iter().enumerate() {
        if let Some(debug_frequency) = debug_frequency {
            if i > 0 && i % debug_frequency == 0 {
                let time_per_pixel = (time.elapsed() / debug_frequency as u32)
                    .subsec_nanos() as f64 / 10f64.powi(9);
                println!(
                    "Completed {} out of {} pixels,  {} milliseconds per pixel\n\
                     Approximately {} sec to go.\n\
                     Region(s) {:?} still remain.",
                    i,
                    size.pow(6),
                    time_per_pixel * 1000f64,
                    (size.pow(6) as f64 - i as f64) * time_per_pixel,
                    (0..frontiers.len())
                        .filter(|&i| !frontiers[i].is_empty())
                        .collect::<Vec<usize>>()
                );
                time = Instant::now();
            }
        }
        let (location, frontier_index) = if i >= random_locs as usize {
            let &(target_cell, frontier_index) = color_offsets
                .iter()
                .filter_map(|offset| {
                    let new0 = color.0 as i64 + offset.0;
                    let new1 = color.1 as i64 + offset.1;
                    let new2 = color.2 as i64 + offset.2;
                    if 0 <= new0 && new0 < 256 && 0 <= new1 && new1 < 256 && 0 <= new2 &&
                        new2 < 256
                    {
                        let color = (new0 as u8, new1 as u8, new2 as u8);
                        assigned_colors.get(&color)
                    } else {
                        None
                    }
                })
                .next()
                .expect("It's not empty any more");
            (
                *frontiers[frontier_index]
                    .iter()
                    .min_by(|loc1, loc2| {
                        location_distance(&target_cell, loc1)
                            .partial_cmp(&location_distance(&target_cell, loc2))
                            .expect("Not NaN")
                    })
                    .expect("There's at least one left"),
                frontier_index,
            )
        } else {
            let location = *thread_rng()
                .choose(&unassigned_locations
                    .iter()
                    .cloned()
                    .collect::<Vec<Location>>())
                .expect("There's plenty_left");
            (location, i)
        };
        unassigned_locations.remove(&location);
        assigned_region.insert(location, frontier_index);
        frontiers[frontier_index].remove(&location);
        for neighbor in &[
            (location.0 + 1, location.1),
            (location.0, location.1 + 1),
            (location.0.saturating_sub(1), location.1),
            (location.0, location.1.saturating_sub(1)),
        ] {
            if let Some(&neighbor_region) = assigned_region.get(neighbor) {
                if neighbor_region != frontier_index {
                    // Collapse the two regions.
                    if debug_frequency.is_some() {
                        println!("Collapsing {} into {}", neighbor_region, frontier_index);
                    }
                    let neighbor_frontier: Vec<Location> =
                        frontiers[neighbor_region].drain().collect();
                    frontiers[frontier_index].extend(neighbor_frontier);
                    for region in assigned_region.values_mut() {
                        if *region == neighbor_region {
                            *region = frontier_index;
                        }
                    }
                    for location_and_region in assigned_colors.values_mut() {
                        if location_and_region.1 == neighbor_region {
                            location_and_region.1 = frontier_index;
                        }
                    }
                }
            } else if unassigned_locations.contains(neighbor) {
                frontiers[frontier_index].insert(*neighbor);
            }
        }
        assigned_colors.insert(color, (location, frontier_index));
        let pixel = image::Rgb([
            (color.0 as f64 * color_multiplier) as u8,
            (color.1 as f64 * color_multiplier) as u8,
            (color.2 as f64 * color_multiplier) as u8,
        ]);
        img.put_pixel(location.0, location.1, pixel);
    }
    let filename = format!("pic{}-{}.png", size, random::<u32>());
    let fout = &mut File::create(&Path::new(&filename)).unwrap();
    image::ImageRgb8(img).save(fout, image::PNG).unwrap();
    println!("Saved to {}", &filename);
}
