#[derive(Debug, Clone)]
pub struct GetConnectionStringOptions<'a>  {
    pub container_id_or_name: &'a str,
    pub verify: Option<bool>,
}
