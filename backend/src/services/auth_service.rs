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
    pub sub: String,       // user_id
    pub username: String,
    pub role: String,
    pub exp: usize,        // 过期时间
    pub iat: usize,        // 签发时间
}

/// 认证服务
pub struct AuthService;

impl AuthService {
    /// 用户注册
    pub async fn register(
        pool: &SqlitePool,
        req: &RegisterRequest,
    ) -> anyhow::Result<UserInfo> {
        // 验证用户名
        if req.username.trim().is_empty() {
            anyhow::bail!("用户名不能为空");
        }
        if req.password.len() < 6 {
            anyhow::bail!("密码至少需要6个字符");
        }

        // 检查用户名是否已存在
        let existing = sqlx::query_scalar::<_, i32>(
            "SELECT COUNT(*) FROM users WHERE username = ?",
        )
        .bind(&req.username)
        .fetch_one(pool)
        .await?;

        if existing > 0 {
            anyhow::bail!("用户名已存在");
        }

        // 密码哈希
        let password_hash = hash(&req.password, DEFAULT_COST)?;

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        // 首个注册用户自动设为管理员
        let total_count: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(pool)
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
        .execute(pool)
        .await?;

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
}
