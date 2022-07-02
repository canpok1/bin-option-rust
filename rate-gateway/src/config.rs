use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub server_host: String,
    pub server_port: i32,
}

impl Config {
    pub fn get_address(&self) -> String {
        format!("{}:{}", self.server_host, self.server_port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_for_get_address() {
        let config = Config {
            server_host: "127.0.0.1".to_string(),
            server_port: 8888,
        };
        assert_eq!(config.get_address(), "127.0.0.1:8888".to_string());
    }
}
