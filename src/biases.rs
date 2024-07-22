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

use crate::config::CONF;
use once_cell::sync::Lazy;
use savefile_derive::Savefile;
use std::{collections::HashMap, path::PathBuf};

pub static BIASES: Lazy<Biases> = Lazy::new(|| Biases::load("biases"));

#[derive(Savefile, Clone)]
pub struct Biases {
    pub map: HashMap<u64, f32>,
}

impl Biases {
    fn new() -> Biases {
        Biases {
            map: HashMap::new(),
        }
    }

    pub fn load(name: &str) -> Biases {
        match std::fs::File::open(
            PathBuf::from(&CONF.indexing.location)
                .join(name)
                .with_extension("bin"),
        ) {
            Ok(mut file) => match savefile::load(&mut file, 0) {
                Ok(biases) => biases,
                Err(_) => Biases::new(),
            },
            Err(_) => Biases::new(),
        }
    }

    pub fn save(&self, name: &str) {
        let path = PathBuf::from(&CONF.indexing.location)
            .join(name)
            .with_extension("bin");
        let mut file = std::fs::File::create(path).unwrap();
        savefile::save(&mut file, 0, self).unwrap();
    }
}

pub fn increment_bias(id: u64, amount: f32) {
    let mut biases = BIASES.clone();

    let mut bias = 0.0;

    if let Some(current) = biases.map.get(&id) {
        bias = *current;
    }

    bias += amount;

    if bias > 2.5 {
        bias = 2.5;
    }

    biases.map.insert(id, bias);

    println!("Incremented bias for {} to {}", id, bias);

    biases.save("biases");
}
