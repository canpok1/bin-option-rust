use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    host: String,
    port: i32,
}

impl Config {
    pub fn get_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_for_get_address() {
        let config = Config {
            host: "127.0.0.1".to_string(),
            port: 8888,
        };
        assert_eq!(config.get_address(), "127.0.0.1:8888".to_string());
    }
}
