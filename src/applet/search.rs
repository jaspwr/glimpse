use crate::{biases::{Biases, BIASES}};

pub fn string_search(
    needle: &String,
    haystack: &Vec<String>,
    max_results: u32,
    id_hash: Box<dyn Fn(&String) -> u64>,
    case_sensitive: bool,
) -> Vec<(String, f32)> {
    let mut results = Vec::<(String, f32)>::new();

    let mut needle = needle.clone();
    if !case_sensitive {
        needle = needle.to_lowercase();
    }

    let mut worst_sim: f32 = 0.0;
    for item_cased in haystack {
        let mut item = item_cased.clone();
        if !case_sensitive {
            item = item.to_lowercase();
        }

        let matched = matched_chars_loose(&needle, &item);

        let mut similarity = matched as f32 + (matched as f32 / item.len() as f32);
        similarity /= needle.len() as f32;

        if item == needle {
            similarity += 4.0;
        } else if item.starts_with(&needle) {
            similarity += 1.0;
        } else if item.contains(&needle) {
            similarity += 0.8;
        }

        if let Some(bias) = BIASES.map.get(&id_hash(&item)) {
            similarity += bias;
        }

        if results.len() < max_results as usize || similarity > worst_sim {
            if matched == 0 {
                continue;
            }
            if needle.len() > 1 && matched < 2 {
                continue;
            }
            worst_sim = similarity;

            results.push((item_cased.clone(), similarity));
            results.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap());
            if results.len() > max_results as usize {
                results.pop();
            }
        }
    }

    results
        .into_iter()
        .filter(|(_, relevance)| *relevance > 0.7)
        .collect()
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
