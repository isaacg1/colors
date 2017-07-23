#![feature(test)]
#![warn(clippy_pedantic)]
#![allow(print_stdout)]
#![warn(warnings)]
#![allow(unknown_lints, missing_docs_in_private_items, cast_sign_loss,
         cast_possible_truncation, cast_precision_loss)]
extern crate test;

extern crate image;
use image::{ImageBuffer, DynamicImage};

#[macro_use]
extern crate itertools;

extern crate rand;
use rand::{thread_rng, Rng, random};

extern crate clap;
use clap::{Arg, App};

use std::fs::File;
use std::path::Path;

use std::collections::{HashSet, HashMap};
use std::hash::Hash;

use std::u32;

use std::time::Instant;

use std::mem::swap;

type Color = (u8, u8, u8);

type Location = (u16, u16);

type RegionId = usize;

type Frontier = VecSet<Location>;

// Synchronized to always have the same data.
struct VecSet<T> {
    vec: Vec<T>,
    set: HashSet<T>,
}

impl<T> VecSet<T>
where
    T: Clone + Eq + Hash,
{
    fn new() -> Self {
        Self {
            vec: Vec::new(),
            set: HashSet::new(),
        }
    }
    fn insert(&mut self, value: T) {
        let was_not_present = self.set.insert(value.clone());
        if was_not_present {
            self.vec.push(value)
        }
    }
    fn remove(&mut self, index: usize) {
        let value = self.vec.swap_remove(index);
        let was_present = self.set.remove(&value);
        assert!(was_present);
    }
    // Takes ownership of other and drops it on purpose.
    fn consume<U>(&mut self, other: Self, ignore: &HashMap<T, Option<U>>)
    where
        U: Eq,
    {
        {
            let set = &self.set;
            let to_add = other
                .vec
                .iter()
                .filter(|location| {
                    ignore.get(location) == Some(&None) && !set.contains(location)
                })
                .cloned();
            self.vec.extend(to_add);
        }
        self.set.extend(other.vec.into_iter().filter(|location| {
            ignore.get(location) == Some(&None)
        }));
    }
    fn iter(&self) -> std::slice::Iter<T> {
        self.vec.iter()
    }
    fn is_empty(&self) -> bool {
        self.vec.is_empty()
    }
    fn len(&self) -> usize {
        self.vec.len()
    }
}



fn squared_location_distance(loc: &Location, oth_loc: &Location) -> i32 {
    let dx = loc.0 as i32 - oth_loc.0 as i32;
    let dy = loc.1 as i32 - oth_loc.1 as i32;

    dx * dx + dy * dy
}

fn maybe_print_debug_info(
    debug_frequency: Option<usize>,
    pixel_index: usize,
    size: u32,
    time: &mut Instant,
    frontiers: &[Frontier],
) {
    if let Some(debug_frequency) = debug_frequency {
        if pixel_index > 0 && pixel_index % debug_frequency == 0 {
            let time_per_pixel = (time.elapsed() / debug_frequency as u32).subsec_nanos() as f64 /
                10_f64.powi(9);
            println!(
                "Completed {} out of {} pixels,  {:.6} milliseconds per pixel\n\
                     Approximately {:.1} sec to go.\n\
                     {} frontier(s) with {} pixels exist.",
                pixel_index,
                size.pow(6),
                time_per_pixel * 1000_f64,
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
    color_offsets: &[(i16, i16, i16)],
    assigned_colors: &'a HashMap<Color, (Location, RegionId)>,
) -> &'a (Location, RegionId) {
    color_offsets
        .iter()
        .filter_map(|offset| {
            let new0 = color.0 as i16 + offset.0;
            let new1 = color.1 as i16 + offset.1;
            let new2 = color.2 as i16 + offset.2;
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
    source_region: RegionId,
    target_region: RegionId,
    debug: bool,
    frontiers: &mut [Frontier],
    locations_to_regions: &mut HashMap<Location, Option<RegionId>>,
    assigned_colors: &mut HashMap<Color, (Location, RegionId)>,
) {
    if debug {
        println!("Collapsing {} into {}", source_region, target_region);
    }
    let mut temp_frontier = Frontier::new();
    swap(&mut temp_frontier, &mut frontiers[source_region]);
    frontiers[target_region].consume(temp_frontier, &locations_to_regions);
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
    let color_range_vec: Vec<u8> = (0..color_range).map(|color| color as u8).collect();
    let color_multiplier = 256_f64 / color_range as f64;
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
        let mut color_offsets: Vec<(i16, i16, i16)> = iproduct!(
            -(color_range as i16)..color_range as i16,
            -(color_range as i16)..color_range as i16,
            -(color_range as i16)..color_range as i16
        ).collect();
        color_offsets.sort_by_key(|offset| {
            (offset.0 as i32).pow(2) + (offset.1 as i32).pow(2) + (offset.2 as i32).pow(2)
        });
        color_offsets
    };
    let mut locations_to_regions: HashMap<Location, Option<RegionId>> =
        iproduct!(0..side_length as u16, 0..side_length as u16)
            .map(|location| (location, None))
            .collect();
    assert_eq!(colors.len(), locations_to_regions.len());
    let mut frontiers: Vec<Frontier> = (0..random_locs).map(|_| Frontier::new()).collect();
    let mut assigned_colors: HashMap<Color, (Location, RegionId)> = HashMap::new();
    let mut img = ImageBuffer::new(side_length, side_length);
    let mut time = Instant::now();
    for (i, color) in colors.into_iter().enumerate() {
        maybe_print_debug_info(debug_frequency, i, size, &mut time, &frontiers);
        let (location, frontier_index, index_in_frontier) = if i >= random_locs as usize {
            let &(target_cell, frontier_index) =
                find_target_cell_and_frontier(color, &color_offsets, &assigned_colors);
            let (index_in_frontier, &location) = frontiers[frontier_index]
                .iter()
                .enumerate()
                .min_by_key(|&(_, loc)| squared_location_distance(&target_cell, loc))
                .expect("There's at least one left");
            (location, frontier_index, Some(index_in_frontier))
        } else {
            let location = loop {
                let &(&location, region) = thread_rng()
                    .choose(&locations_to_regions.iter().collect::<Vec<_>>())
                    .expect("There's plenty_left");
                if region.is_none() {
                    break location;
                }
            };
            (location, i, None)
        };
        let previous_region = locations_to_regions.insert(location, Some(frontier_index));
        assert_eq!(previous_region, Some(None));
        if let Some(index) = index_in_frontier {
            frontiers[frontier_index].remove(index);
        }
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
        img.put_pixel(location.0 as u32, location.1 as u32, pixel);
    }
    image::ImageRgb8(img)
}

fn main() {
    let matches = App::new("Colors")
        .version("0.5")
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
    let fout =
        &mut File::create(&Path::new(&filename)).expect("Create the file to save in should work");
    image.save(fout, image::PNG).expect(
        "Saving should just work.",
    );
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
