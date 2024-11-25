use sqlx::postgres::PgPool;

use crate::structs::server_info;

pub async fn push_server(
    pool: &PgPool,
    frontend_game_name: &str,
    region: &str,
    platform: &str,
    server_infos: Vec<server_info::ServerInfo>,
) -> anyhow::Result<()> {
    let mut server_names: Vec<String> = vec![];
    let mut soldier_amounts: Vec<i64> = vec![];
    let mut queue_amounts: Vec<i64> = vec![];
    let mut guids: Vec<Option<String>> = vec![];
    let mut game_ids: Vec<Option<String>> = vec![];
    let mut modes: Vec<Option<String>> = vec![];
    let mut maps: Vec<Option<String>> = vec![];
    let mut is_officials: Vec<Option<bool>> = vec![];
    for server_info in server_infos {
        if !server_info.name.is_empty() {
            server_names.push(server_info.name);
            soldier_amounts.push(server_info.soldiers);
            queue_amounts.push(server_info.queue);
            game_ids.push(match !server_info.game_id.is_empty() {
                true => Some(server_info.game_id),
                false => None,
            });
            guids.push(match !server_info.guid.is_empty() {
                true => Some(server_info.guid),
                false => None,
            });
            modes.push(match !server_info.mode.is_empty() {
                true => Some(server_info.mode),
                false => None,
            });
            maps.push(match !server_info.map.is_empty() {
                true => Some(server_info.map),
                false => None,
            });
            is_officials.push(server_info.is_official);
        }
    }

    sqlx::query!(
        "
            INSERT INTO battlefield_servers(game, region, platform, servername, is_official, game_id, guid, game_mode, game_map, soldier_amount, queue_amount) 
            SELECT * FROM UNNEST($1::text[], $2::text[], $3::text[], $4::text[], $5::bool[], $6::text[], $7::text[], $8::text[], $9::text[], $10::int8[], $11::int8[])
        ",
        &vec![frontend_game_name.to_string(); server_names.len()][..],
        &vec![region.to_string(); server_names.len()][..],
        &vec![platform.to_string(); server_names.len()][..],
        &server_names[..],
        // Due to a limitation in how SQLx typechecks query parameters, `Vec<Option<T>>` is unable to be typechecked.
        // This demonstrates the explicit type override syntax, which tells SQLx not to typecheck these parameters.
        // See the documentation for `query!()` for more details.
        &is_officials as &[Option<bool>],
        &game_ids as &[Option<String>],
        &guids as &[Option<String>],
        &modes as &[Option<String>],
        &maps as &[Option<String>],
        &soldier_amounts[..],
        &queue_amounts[..]
    )
        .execute(pool)
        .await?;
    Ok(())
}
