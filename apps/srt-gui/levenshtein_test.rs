fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    
    let a_len = a_chars.len();
    let b_len = b_chars.len();
    
    if a_len == 0 { return b_len; }
    if b_len == 0 { return a_len; }
    
    let mut dp = vec![vec![0; b_len + 1]; a_len + 1];
    
    for i in 0..=a_len { dp[i][0] = i; }
    for j in 0..=b_len { dp[0][j] = j; }
    
    for i in 1..=a_len {
        for j in 1..=b_len {
            let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }
    
    dp[a_len][b_len]
}

fn text_similarity(a: &str, b: &str) -> f64 {
    let len_a = a.chars().count();
    let len_b = b.chars().count();
    if len_a == 0 && len_b == 0 {
        return 1.0;
    }
    let max_len = len_a.max(len_b) as f64;
    let dist = levenshtein_distance(a, b) as f64;
    1.0 - (dist / max_len)
}
