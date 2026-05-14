use narinfo::NixCacheInfo;

#[tokio::test]
async fn test_get_nix_cache_info() {
    let response = reqwest::get("http://127.0.0.1:8787/nix-cache-info")
        .await
        .expect("the request failed");
    assert_eq!(response.status(), 200);
    let data = response.text().await.expect("the body failed");
    let info = NixCacheInfo::parse(&data).expect("the response failed");
    assert_eq!(info.store_dir, "/nix/store");
    assert_ne!(info.wants_mass_query, true);
    assert_eq!(info.priority, 40);
}
