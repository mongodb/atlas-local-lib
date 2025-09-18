#[derive(Debug, Clone, Default)]
pub struct GetConnectionStringOptions<'a> {
    pub container_id_or_name: &'a str,
    pub db_username: Option<&'a str>,
    pub db_password: Option<&'a str>,
    pub verify: Option<bool>,
    pub docker_hostname: Option<&'a str>,
}
