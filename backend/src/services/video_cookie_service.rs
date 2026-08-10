use std::collections::BTreeSet;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::Path;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::Utc;
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

const KEY_FILE: &str = "video-cookie.key";

#[derive(Debug, Clone, Serialize)]
pub struct VideoCookieStatus {
    pub platform: String,
    pub cookie_count: i64,
    pub domains: Vec<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoCookieBackup {
    pub version: u32,
    pub platform: String,
    pub encrypted_payload: String,
    pub cookie_count: i64,
    pub domains: Vec<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, FromRow)]
struct VideoCookieRow {
    platform: String,
    encrypted_payload: String,
    cookie_count: i64,
    domain_summary: String,
    updated_at: String,
}

pub struct VideoCookieService;

impl VideoCookieService {
    pub fn supported_platforms() -> Vec<&'static str> {
        vec!["youtube", "bilibili", "sohu", "youku", "mgtv", "pptv", "qq"]
    }

    pub fn normalize_platform(platform: &str) -> Option<&'static str> {
        match platform.trim().to_ascii_lowercase().as_str() {
            "youtube" | "youtu.be" => Some("youtube"),
            "bilibili" | "b23" => Some("bilibili"),
            "sohu" | "tv.sohu" => Some("sohu"),
            "youku" | "tudou" => Some("youku"),
            "mgtv" | "mango" | "hunantv" => Some("mgtv"),
            "pptv" | "pps" => Some("pptv"),
            "qq" | "tencent" | "vqq" => Some("qq"),
            _ => None,
        }
    }

    pub fn platform_for_url(url: &str) -> Option<&'static str> {
        let lower = url.to_ascii_lowercase();
        if lower.contains("youtube.com") || lower.contains("youtu.be") {
            Some("youtube")
        } else if lower.contains("bilibili.com") || lower.contains("b23.tv") {
            Some("bilibili")
        } else if lower.contains("sohu.com") {
            Some("sohu")
        } else if lower.contains("youku.com") || lower.contains("tudou.com") {
            Some("youku")
        } else if lower.contains("mgtv.com") || lower.contains("hunantv.com") {
            Some("mgtv")
        } else if lower.contains("pptv.com") || lower.contains("pps.tv") {
            Some("pptv")
        } else if lower.contains("v.qq.com") || lower.contains("video.qq.com") {
            Some("qq")
        } else {
            None
        }
    }

    pub async fn list(db: &SqlitePool) -> anyhow::Result<Vec<VideoCookieStatus>> {
        let rows = sqlx::query_as::<_, VideoCookieRow>(
            "SELECT platform, encrypted_payload, cookie_count, domain_summary, updated_at FROM video_cookie_store ORDER BY platform ASC",
        )
        .fetch_all(db)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| VideoCookieStatus {
                platform: row.platform,
                cookie_count: row.cookie_count,
                domains: parse_domain_summary(&row.domain_summary),
                updated_at: row.updated_at,
            })
            .collect())
    }

    pub async fn import_json(
        db: &SqlitePool,
        secure_dir: &str,
        platform: &str,
        cookies: serde_json::Value,
        user_id: &str,
    ) -> anyhow::Result<VideoCookieStatus> {
        let platform = Self::normalize_platform(platform)
            .ok_or_else(|| anyhow::anyhow!("不支持的平台: {platform}"))?;
        let (cookie_count, domains) = validate_cookie_payload(platform, &cookies)?;
        let payload = serde_json::to_string(&cookies)?;
        let key = load_or_create_key(secure_dir)?;
        let encrypted_payload = encrypt_payload(&key, payload.as_bytes())?;
        upsert_record(db, platform, &encrypted_payload, cookie_count, &domains, user_id).await?;
        Ok(VideoCookieStatus {
            platform: platform.to_string(),
            cookie_count,
            domains,
            updated_at: Utc::now().to_rfc3339(),
        })
    }

    pub async fn export_backup(
        db: &SqlitePool,
        platform: &str,
    ) -> anyhow::Result<VideoCookieBackup> {
        let platform = Self::normalize_platform(platform)
            .ok_or_else(|| anyhow::anyhow!("不支持的平台: {platform}"))?;
        let row = get_row(db, platform)
            .await?
            .ok_or_else(|| anyhow::anyhow!("平台尚未导入 Cookie: {platform}"))?;
        Ok(VideoCookieBackup {
            version: 1,
            platform: row.platform,
            encrypted_payload: row.encrypted_payload,
            cookie_count: row.cookie_count,
            domains: parse_domain_summary(&row.domain_summary),
            updated_at: row.updated_at,
        })
    }

    pub async fn import_backup(
        db: &SqlitePool,
        secure_dir: &str,
        backup: VideoCookieBackup,
        user_id: &str,
    ) -> anyhow::Result<VideoCookieStatus> {
        if backup.version != 1 {
            anyhow::bail!("不支持的 Cookie 备份版本: {}", backup.version);
        }
        let platform = Self::normalize_platform(&backup.platform)
            .ok_or_else(|| anyhow::anyhow!("不支持的平台: {}", backup.platform))?;
        let key = load_or_create_key(secure_dir)?;
        let decrypted = decrypt_payload(&key, &backup.encrypted_payload)?;
        let cookies: serde_json::Value = serde_json::from_slice(&decrypted)?;
        let (cookie_count, domains) = validate_cookie_payload(platform, &cookies)?;
        upsert_record(
            db,
            platform,
            &backup.encrypted_payload,
            cookie_count,
            &domains,
            user_id,
        )
        .await?;
        Ok(VideoCookieStatus {
            platform: platform.to_string(),
            cookie_count,
            domains,
            updated_at: Utc::now().to_rfc3339(),
        })
    }

    pub async fn delete(db: &SqlitePool, platform: &str) -> anyhow::Result<bool> {
        let platform = Self::normalize_platform(platform)
            .ok_or_else(|| anyhow::anyhow!("不支持的平台: {platform}"))?;
        let result = sqlx::query("DELETE FROM video_cookie_store WHERE platform = ?")
            .bind(platform)
            .execute(db)
            .await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn load_cookie_json(
        db: &SqlitePool,
        secure_dir: &str,
        platform: &str,
    ) -> anyhow::Result<Option<serde_json::Value>> {
        let Some(platform) = Self::normalize_platform(platform) else {
            return Ok(None);
        };
        let Some(row) = get_row(db, platform).await? else {
            return Ok(None);
        };
        let key = load_or_create_key(secure_dir)?;
        let decrypted = decrypt_payload(&key, &row.encrypted_payload)?;
        let cookies = serde_json::from_slice(&decrypted)?;
        Ok(Some(cookies))
    }
}

