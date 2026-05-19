use std::{borrow::Cow, fmt::Write};

use base64::{engine::general_purpose::STANDARD, Engine};
use http_auth_basic::Credentials;
use narinfo::*;
use worker::*;

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_narinfo(body: &str) -> NarInfo<'_> {
        NarInfo::parse(body).expect("narinfo should parse")
    }

    #[test]
    fn validate_narinfo_upload_ok_minimal() {
        let body = "StorePath: /nix/store/abc-min\nURL: nar/abc.nar\nNarHash: sha256-LHdODcc9LKl8TykaDMvSkpcBrXrTcP8aW2B6trJhxdE=\nNarSize: 1\nReferences: \nCompression: none";
        let info = parse_narinfo(body);
        assert!(validate_narinfo_upload("abc", &info).is_ok());
    }

    #[test]
    fn validate_narinfo_upload_rejects_store_path() {
        let body = "StorePath: /tmp/abc\nURL: nar/abc.nar\nNarHash: sha256-LHdODcc9LKl8TykaDMvSkpcBrXrTcP8aW2B6trJhxdE=\nNarSize: 1\nReferences: ";
        let info = parse_narinfo(body);
        let err = validate_narinfo_upload("abc", &info).unwrap_err();
        assert!(err.contains("StorePath"));
    }

    #[test]
    fn validate_narinfo_upload_rejects_url_mismatch() {
        let body = "StorePath: /nix/store/abc-min\nURL: nar/zzz.nar\nNarHash: sha256-LHdODcc9LKl8TykaDMvSkpcBrXrTcP8aW2B6trJhxdE=\nNarSize: 1\nReferences: ";
        let info = parse_narinfo(body);
        let err = validate_narinfo_upload("abc", &info).unwrap_err();
        assert!(err.contains("URL"));
    }

    #[test]
    fn validate_narinfo_upload_rejects_nar_size_zero() {
        let body = "StorePath: /nix/store/abc-min\nURL: nar/abc.nar\nNarHash: sha256-LHdODcc9LKl8TykaDMvSkpcBrXrTcP8aW2B6trJhxdE=\nNarSize: 0\nReferences: ";
        let info = parse_narinfo(body);
        let err = validate_narinfo_upload("abc", &info).unwrap_err();
        assert!(err.contains("NarSize"));
    }

    #[test]
    fn validate_narinfo_upload_rejects_bad_compression() {
        let body = "StorePath: /nix/store/abc-min\nURL: nar/abc.nar\nNarHash: sha256-LHdODcc9LKl8TykaDMvSkpcBrXrTcP8aW2B6trJhxdE=\nNarSize: 1\nReferences: \nCompression: gzip";
        let info = parse_narinfo(body);
        let err = validate_narinfo_upload("abc", &info).unwrap_err();
        assert!(err.contains("Compression"));
    }

    #[test]
    fn validate_sha256_hash_field_accepts_base32_colon_format() {
        let value = "sha256:0c8ld5yxcr6a6j63mvrqbqiy08q6f85wd74817ai7pvd5nkidcqw";
        assert!(validate_sha256_hash_field("NarHash", value).is_ok());
    }

    #[test]
    fn validate_sha256_hash_field_rejects_invalid_base64_dash_format() {
        let value = "sha256-not_base64";
        let err = validate_sha256_hash_field("NarHash", value).unwrap_err();
        assert!(err.contains("sha256-<base64>"));
    }
}

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    if !authorize(&req, &env) {
        return Response::error("access denied", 401);
    }

    Router::new()
        .get("/nix-cache-info", get_nix_cache_info)
        .post_async("/", post_mass_query)
        .get_async("/:hash", get_nar_info)
        .put_async("/:hash", put_nar_info)
        .get_async("/nar/:hash", get_nar)
        .put_async("/nar/:hash", put_nar)
        .run(req, env)
        .await
}

/// Validates HTTP Basic credentials for PUT requests.
///
/// This worker uses Basic Auth with the username set to `"x-auth-token"` and
/// the password set to the configured `NIX_TOKEN` value.
///
/// Returns `true` when the request contains a valid `Authorization` header and
/// the credentials match; otherwise returns `false`.
fn authorize(req: &Request, env: &Env) -> bool {
    if req.method() != Method::Put {
        return true;
    }

    let Some(header) = req.headers().get("Authorization").unwrap_or_default() else {
        return false;
    };

    let input = match Credentials::from_header(header) {
        Ok(input) => input,
        Err(_) => return false,
    };

    let expected = Credentials {
        user_id: "x-auth-token".to_string(),
        password: match env.var("NIX_TOKEN") {
            Ok(secret) => secret.to_string(),
            Err(_) => return false,
        },
    };

    input.eq(&expected)
}

