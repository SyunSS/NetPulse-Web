use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::models::user::{LoginResponse, RegisterRequest, User, UserInfo};

/// JWT Claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user_id
    pub username: String,
    pub role: String,
    pub exp: usize, // 过期时间
    pub iat: usize, // 签发时间
}

/// 认证服务
pub struct AuthService;

impl AuthService {
    /// 用户注册
    pub async fn register(pool: &SqlitePool, req: &RegisterRequest) -> anyhow::Result<UserInfo> {
        // 验证用户名
        if req.username.trim().is_empty() {
            anyhow::bail!("用户名不能为空");
        }
        if req.password.len() < 8 {
            anyhow::bail!("密码至少需要8个字符");
        }
        if !req.password.chars().any(|c| c.is_uppercase()) {
            anyhow::bail!("密码需要包含至少一个大写字母");
        }
        if !req.password.chars().any(|c| c.is_lowercase()) {
            anyhow::bail!("密码需要包含至少一个小写字母");
        }
        if !req.password.chars().any(|c| c.is_ascii_digit()) {
            anyhow::bail!("密码需要包含至少一个数字");
        }

        // 密码哈希
        let password_hash = hash(&req.password, DEFAULT_COST)?;

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let mut tx = pool.begin().await?;

        let existing =
            sqlx::query_scalar::<_, i32>("SELECT COUNT(*) FROM users WHERE username = ?")
                .bind(&req.username)
                .fetch_one(&mut *tx)
                .await?;
        if existing > 0 {
            anyhow::bail!("用户名已存在");
        }

        // Run the first-admin check and insert in one write transaction so two
        // concurrent first registrations cannot both observe an empty table.
        let total_count: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&mut *tx)
            .await?;
        let role = if total_count == 0 { "admin" } else { "user" };

        sqlx::query(
            "INSERT INTO users (id, username, password_hash, role, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&req.username)
        .bind(&password_hash)
        .bind(role)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(UserInfo {
            id,
            username: req.username.clone(),
            role: role.to_string(),
        })
    }

    /// 用户登录
    pub async fn login(
        pool: &SqlitePool,
        config: &AppConfig,
        username: &str,
        password: &str,
    ) -> anyhow::Result<LoginResponse> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = ?")
            .bind(username)
            .fetch_optional(pool)
            .await?;

        let user = user.ok_or_else(|| anyhow::anyhow!("用户名或密码错误"))?;

        if !verify(password, &user.password_hash)? {
            anyhow::bail!("用户名或密码错误");
        }

        let now = Utc::now();
        let exp = now + chrono::Duration::hours(config.jwt.expiration_hours);

        let claims = Claims {
            sub: user.id.clone(),
            username: user.username.clone(),
            role: user.role.clone(),
            exp: exp.timestamp() as usize,
            iat: now.timestamp() as usize,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(config.jwt.secret.as_bytes()),
        )?;

        Ok(LoginResponse {
            token,
            user: user.into(),
        })
    }

    /// 验证 JWT Token
    pub fn verify_token(config: &AppConfig, token: &str) -> anyhow::Result<Claims> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(config.jwt.secret.as_bytes()),
            &Validation::default(),
        )?;
        Ok(token_data.claims)
    }

    /// Validate both the token and the current database identity. This makes
    /// role changes and deleted users take effect before the request reaches a handler.
    pub async fn verify_current_user(
        pool: &SqlitePool,
        config: &AppConfig,
        token: &str,
    ) -> anyhow::Result<Claims> {
        let mut claims = Self::verify_token(config, token)?;
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
            .bind(&claims.sub)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| anyhow::anyhow!("用户不存在"))?;
        claims.username = user.username;
        claims.role = user.role;
        Ok(claims)
    }
}
