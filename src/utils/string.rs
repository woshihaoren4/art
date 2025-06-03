use regex::Regex;

pub fn extract_template_content(s: &str) -> Vec<String> {
    let re = Regex::new(r"\$\{\{\s*(.*?)\s*\}\}").unwrap();
    re.captures_iter(s)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_extract_template_content() {
        let res = extract_template_content(r#"${{aaa}}"#);
        assert_eq!(res[0], "aaa");
        let res = extract_template_content(r#"fda&*(h${{aaa}}430&)"#);
        assert_eq!(res[0], "aaa");
        let res = extract_template_content(r#"fda&*(h${{hello.world}}430&)"#);
        assert_eq!(res[0], "hello.world");
        let res = extract_template_content(r#"f${{hello}}da&*(h${{world}}430&)"#);
        assert_eq!(res[0], "hello");
        assert_eq!(res[1], "world");
    }
}
