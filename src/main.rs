#![feature(test)]
extern crate test;

extern crate image;
#[macro_use]
extern crate itertools;
extern crate rand;
extern crate clap;

use clap::{Arg, App};

use image::{ImageBuffer, DynamicImage};

use std::fs::File;
use std::path::Path;

use rand::{thread_rng, Rng, random};

use std::collections::{HashSet, HashMap};

use std::u32;

use std::time::Instant;

use std::mem::swap;

type Color = (u8, u8, u8);

type Location = (u32, u32);

type FrontierIndex = usize;

fn squared_location_distance(loc: &Location, oth_loc: &Location) -> i64 {
    let dx = loc.0 as i64 - oth_loc.0 as i64;
    let dy = loc.1 as i64 - oth_loc.1 as i64;

    dx * dx + dy * dy
}

fn maybe_print_debug_info(
    debug_frequency: Option<usize>,
    pixel_index: usize,
    size: u32,
    time: &mut Instant,
    frontiers: &Vec<HashSet<Location>>,
) {
    if let Some(debug_frequency) = debug_frequency {
        if pixel_index > 0 && pixel_index % debug_frequency == 0 {
            let time_per_pixel = (time.elapsed() / debug_frequency as u32).subsec_nanos() as f64 /
                10f64.powi(9);
            println!(
                "Completed {} out of {} pixels,  {} milliseconds per pixel\n\
                     Approximately {} sec to go.\n\
                     {} frontier(s) with {} pixels exist.",
                pixel_index,
                size.pow(6),
                time_per_pixel * 1000f64,
                (size.pow(6) as f64 - pixel_index as f64) * time_per_pixel,
                frontiers
                    .iter()
                    .filter(|&frontier| !frontier.is_empty())
                    .count(),
                frontiers
                    .iter()
                    .map(|frontier| frontier.len())
                    .sum::<usize>()
            );
            *time = Instant::now();
        }
    }
}

fn find_target_cell_and_frontier<'a>(
    color: Color,
    color_offsets: &Vec<(i64, i64, i64)>,
    assigned_colors: &'a HashMap<Color, (Location, FrontierIndex)>,
) -> &'a (Location, FrontierIndex) {
    color_offsets
        .iter()
        .filter_map(|offset| {
            let new0 = color.0 as i64 + offset.0;
            let new1 = color.1 as i64 + offset.1;
            let new2 = color.2 as i64 + offset.2;
            if 0 <= new0 && new0 < 256 && 0 <= new1 && new1 < 256 && 0 <= new2 && new2 < 256 {
                let color = (new0 as u8, new1 as u8, new2 as u8);
                assigned_colors.get(&color)
            } else {
                None
            }
        })
        .next()
        .expect("It's not empty any more")
}

