use reqwest::header::HeaderMap;

pub async fn get_session_info(ea_access_token: String) -> anyhow::Result<bool> {
    let url = "https://gateway.ea.com/proxy/identity/pids/me/personas".to_string();
    let mut headers = HeaderMap::new();
    headers.insert("Accept", "application/json".parse()?);
    headers.insert(
        "Authorization",
        format!("Bearer {}", ea_access_token).parse()?,
    );
    headers.insert("X-Expand-Results", "true".parse()?);
    let client = reqwest::Client::builder().build()?;
    let resp = client.get(url).headers(headers).send().await?;

    Ok(resp.status() == 200)
}
