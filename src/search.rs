pub fn string_search(needle: &String, haystack: &Vec<String>, max_results: u32) -> Vec<String> {
    let mut results = Vec::<String>::new();

    let mut results_count = 0;
    for item in haystack {
        if item.contains(needle) {
            results.push(item.clone());
            results_count += 1;
            if results_count >= max_results {
                break;
            }
        }
    }

    results
}