fn collapse_into(
    source_region: usize,
    target_region: usize,
    debug: bool,
    frontiers: &mut Vec<HashSet<Location>>,
    locations_to_regions: &mut HashMap<Location, Option<usize>>,
    assigned_colors: &mut HashMap<Color, (Location, usize)>,
) {
    if debug {
        println!("Collapsing {} into {}", source_region, target_region);
    }
    let mut temp_frontier = HashSet::new();
    swap(&mut temp_frontier, &mut frontiers[source_region]);
    frontiers[target_region].extend(temp_frontier.drain().filter(|location| {
        locations_to_regions.get(location) == Some(&None)
    }));
    for region in locations_to_regions.values_mut() {
        if *region == Some(source_region) {
            *region = Some(target_region);
        }
    }
    for location_and_region in assigned_colors.values_mut() {
        if location_and_region.1 == source_region {
            location_and_region.1 = target_region;
        }
    }
}
fn make_image(size: u32, debug_frequency: Option<usize>) -> DynamicImage {
    assert!(size <= 16);
    let color_range = size * size;
    let color_range_vec: Vec<u8> = (0..size * size).map(|color| color as u8).collect();
    let color_multiplier = 255f64 / color_range as f64;
    let side_length = size * size * size;
    let random_locs = size * 2;
    let colors = {
        let mut colors: Vec<Color> = iproduct!(
            color_range_vec.iter().cloned(),
            color_range_vec.iter().cloned(),
            color_range_vec.iter().cloned()
        ).collect();
        thread_rng().shuffle(&mut colors);
        colors
    };
    let color_offsets = {
        let mut color_offsets: Vec<(i64, i64, i64)> = iproduct!(
            -(color_range as i64)..color_range as i64,
            -(color_range as i64)..color_range as i64,
            -(color_range as i64)..color_range as i64
        ).collect();
        color_offsets.sort_by_key(|offset| offset.0.pow(2) + offset.1.pow(2) + offset.2.pow(2));
        color_offsets
    };
    let mut locations_to_regions: HashMap<Location, Option<usize>> =
        iproduct!(0..side_length, 0..side_length)
            .map(|location| (location, None))
            .collect();
    assert_eq!(colors.len(), locations_to_regions.len());
    let mut frontiers: Vec<HashSet<Location>> = (0..random_locs).map(|_| HashSet::new()).collect();
    let mut assigned_colors: HashMap<Color, (Location, usize)> = HashMap::new();
    let mut img = ImageBuffer::new(side_length, side_length);
    let mut time = Instant::now();
    for (i, color) in colors.into_iter().enumerate() {
        maybe_print_debug_info(debug_frequency, i, size, &mut time, &frontiers);
        let (location, frontier_index) = if i >= random_locs as usize {
            let &(target_cell, frontier_index) =
                find_target_cell_and_frontier(color, &color_offsets, &assigned_colors);
            (
                *frontiers[frontier_index]
                    .iter()
                    .min_by_key(|loc| squared_location_distance(&target_cell, loc))
                    .expect("There's at least one left"),
                frontier_index,
            )
        } else {
            let location = loop {
                let &(&location, region) = thread_rng()
                    .choose(&locations_to_regions.iter().collect::<Vec<_>>())
                    .expect("There's plenty_left");
                if region.is_none() {
                    break location;
                }
            };
            (location, i)
        };
        let previous_region = locations_to_regions.insert(location, Some(frontier_index));
        assert_eq!(previous_region, Some(None));
        frontiers[frontier_index].remove(&location);
        for neighbor in &[
            (location.0 + 1, location.1),
            (location.0, location.1 + 1),
            (location.0.saturating_sub(1), location.1),
            (location.0, location.1.saturating_sub(1)),
        ]
        {
            if let Some(&region) = locations_to_regions.get(neighbor) {
                if let Some(neighbor_region) = region {
                    if neighbor_region != frontier_index {
                        collapse_into(
                            neighbor_region,
                            frontier_index,
                            debug_frequency.is_some(),
                            &mut frontiers,
                            &mut locations_to_regions,
                            &mut assigned_colors,
                        );
                    }
                } else {
                    frontiers[frontier_index].insert(*neighbor);
                }
            }
        }
        assigned_colors.insert(color, (location, frontier_index));
        let pixel = image::Rgb(
            [
                (color.0 as f64 * color_multiplier) as u8,
                (color.1 as f64 * color_multiplier) as u8,
                (color.2 as f64 * color_multiplier) as u8,
            ],
        );
        img.put_pixel(location.0, location.1, pixel);
    }
    image::ImageRgb8(img)
}

fn main() {
    let matches = App::new("Colors")
        .version("0.4")
        .author("Isaac Grosof <isaacbg227@gmail.com>")
        .about("Makes beautiful (giant) images")
        .arg(
            Arg::with_name("size")
                .help(
                    "Cube root of the side length of the image. 0-16 are supported",
                )
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("verbosity")
                .short("v")
                .long("verbosity")
                .takes_value(true)
                .help(
                    "Sets the verbose output frequency. 20000 is a typical value.",
                ),
        )
        .get_matches();
    let size: u32 = matches
        .value_of("size")
        .expect("Size must be provided")
        .parse()
        .expect("Size must be an integer in range");
    let debug_frequency: Option<usize> = matches.value_of("verbosity").map(|verbosity| {
        verbosity.parse().expect(
            "Verbosity must be an integer in range",
        )
    });
    assert!(size <= 16, "Size must be no more than 16");
    let filename = format!("pic{}-{}.png", size, random::<u32>());
    let image = make_image(size, debug_frequency);
    let fout = &mut File::create(&Path::new(&filename)).unwrap();
    image.save(fout, image::PNG).unwrap();
    println!("Saved to {}", &filename);
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;

    #[bench]
    fn size_1(b: &mut Bencher) {
        b.iter(|| make_image(1, None))
    }

    #[bench]
    fn size_2(b: &mut Bencher) {
        b.iter(|| make_image(2, None))
    }


    #[bench]
    fn size_3(b: &mut Bencher) {
        b.iter(|| make_image(3, None))
    }

    #[bench]
    fn size_4(b: &mut Bencher) {
        b.iter(|| make_image(4, None))
    }
}
