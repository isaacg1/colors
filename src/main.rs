#![feature(test)]
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

use std::{u8, i32, u32};

use std::time::Instant;

use std::mem::swap;

type Color = [u8; 3];

type Location = [u16; 2];

type RegionId = usize;

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
    fn remove(&mut self, index: usize) -> T {
        let value = self.vec.swap_remove(index);
        let was_present = self.set.remove(&value);
        assert!(was_present);
        value
    }
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

struct Frontier {
    squares: Vec<Vec<VecSet<Location>>>,
    square_size: usize,
    pixel_size: usize,
}

impl Frontier {
    fn new(square_size: usize, pixel_size: usize) -> Self {
        Frontier {
            squares: (0..square_size)
                .map(|_| (0..square_size).map(|_| VecSet::new()).collect())
                .collect(),
            square_size: square_size as usize,
            pixel_size: pixel_size as usize,
        }
    }
    fn new_like(other: &Self) -> Self {
        Frontier::new(other.square_size, other.pixel_size)
    }
    fn get_square_mut(&mut self, loc: &Location) -> &mut VecSet<Location> {
        &mut self.squares[loc[0] as usize * self.square_size / self.pixel_size]
            [loc[1] as usize * self.square_size / self.pixel_size]
    }
    fn insert(&mut self, loc: Location) {
        self.get_square_mut(&loc).insert(loc)
    }
    fn consume<U>(&mut self, other: Self, ignore: &HashMap<Location, Option<U>>)
    where
        U: Eq,
    {
        for (our_square, oth_square) in
            self.squares.iter_mut().flat_map(|vec| vec.iter_mut()).zip(
                other.squares.into_iter().flat_map(|vec| vec.into_iter()),
            )
        {
            our_square.consume(oth_square, ignore)
        }
    }
    fn extract_nearest_neighbor(&mut self, target: &Location) -> Location {
        let loc_r = target[0] as usize * self.square_size / self.pixel_size;
        let loc_c = target[1] as usize * self.square_size / self.pixel_size;
        let pixel_size = self.pixel_size;
        let square_size = self.square_size;
        let closest_pixel_in_square = |target_i, square_i, base| if target_i == square_i {
            base
        } else if target_i < square_i {
            (square_i * pixel_size / square_size) as u16
        } else {
            ((square_i + 1) * pixel_size / square_size - 1) as u16
        };
        let (square, index) = {
            let mut best_square_and_index = None;
            let mut best_distance = i32::MAX;
            let mut data: Vec<_> = self.squares
                .iter()
                .enumerate()
                .flat_map(|(ri, vec)| {
                    vec.iter().enumerate().map(
                        move |(ci, vecset)| ((ri, ci), vecset),
                    )
                })
                .map(|((ri, ci), square)| {
                    let closest_r = closest_pixel_in_square(loc_r, ri, target[0]);
                    let closest_c = closest_pixel_in_square(loc_c, ci, target[1]);
                    ((ri, ci), square, [closest_r, closest_c])
                })
                .collect();
            data.sort_by_key(|&(_, _, closest)| {
                squared_location_distance(&target, &closest)
            });
            for (indices, square, closest) in data {
                if squared_location_distance(&target, &closest) < best_distance {
                    if let Some((index_in_frontier, loc)) =
                        square.iter().enumerate().min_by_key(|&(_, loc)| {
                            squared_location_distance(&target, loc)
                        })
                    {
                        let distance = squared_location_distance(&target, loc);
                        if distance < best_distance {
                            best_distance = distance;
                            best_square_and_index = Some((indices, index_in_frontier));
                        }
                    }
                }
            }
            best_square_and_index.expect("There's at least one left")
        };
        self.squares[square.0][square.1].remove(index)
    }
    fn is_empty(&self) -> bool {
        self.squares.iter().flat_map(|vec| vec.iter()).all(
            |vecset| {
                vecset.is_empty()
            },
        )
    }
    fn len(&self) -> usize {
        self.squares
            .iter()
            .flat_map(|vec| vec.iter())
            .map(|vecset| vecset.len())
            .sum()
    }
}

