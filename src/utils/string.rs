use regex::Regex;

pub fn extract_template_content(s: &str) -> Vec<String> {
    let re = Regex::new(r"\$\{\{\s*(.*?)\s*\}\}").unwrap();
    re.captures_iter(s)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect()
}