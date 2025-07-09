pub fn slugify(text: &str) -> String {
    text.to_lowercase()
        .replace(|c: char| !c.is_alphanumeric() && c != ' ', "")
        .trim()
        .replace(' ', "-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify_simple() {
        assert_eq!(slugify("Hello World"), "hello-world");
    }

    #[test]
    fn test_slugify_with_special_chars() {
        assert_eq!(slugify("Hello, World! 123"), "helloworld-123");
    }

    #[test]
    fn test_slugify_extra_spaces() {
        assert_eq!(slugify("  leading and trailing spaces  "), "leading-and-trailing-spaces");
    }

    #[test]
    fn test_slugify_empty() {
        assert_eq!(slugify(""), "");
    }

    #[test]
    fn test_slugify_already_slug() {
        assert_eq!(slugify("already-a-slug"), "already-a-slug");
    }

    #[test]
    fn test_slugify_mixed_case() {
        assert_eq!(slugify("MixedCase Slug"), "mixedcase-slug");
    }
}