fn validate_narinfo_upload(hash: &str, info: &NarInfo<'_>) -> Result<(), String> {
    // Required fields
    if !info.store_path.starts_with("/nix/store/") {
        return Err("StorePath must start with /nix/store/".to_string());
    }

    // URL must be nar/<hash>.nar (hash comes from route param, without .narinfo)
    let expected_url = format!("nar/{hash}.nar");
    if info.url != expected_url {
        return Err(format!("URL must be {expected_url}"));
    }

    validate_sha256_hash_field("NarHash", &info.nar_hash)?;

    if info.nar_size == 0 {
        return Err("NarSize must be a positive integer".to_string());
    }

    // References: allow empty, but if present each entry must look like a store path.
    for reference in info.references.iter() {
        if reference.is_empty() {
            continue;
        }
        if !reference.starts_with("/nix/store/") {
            return Err("References must be space-separated store paths".to_string());
        }
    }

    // Optional fields
    if let Some(deriver) = &info.deriver {
        if !deriver.starts_with("/nix/store/") {
            return Err("Deriver must start with /nix/store/".to_string());
        }
    }

    if let Some(compression) = &info.compression {
        let compression = compression.as_ref();
        match compression {
            "xz" | "bzip2" | "zstd" | "none" => {}
            _ => return Err("Compression must be one of xz, bzip2, zstd, none".to_string()),
        }
    }

    if let Some(file_hash) = info.file_hash {
        validate_sha256_hash_field("FileHash", file_hash)?;
    }

    if let Some(file_size) = info.file_size {
        if file_size == 0 {
            return Err("FileSize must be a positive integer".to_string());
        }
    }

    // Sig: narinfo crate already ensures `Sig` is parseable (contains ':').

    Ok(())
}

fn validate_sha256_hash_field(field: &str, value: &str) -> Result<(), String> {
    // Accept both common narinfo formats:
    // - "sha256:<base32>" (cache.nixos.org)
    // - "sha256-<base64>" (often emitted by tooling)
    let (prefix, hash, hint) = if let Some((prefix, hash)) = value.split_once(':') {
        (prefix, hash, "sha256:<base32> or sha256:<base64>")
    } else if let Some((prefix, hash)) = value.split_once('-') {
        (prefix, hash, "sha256-<base64>")
    } else {
        return Err(format!(
            "{field} must be sha256:<base32>, sha256:<base64>, or sha256-<base64>"
        ));
    };

    if prefix != "sha256" {
        return Err(format!("{field} must start with sha256"));
    }

    if hash.is_empty() {
        return Err(format!("{field} must include a hash value"));
    }

    // The ecosystem has both base32 and base64 representations. For ':' format we accept both.
    // For '-' format we only accept base64.
    let allow_base32 = value.contains(':');

    if allow_base32 && hash.bytes().all(|c| matches!(c, b'a'..=b'z' | b'2'..=b'7')) {
        return Ok(());
    }

    STANDARD
        .decode(hash)
        .map_err(|_| format!("{field} must be {hint}"))?;

    Ok(())
}

/// GET /nix-cache-info
///
/// Returns the cache configuration in the format expected by the Nix client.
fn get_nix_cache_info(_req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    let info = NixCacheInfo {
        store_dir: Cow::from("/nix/store"),
        wants_mass_query: true,
        priority: 40,
    };

    let mut data = String::new();
    info.serialize_into(&mut data).unwrap();
    Response::ok(data)
}

/// POST /
///
/// Mass query endpoint. Accepts a newline-separated list of store
/// path hashes in the request body and returns the subset that are
/// present in the cache. This allows Nix to batch-check availability
/// instead of issuing individual GET requests per package.
async fn post_mass_query(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body = req.text().await?;
    let mut data = String::new();
    let bucket = ctx.env.bucket("NIX_BUCKET")?;

    for hash in body.lines() {
        let key = if hash.ends_with(".narinfo") {
            hash.to_string()
        } else {
            format!("{hash}.narinfo")
        };

        if bucket.head(key).await?.is_some() {
            writeln!(data, "{hash}").unwrap();
        }
    }

    Response::ok(data)
}

