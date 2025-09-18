#[derive(Debug, Clone, Default)]
pub struct GetConnectionStringOptions {
    pub container_id_or_name: String,
    pub db_username: Option<String>,
    pub db_password: Option<String>,
    pub verify: Option<bool>,
    pub docker_hostname: Option<String>,
}
