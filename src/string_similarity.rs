use crate::biases::BIASES;

pub fn word_similarity(needle: &String, item: String, id_hash: &Box<dyn Fn(&str) -> u64>) -> f32 {
    let matched = matched_chars_loose(needle, &item);

    let mut similarity = matched as f32 + (matched as f32 / item.len() as f32);
    similarity /= needle.len() as f32;

    if item == *needle {
        similarity += 8.0;
    } else if item.starts_with(needle) {
        similarity += 4.0;
    } else if item.contains(needle) {
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
