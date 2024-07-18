use glimpse::string_similarity::word_similarity;

use crate::utils::HashFn;

pub fn string_search(
    needle: &String,
    haystack: &Vec<String>,
    max_results: u32,
    id_hash: &HashFn,
    case_sensitive: bool,
) -> Vec<(String, f32)> {
    let mut results = Vec::<(String, f32)>::new();

    if needle.is_empty() {
        return results;
    }

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

        let similarity = word_similarity(&needle, item, id_hash);

        if results.len() < max_results as usize || similarity > worst_sim {
            if similarity == 0.0 {
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
