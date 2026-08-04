use bson::oid::ObjectId;

/// 与 Java `new ObjectId().toString()` 完全一致的 24 位小写十六进制主键。
pub fn new_object_id() -> String {
    ObjectId::new().to_hex()
}

#[cfg(test)]
mod tests {
    use super::new_object_id;

    #[test]
    fn generated_key_is_24_lowercase_hex_chars() {
        let id = new_object_id();
        assert_eq!(id.len(), 24);
        assert!(id
            .chars()
            .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase()));
    }

    #[test]
    fn generated_keys_are_unique() {
        assert_ne!(new_object_id(), new_object_id());
    }
}
