#[derive(Clone, Debug)]
pub struct Endpoints {
    pub login: String,
    pub challenge: String,
    pub user_info: String,
    pub online_check: String,
}

impl Endpoints {
    pub fn from_host(host: &str, insecure: bool) -> Self {
        let scheme = if insecure { "http" } else { "https" };
        Self {
            login: format!("{scheme}://{host}/cgi-bin/srun_portal"),
            challenge: format!("{scheme}://{host}/cgi-bin/get_challenge"),
            user_info: format!("{scheme}://{host}/cgi-bin/rad_user_info"),
            online_check: format!("{scheme}://{host}/srun_portal_pc"),
        }
    }
}