async fn get_row(db: &SqlitePool, platform: &str) -> anyhow::Result<Option<VideoCookieRow>> {
    let row = sqlx::query_as::<_, VideoCookieRow>(
        "SELECT platform, encrypted_payload, cookie_count, domain_summary, updated_at FROM video_cookie_store WHERE platform = ?",
    )
    .bind(platform)
    .fetch_optional(db)
    .await?;
    Ok(row)
}

async fn upsert_record(
    db: &SqlitePool,
    platform: &str,
    encrypted_payload: &str,
    cookie_count: i64,
    domains: &[String],
    user_id: &str,
) -> anyhow::Result<()> {
    let now = Utc::now().to_rfc3339();
    let domain_summary = serde_json::to_string(domains)?;
    sqlx::query(
        r#"INSERT INTO video_cookie_store (platform, encrypted_payload, cookie_count, domain_summary, updated_by, updated_at)
           VALUES (?, ?, ?, ?, ?, ?)
           ON CONFLICT(platform) DO UPDATE SET
             encrypted_payload = excluded.encrypted_payload,
             cookie_count = excluded.cookie_count,
             domain_summary = excluded.domain_summary,
             updated_by = excluded.updated_by,
             updated_at = excluded.updated_at"#,
    )
    .bind(platform)
    .bind(encrypted_payload)
    .bind(cookie_count)
    .bind(domain_summary)
    .bind(user_id)
    .bind(now)
    .execute(db)
    .await?;
    Ok(())
}

fn validate_cookie_payload(
    platform: &str,
    cookies: &serde_json::Value,
) -> anyhow::Result<(i64, Vec<String>)> {
    let arr = cookies
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Cookie JSON 必须是数组"))?;
    if arr.is_empty() {
        anyhow::bail!("Cookie JSON 不能为空");
    }

    let allowed = allowed_domains(platform);
    let mut domains = BTreeSet::new();
    for cookie in arr {
        let obj = cookie
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("Cookie 数组中包含非对象项"))?;
        let name = obj.get("name").and_then(|v| v.as_str()).unwrap_or_default();
        let domain = obj
            .get("domain")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if !obj.get("value").is_some_and(|v| v.is_string()) || name.is_empty() || domain.is_empty() {
            anyhow::bail!("Cookie 必须包含 name/domain 字段，value 必须是字符串");
        }
        let normalized = normalize_domain(domain);
        if !is_allowed_domain(&normalized, allowed) {
            anyhow::bail!("Cookie 域名不属于平台 {platform}: {domain}");
        }
        domains.insert(normalized);
    }

    Ok((arr.len() as i64, domains.into_iter().collect()))
}

