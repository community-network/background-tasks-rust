pub mod old_games;
pub mod server_manager;
pub mod companion;
pub mod battlelog;
pub mod bf2042;

pub async fn push_to_database(influx_client: &influxdb2::Client, game_name: &str, frontend_game_name: &str, platform: &str) -> anyhow::Result<()> {
    
                // TODO: do something with it
    Ok(())
}