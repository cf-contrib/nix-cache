use narinfo::{NarInfo, NixCacheInfo};

#[tokio::test]
async fn test_get_nix_cache_info() {
    let response = reqwest::get("http://127.0.0.1:8787/nix-cache-info")
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
    let response = reqwest::get("http://127.0.0.1:8787/j5m1qd2dbsmhq0mw13yb8wijnm3pq4z0")
        .await
        .expect("the request failed");
    assert_eq!(response.status(), 200);
    let data = response.text().await.expect("the body failed");
    let info = NarInfo::parse(&data).expect("the response failed");
    assert_eq!(info.url, "nar/j5m1qd2dbsmhq0mw13yb8wijnm3pq4z0.nar");
    assert_eq!(
        info.nar_hash,
        "sha256-LHdODcc9LKl8TykaDMvSkpcBrXrTcP8aW2B6trJhxdE="
    );
    assert_eq!(
        info.store_path,
        "/nix/store/j5m1qd2dbsmhq0mw13yb8wijnm3pq4z0-cf-nix-r2-fixture"
    );
    assert_eq!(
        info.deriver.unwrap(),
        "/nix/store/f9x8my1mqpayq9fy7c5mj6xyj4ic6in2-cf-nix-r2-fixture.drv"
    );
}

#[tokio::test]
async fn test_get_nar_info_not_found() {
    let response = reqwest::get("http://127.0.0.1:8787/j6m2qd3dbsmhq0mw14yb9wijnm4pq6z1")
        .await
        .expect("the request failed");
    assert_eq!(response.status(), 404);
}