fn parse_domain_summary(raw: &str) -> Vec<String> {
    serde_json::from_str(raw).unwrap_or_default()
}

fn normalize_domain(domain: &str) -> String {
    domain
        .trim()
        .trim_start_matches('.')
        .trim_end_matches('.')
        .to_ascii_lowercase()
}

fn is_allowed_domain(domain: &str, allowed: &[&str]) -> bool {
    allowed
        .iter()
        .any(|suffix| domain == *suffix || domain.ends_with(&format!(".{suffix}")))
}

fn allowed_domains(platform: &str) -> &'static [&'static str] {
    match platform {
        "youtube" => &["youtube.com", "google.com", "googleusercontent.com"],
        "bilibili" => &["bilibili.com", "b23.tv"],
        "sohu" => &["sohu.com"],
        "youku" => &["youku.com", "tudou.com"],
        "mgtv" => &["mgtv.com", "hunantv.com"],
        "pptv" => &["pptv.com", "pps.tv"],
        "qq" => &["qq.com"],
        _ => &[],
    }
}

fn load_or_create_key(secure_dir: &str) -> anyhow::Result<Vec<u8>> {
    std::fs::create_dir_all(secure_dir)?;
    let path = Path::new(secure_dir).join(KEY_FILE);
    if path.exists() {
        let mut raw = String::new();
        OpenOptions::new()
            .read(true)
            .open(&path)?
            .read_to_string(&mut raw)?;
        let key = STANDARD.decode(raw.trim())?;
        if key.len() != 32 {
            anyhow::bail!("视频 Cookie 密钥长度无效: {}", path.display());
        }
        return Ok(key);
    }

    let mut key = vec![0u8; 32];
    SystemRandom::new()
        .fill(&mut key)
        .map_err(|_| anyhow::anyhow!("生成视频 Cookie 密钥失败"))?;
    let encoded = STANDARD.encode(&key);
    let mut opts = OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut file = opts.open(&path)?;
    file.write_all(encoded.as_bytes())?;
    file.write_all(b"\n")?;
    Ok(key)
}

fn encrypt_payload(key: &[u8], payload: &[u8]) -> anyhow::Result<String> {
    let cipher = LessSafeKey::new(
        UnboundKey::new(&AES_256_GCM, key).map_err(|_| anyhow::anyhow!("Cookie 密钥无效"))?,
    );
    let mut nonce_bytes = [0u8; 12];
    SystemRandom::new()
        .fill(&mut nonce_bytes)
        .map_err(|_| anyhow::anyhow!("生成视频 Cookie nonce 失败"))?;
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);
    let mut ciphertext = payload.to_vec();
    cipher
        .seal_in_place_append_tag(nonce, Aad::empty(), &mut ciphertext)
        .map_err(|_| anyhow::anyhow!("Cookie 加密失败"))?;
    Ok(serde_json::json!({
        "alg": "A256GCM",
        "nonce": STANDARD.encode(nonce_bytes),
        "ciphertext": STANDARD.encode(ciphertext),
    })
    .to_string())
}

fn decrypt_payload(key: &[u8], encrypted_payload: &str) -> anyhow::Result<Vec<u8>> {
    let value: serde_json::Value = serde_json::from_str(encrypted_payload)?;
    if value.get("alg").and_then(|v| v.as_str()) != Some("A256GCM") {
        anyhow::bail!("不支持的 Cookie 加密算法");
    }
    let nonce = value
        .get("nonce")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Cookie 加密包缺少 nonce"))?;
    let ciphertext = value
        .get("ciphertext")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Cookie 加密包缺少 ciphertext"))?;
    let nonce = STANDARD.decode(nonce)?;
    let ciphertext = STANDARD.decode(ciphertext)?;
    let cipher = LessSafeKey::new(
        UnboundKey::new(&AES_256_GCM, key).map_err(|_| anyhow::anyhow!("Cookie 密钥无效"))?,
    );
    let mut plaintext = ciphertext;
    cipher
        .open_in_place(
            Nonce::try_assume_unique_for_key(nonce.as_slice())
                .map_err(|_| anyhow::anyhow!("Cookie nonce 无效"))?,
            Aad::empty(),
            &mut plaintext,
        )
        .map(|value| value.to_vec())
        .map_err(|_| anyhow::anyhow!("Cookie 解密失败，请确认 /app/storage 密钥未变化"))
}
