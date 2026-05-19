use base64::{engine::general_purpose::STANDARD, Engine};
use narinfo::NarInfo;

pub trait Validate {
    type Context;

    fn validate(&self, ctx: &Self::Context) -> Result<(), String>;
}

/// Context for validating a narinfo upload.
///
/// `hash` is taken from the request route param (without `.narinfo`).
pub struct NarInfoContext {
    pub hash: String,
}

impl Validate for NarInfo<'_> {
    type Context = NarInfoContext;

    fn validate(&self, ctx: &Self::Context) -> Result<(), String> {
        // Required fields
        if !self.store_path.starts_with("/nix/store/") {
            return Err("StorePath must start with /nix/store/".to_string());
        }

        // URL must be nar/<hash>.nar
        let expected_url = format!("nar/{}.nar", ctx.hash);
        if self.url != expected_url {
            return Err(format!("URL must be {expected_url}"));
        }

        validate_sha256_hash_field("NarHash", &self.nar_hash)?;

        if self.nar_size == 0 {
            return Err("NarSize must be a positive integer".to_string());
        }

        // References: allow empty, but if present each entry must look like a store path.
        for reference in self.references.iter() {
            if reference.is_empty() {
                continue;
            }
            if !reference.starts_with("/nix/store/") {
                return Err("References must be space-separated store paths".to_string());
            }
        }

        // Optional fields
        if let Some(deriver) = &self.deriver {
            if !deriver.starts_with("/nix/store/") {
                return Err("Deriver must start with /nix/store/".to_string());
            }
        }

        if let Some(compression) = &self.compression {
            let compression = compression.as_ref();
            match compression {
                "xz" | "bzip2" | "zstd" | "none" => {}
                _ => return Err("Compression must be one of xz, bzip2, zstd, none".to_string()),
            }
        }

        if let Some(file_hash) = self.file_hash {
            validate_sha256_hash_field("FileHash", file_hash)?;
        }

        if let Some(file_size) = self.file_size {
            if file_size == 0 {
                return Err("FileSize must be a positive integer".to_string());
            }
        }

        // Sig: narinfo crate already ensures `Sig` is parseable (contains ':').

        Ok(())
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn narinfo() -> NarInfo<'static> {
        NarInfo::builder()
            .store_path("/nix/store/abc-min".into())
            .url("nar/abc.nar")
            .nar_hash("sha256-LHdODcc9LKl8TykaDMvSkpcBrXrTcP8aW2B6trJhxdE=".into())
            .nar_size(1)
            .references(vec![])
            .compression(Some("none".into()))
            .build()
            .expect("NarInfo should build")
    }

    #[test]
    fn narinfo_validate_ok_minimal() {
        let info = narinfo();
        let ctx = NarInfoContext {
            hash: "abc".to_string(),
        };
        assert!(info.validate(&ctx).is_ok());
    }

    #[test]
    fn narinfo_validate_rejects_store_path() {
        let mut info = narinfo();
        info.store_path = "/tmp/abc".into();

        let ctx = NarInfoContext {
            hash: "abc".to_string(),
        };
        let err = info.validate(&ctx).unwrap_err();
        assert!(err.contains("StorePath"));
    }

    #[test]
    fn narinfo_validate_rejects_url_mismatch() {
        let mut info = narinfo();
        info.url = "nar/zzz.nar";

        let ctx = NarInfoContext {
            hash: "abc".to_string(),
        };
        let err = info.validate(&ctx).unwrap_err();
        assert!(err.contains("URL"));
    }

    #[test]
    fn narinfo_validate_rejects_nar_size_zero() {
        let mut info = narinfo();
        info.nar_size = 0;

        let ctx = NarInfoContext {
            hash: "abc".to_string(),
        };
        let err = info.validate(&ctx).unwrap_err();
        assert!(err.contains("NarSize"));
    }

    #[test]
    fn narinfo_validate_rejects_bad_compression() {
        let mut info = narinfo();
        info.compression = Some("gzip".into());

        let ctx = NarInfoContext {
            hash: "abc".to_string(),
        };
        let err = info.validate(&ctx).unwrap_err();
        assert!(err.contains("Compression"));
    }

    #[test]
    fn sha256_hash_field_accepts_base32_colon_format() {
        let value = "sha256:0c8ld5yxcr6a6j63mvrqbqiy08q6f85wd74817ai7pvd5nkidcqw";
        assert!(validate_sha256_hash_field("NarHash", value).is_ok());
    }

    #[test]
    fn sha256_hash_field_rejects_invalid_base64_dash_format() {
        let value = "sha256-not_base64";
        let err = validate_sha256_hash_field("NarHash", value).unwrap_err();
        assert!(err.contains("sha256-<base64>"));
    }
}
