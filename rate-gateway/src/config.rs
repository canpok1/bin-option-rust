use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    // サーバー関連
    pub server_host: String,
    pub server_port: i32,

    // DB関連
    pub db_host: String,
    pub db_port: u16,
    pub db_name: String,
    pub db_user_name: String,
    pub db_password: String,
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
            db_host: "dummy_host".to_string(),
            db_port: 3306,
            db_name: "dummy_db".to_string(),
            db_user_name: "dummy_user".to_string(),
            db_password: "dummy_password".to_string(),
        };
        assert_eq!(config.get_address(), "127.0.0.1:8888".to_string());
    }
}
