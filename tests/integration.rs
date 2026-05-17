mod helper;

use narinfo::{NarInfo, NixCacheInfo};

#[tokio::test]
async fn test_get_nix_cache_info() {
    let response = helper::get("nix-cache-info")
        .await
        .expect("the request failed");
    assert_eq!(response.status(), 200);
    let data = response.text().await.expect("the body failed");
    let info = NixCacheInfo::parse(&data).expect("the response failed");
    assert_eq!(info.store_dir, "/nix/store");
    assert!(!info.wants_mass_query);
    assert_eq!(info.priority, 40);
}

#[tokio::test]
async fn test_get_nar_info() {
    let file_type = "text/x-nix-narinfo";
    let file_name = "j5m1qd2dbsmhq0mw13yb8wijnm3pq4z0.narinfo";
    let file_path = format!("tests/fixture/{file_name}");
    let file_data = std::fs::read_to_string(file_path).unwrap();

    let post_resp = helper::put(file_name, file_type, file_data.clone())
        .await
        .expect("the request failed");
    assert_eq!(post_resp.status(), 200);

    let get_resp = helper::get(file_name).await.expect("the request failed");
    assert_eq!(get_resp.status(), 200);
    assert_eq!(get_resp.headers().get("content-type").unwrap(), file_type);

    let resp_body = get_resp.text().await.expect("the body failed");

    let local_info = NarInfo::parse(&file_data).unwrap();
    let server_info = NarInfo::parse(&resp_body).expect("the response failed");
    assert_eq!(server_info.url, local_info.url);
    assert_eq!(server_info.nar_hash, local_info.nar_hash);
    assert_eq!(server_info.store_path, local_info.store_path);
    assert_eq!(server_info.deriver.unwrap(), local_info.deriver.unwrap(),);
}

#[tokio::test]
async fn test_get_nar_info_not_found() {
    let get_resp = helper::get("j6m2qd3dbsmhq0mw14yb9wijnm4pq6z1.narinfo")
        .await
        .expect("the request failed");
    assert_eq!(get_resp.status(), 404);
}

#[tokio::test]
async fn test_get_nar() {
    let file_type = "application/x-nix-archive";
    let file_name = "j5m1qd2dbsmhq0mw13yb8wijnm3pq4z0.nar";
    let file_path = format!("tests/fixture/{file_name}");
    let file_url = format!("nar/{file_name}");
    let file_data = std::fs::read(file_path).unwrap();

    let post_resp = helper::put(&file_url, file_type, file_data.clone())
        .await
        .expect("the request failed");
    assert_eq!(post_resp.status(), 200);

    let get_resp = helper::get(&file_url).await.expect("the request failed");
    assert_eq!(get_resp.status(), 200);
    assert_eq!(get_resp.headers().get("content-type").unwrap(), file_type,);

    let bytes = get_resp.bytes().await.expect("body read failed");
    assert!(!bytes.is_empty(), "nar body should not be empty");
    assert_eq!(bytes.len(), 256);
}

#[tokio::test]
async fn test_get_nar_not_found() {
    let get_resp = helper::get("nar/j6m2qd3dbsmhq0mw14yb9wijnm4pq6z1.nar")
        .await
        .expect("the request failed");
    assert_eq!(get_resp.status(), 404);
}