fn squared_location_distance(loc: &Location, oth_loc: &Location) -> i32 {
    let dx = loc[0] as i32 - oth_loc[0] as i32;
    let dy = loc[1] as i32 - oth_loc[1] as i32;

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
                "Completed {} out of {} pixels,  {:.3} microseconds per pixel\n\
                     Approximately {:.1} sec to go.\n\
                     {} frontier(s) with {} pixels exist.",
                pixel_index,
                size.pow(6),
                time_per_pixel * 1000_000_f64,
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
    color_offsets: &[[i16; 3]],
    assigned_colors: &'a HashMap<Color, (Location, RegionId)>,
) -> &'a (Location, RegionId) {
    color_offsets
        .iter()
        .filter_map(|offset| {
            let new0 = color[0] as i16 + offset[0];
            let new1 = color[1] as i16 + offset[1];
            let new2 = color[2] as i16 + offset[2];
            if 0 <= new0 && new0 <= u8::MAX as i16 && 0 <= new1 && new1 <= u8::MAX as i16 &&
                0 <= new2 && new2 <= u8::MAX as i16
            {
                let color = [new0 as u8, new1 as u8, new2 as u8];
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
    let mut temp_frontier = Frontier::new_like(&frontiers[source_region]);
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
fn make_image(size: u32, frontier_groups: usize, debug_frequency: Option<usize>) -> DynamicImage {
    assert!(size <= 16);
    let color_range = size * size;
    let color_multiplier = u8::MAX as f64 / color_range as f64;
    let side_length = size * size * size;
    let random_locs = size * 2;
    let colors = {
        let color_range_vec: Vec<u8> = (0..color_range).map(|color| color as u8).collect();
        let mut colors: Vec<Color> = iproduct!(
            color_range_vec.iter().cloned(),
            color_range_vec.iter().cloned(),
            color_range_vec.iter().cloned()
        ).map(|(a, b, c)| [a, b, c])
            .collect();
        thread_rng().shuffle(&mut colors);
        colors
    };
    let color_offsets: Vec<[i16; 3]> = {
        let mut positive_color_offsets: Vec<[i16; 3]> = iproduct!(
            0..color_range as i16,
            0..color_range as i16,
            0..color_range as i16
        ).map(|(a, b, c)| [a, b, c])
            .collect();
        positive_color_offsets.sort_by_key(|offset| {
            (offset[0] as i32).pow(2) + (offset[1] as i32).pow(2) + (offset[2] as i32).pow(2)
        });
        let mut color_offsets = Vec::new();
        for offset in positive_color_offsets {
            color_offsets.extend(
                &[
                    [offset[0], offset[1], offset[2]],
                    [-offset[0], offset[1], offset[2]],
                    [offset[0], -offset[1], offset[2]],
                    [offset[0], offset[1], -offset[2]],
                    [-offset[0], -offset[1], offset[2]],
                    [-offset[0], offset[1], -offset[2]],
                    [offset[0], -offset[1], -offset[2]],
                    [-offset[0], -offset[1], -offset[2]],
                ],
            )
        }
        color_offsets
    };
    let mut locations_to_regions: HashMap<Location, Option<RegionId>> =
        iproduct!(0..side_length as u16, 0..side_length as u16)
            .map(|location| ([location.0, location.1], None))
            .collect();
    assert_eq!(colors.len(), locations_to_regions.len());
    let mut frontiers: Vec<Frontier> = (0..random_locs)
        .map(|_| Frontier::new(frontier_groups, side_length as usize))
        .collect();
    let mut assigned_colors: HashMap<Color, (Location, RegionId)> = HashMap::new();
    let mut img = ImageBuffer::new(side_length, side_length);
    let mut time = Instant::now();
    for (i, color) in colors.into_iter().enumerate() {
        maybe_print_debug_info(debug_frequency, i, size, &mut time, &frontiers);
        let (location, frontier_index) = if i >= random_locs as usize {
            let &(target_cell, frontier_index) =
                find_target_cell_and_frontier(color, &color_offsets, &assigned_colors);
            let location = frontiers[frontier_index].extract_nearest_neighbor(&target_cell);
            (location, frontier_index)
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
        for neighbor in &[
            [location[0] + 1, location[1]],
            [location[0], location[1] + 1],
            [location[0].saturating_sub(1), location[1]],
            [location[0], location[1].saturating_sub(1)],
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
                (color[0] as f64 * color_multiplier) as u8,
                (color[1] as f64 * color_multiplier) as u8,
                (color[2] as f64 * color_multiplier) as u8,
            ],
        );
        img.put_pixel(location[0] as u32, location[1] as u32, pixel);
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
        .arg(
            Arg::with_name("frontier_groups")
                .short("f")
                .long("frontier")
                .takes_value(true)
                .help("Sets the root of the number of buckets a frontier occupies"),
        )
        .get_matches();
    let size: u32 = matches
        .value_of("size")
        .expect("Size must be provided")
        .parse()
        .expect("Size must be an integer in range");
    let frontier_groups = matches.value_of("frontier_groups").map_or(1, |frontier| {
        frontier.parse().expect(
            "Frontier groups must be an integer in range",
        )
    });
    let debug_frequency: Option<usize> = matches.value_of("verbosity").map(|verbosity| {
        verbosity.parse().expect(
            "Verbosity must be an integer in range",
        )
    });
    assert!(size <= 16, "Size must be no more than 16");
    let filename = format!("pic{}-{}.png", size, random::<u32>());
    let image = make_image(size, frontier_groups, debug_frequency);
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
