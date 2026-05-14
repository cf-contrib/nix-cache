use worker::*;

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let mut router = Router::new();

    // GET /nix-cache-info
    //
    // Returns information about the Nix binary cache, such as the store
    // directory, desired number of parallel connections, binary cache
    // version, and priority. This endpoint mirrors the `nix-cache-info`
    // file typically served by Nix binary caches.
    router = router.get("/nix-cache-info", |_, _| {
        // TODO: implement the endpoint
        Response::ok("")
    });

    // POST /
    //
    // Mass query endpoint. Accepts a newline-separated list of store
    // path hashes in the request body and returns the subset that are
    // present in the cache. This allows Nix to batch-check availability
    // instead of issuing individual GET requests per package.
    router = router.post_async("/", |_, _| async move {
        // TODO: implement the endpoint
        Response::ok("")
    });

    // GET /:hash
    //
    // Retrieves a cached `.narinfo` metadata file for the store path
    // identified by `:hash`. The narinfo contains references, nar hash,
    // file size, and other metadata required by Nix to perform
    // substitution of the corresponding store path.
    router = router.get_async("/:hash", |_, _| async move {
        // TODO: implement the endpoint
        Response::ok("")
    });

    // PUT /:hash
    //
    // Uploads a `.narinfo` metadata file for the store path identified
    // by `:hash` into the cache. The request body should contain the
    // narinfo contents. This allows for populating the cache with build
    // results from external sources.
    router = router.put_async("/:hash", |_, _| async move {
        // TODO: implement the endpoint
        Response::ok("")
    });

    // GET /nar/:hash.nar
    //
    // Serves the actual NAR archive (the binary payload) for the store
    // path identified by `:hash`. The NAR is the compressed archive of
    // the store path contents, fetched by Nix after reading the
    // corresponding `.narinfo` metadata.
    router = router.get_async("/nar/:hash.nar", |_, _| async move {
        // TODO: implement the endpoint
        Response::ok("")
    });

    // PUT /nar/:hash.nar
    //
    // Uploads a NAR archive for the store path identified by `:hash`.
    // The request body should contain the compressed NAR binary.
    // Uploaded alongside the corresponding `.narinfo` to fully populate
    // a store path in the cache.
    router = router.put_async("/nar/:hash.nar", |_, _| async move {
        // TODO: implement the endpoint
        Response::ok("")
    });

    router.run(req, env).await
}
