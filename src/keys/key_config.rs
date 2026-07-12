#[derive(Debug, Clone)]
pub struct KeyConfig {
    pub keys: crate::keys::KeyList,
}

impl Default for KeyConfig {
    fn default() -> Self {
        Self { keys: crate::keys::KeyList::default() }
    }
}

impl KeyConfig {
    pub fn init() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_config_methods() {
        let _ = KeyConfig::init();
        let _ = KeyConfig::default();
    }
}
