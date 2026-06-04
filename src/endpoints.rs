#[derive(Clone, Debug)]
pub struct Endpoints {
    /// The bare portal host (no scheme, no path). Kept around so the client
    /// can ask `profiles::host_supports_rich_status(&self.host)` when deciding
    /// whether to surface verified-schema extras in `Status::details`.
    pub host: String,
    pub login: String,
    pub challenge: String,
    pub user_info: String,
    pub online_check: String,
}

impl Endpoints {
    pub fn from_host(host: &str, insecure: bool) -> Self {
        let scheme = if insecure { "http" } else { "https" };
        Self {
            host: host.to_string(),
            login: format!("{scheme}://{host}/cgi-bin/srun_portal"),
            challenge: format!("{scheme}://{host}/cgi-bin/get_challenge"),
            user_info: format!("{scheme}://{host}/cgi-bin/rad_user_info"),
            online_check: format!("{scheme}://{host}/srun_portal_pc"),
        }
    }
}
