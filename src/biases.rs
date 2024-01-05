use crate::file_index;
use once_cell::sync::Lazy;
use savefile_derive::Savefile;
use std::collections::HashMap;

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
        match std::fs::File::open(file_index::PATH.join(name).with_extension("bin")) {
            Ok(mut file) => match savefile::load(&mut file, 0) {
                Ok(biases) => biases,
                Err(_) => Biases::new(),
            },
            Err(_) => Biases::new(),
        }
    }

    pub fn save(&self, name: &str) {
        let path = file_index::PATH.join(name).with_extension("bin");
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