/// GET /:hash.narinfo
///
/// Retrieves a cached `.narinfo` metadata file for the store path
/// identified by `:hash`. The narinfo contains references, nar hash,
/// file size, and other metadata required by Nix to perform
/// substitution of the corresponding store path.
async fn get_nar_info(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let Some(hash) = ctx.param("hash") else {
        return Response::error("missing hash", 400);
    };

    let key = if hash.ends_with(".narinfo") {
        hash.to_string()
    } else {
        format!("{hash}.narinfo")
    };

    let bucket = ctx.env.bucket("NIX_BUCKET")?;
    let Some(object) = bucket.get(key).execute().await? else {
        return Response::error("object not found", 404);
    };

    let Some(body) = object.body() else {
        return Response::error("object has no body", 500);
    };
    let body = body.text().await?;
    let info = match NarInfo::parse(&body) {
        Ok(info) => info,
        Err(err) => {
            console_error!("narinfo parse failed: {err:?}");
            return Response::error("object has an invalid body", 500);
        }
    };

    let mut data = String::new();
    info.serialize_into(&mut data).unwrap();
    // The library does not emit a newline which causes the nix client to fail
    writeln!(data).unwrap();

    let mut response = Response::ok(data)?;
    response
        .headers_mut()
        .set("content-type", "text/x-nix-narinfo")?;
    Ok(response)
}

/// PUT /:hash.narinfo
///
/// Uploads a `.narinfo` metadata file for the store path identified
/// by `:hash` into the cache. The request body should contain the
/// narinfo contents. This allows for populating the cache with build
/// results from external sources.
async fn put_nar_info(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let Some(hash) = ctx.param("hash") else {
        return Response::error("missing hash", 400);
    };

    let key = if hash.ends_with(".narinfo") {
        hash.to_string()
    } else {
        format!("{hash}.narinfo")
    };

    let body = req.text().await?;
    let info = match NarInfo::parse(&body) {
        Ok(info) => info,
        Err(err) => {
            console_error!("narinfo parse failed: {err:?}");
            return Response::error("invalid body", 400);
        }
    };

    let hash = hash.strip_suffix(".narinfo").unwrap_or(hash);
    if let Err(msg) = validate_narinfo_upload(hash, &info) {
        return Response::error(msg, 400);
    }

    let mut data = String::new();
    info.serialize_into(&mut data).unwrap();

    let bucket = ctx.env.bucket("NIX_BUCKET")?;
    bucket.put(key, data).execute().await?;

    Response::empty()
}

/// GET /nar/:hash.nar
///
/// Serves the actual NAR archive (the binary payload) for the store
/// path identified by `:hash`. The NAR is the compressed archive of
/// the store path contents, fetched by Nix after reading the
/// corresponding `.narinfo` metadata.
async fn get_nar(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let Some(hash) = ctx.param("hash") else {
        return Response::error("missing hash", 400);
    };

    let key = if hash.ends_with(".nar") {
        hash.to_string()
    } else {
        format!("{hash}.nar")
    };

    let bucket = ctx.env.bucket("NIX_BUCKET")?;
    let Some(object) = bucket.get(key).execute().await? else {
        return Response::error("object not found", 404);
    };

    let Some(body) = object.body() else {
        return Response::error("object has no body", 500);
    };

    let mut response = Response::from_body(body.response_body()?)?;
    response
        .headers_mut()
        .set("content-type", "application/x-nix-archive")?;
    Ok(response)
}

/// PUT /nar/:hash.nar
///
/// Uploads a NAR archive for the store path identified by `:hash`.
/// The request body should contain the compressed NAR binary.
/// Uploaded alongside the corresponding `.narinfo` to fully populate
/// a store path in the cache.
async fn put_nar(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let Some(hash) = ctx.param("hash") else {
        return Response::error("missing hash", 400);
    };

    let key = if hash.ends_with(".nar") {
        hash.to_string()
    } else {
        format!("{hash}.nar")
    };

    let body = match req.inner().body() {
        Some(stream) => stream,
        None => return Response::error("missing body", 400),
    };
    let bucket = ctx.env.bucket("NIX_BUCKET")?;
    bucket.put(key, body).execute().await?;

    Response::empty()
}
