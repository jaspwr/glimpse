// Glimpse - GNU/Linux Launcher and File search utility.
// Copyright (C) 2024 https://github.com/jaspwr

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use crate::biases::BIASES;

pub fn word_similarity(needle: &String, item: String, id_hash: &Box<dyn Fn(&str) -> u64>) -> f32 {
    let needle = needle.to_lowercase();
    let item = item.to_lowercase();

    let matched = matched_chars_loose(&needle, &item);

    let mut similarity = matched as f32 + (matched as f32 / item.len() as f32);
    similarity /= needle.len() as f32;

    if item == *needle {
        similarity += 8.0;
    } else if item.starts_with(&needle) {
        similarity += 4.0;
    } else if item.contains(&needle) {
        similarity += 3.5;
    }

    if let Some(bias) = BIASES.map.get(&id_hash(&item)) {
        similarity += bias;
    }

    if matched == 0 {
        similarity = 0.0;
    }
    if needle.len() > 1 && matched < 2 {
        similarity = 0.0;
    }

    similarity
}

fn matched_chars_loose(checking: &String, against: &String) -> u32 {
    let mut ret: u32 = 0;
    let mut against_char = against.chars();
    loop {
        if ret as usize >= checking.len() {
            break;
        }

        let next_char = match against_char.next() {
            Some(next_char) => next_char,
            None => break,
        };

        if checking.chars().nth(ret as usize).unwrap() == next_char {
            ret += 1;
        }
    }
    ret
}
