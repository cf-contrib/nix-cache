use std::{borrow::Cow, fmt::Write};

use narinfo::*;
use worker::*;

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    Router::new()
        .get("/nix-cache-info", get_nix_cache_info)
        .post_async("/", post_mass_query)
        .get_async("/:hash", get_narinfo)
        .put_async("/:hash", put_narinfo)
        .get_async("/nar/:hash", get_nar)
        .put_async("/nar/:hash", put_nar)
        .run(req, env)
        .await
}

/// GET /nix-cache-info
///
/// Returns information about the Nix binary cache, such as the store
/// directory, desired number of parallel connections, binary cache
/// version, and priority. This endpoint mirrors the `nix-cache-info`
/// file typically served by Nix binary caches.
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
async fn get_narinfo(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
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
async fn put_narinfo(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
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
