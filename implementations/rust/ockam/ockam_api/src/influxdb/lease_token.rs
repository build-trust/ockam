use crate::colors::{color_error, color_ok, color_primary};
use crate::output::Output;
use crate::terminal::fmt;
use crate::ApiError;
use minicbor::{CborLen, Decode, Encode};
use ockam::identity::Identifier;
use ockam_core::compat::fmt::Error as FmtError;
use serde::{Deserialize, Serialize};
use std::cmp::PartialEq;
use std::fmt::{Display, Formatter};
use strum::{Display, EnumString};
use time::OffsetDateTime;

#[derive(Encode, Decode, CborLen, Serialize, Deserialize, Debug, Clone)]
#[cbor(map)]
pub struct LeaseToken {
    #[cbor(n(1))]
    pub id: String,

    #[cbor(n(2))]
    pub issued_for: Identifier,

    #[cbor(n(3))]
    pub created_at: i64,

    #[cbor(n(4))]
    pub expires_at: i64,

    #[cbor(n(5))]
    pub token: String,

    #[cbor(n(6))]
    pub status: TokenStatus,
}

#[cfg(test)]
impl Default for LeaseToken {
    fn default() -> Self {
        use std::str::FromStr;
        Self {
            id: "token_id".to_string(),
            issued_for: Identifier::from_str(
                "I0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            )
            .unwrap(),
            created_at: OffsetDateTime::now_utc().unix_timestamp(),
            expires_at: OffsetDateTime::now_utc().unix_timestamp(),
            token: "token".to_string(),
            status: TokenStatus::Active,
        }
    }
}

impl LeaseToken {
    pub fn is_active(&self) -> bool {
        self.status == TokenStatus::Active
    }

    pub fn created_at(&self) -> ockam_core::Result<OffsetDateTime> {
        OffsetDateTime::from_unix_timestamp(self.created_at)
            .map_err(|e| ApiError::core(e.to_string()))
    }

    pub fn expires_at(&self) -> ockam_core::Result<OffsetDateTime> {
        OffsetDateTime::from_unix_timestamp(self.expires_at)
            .map_err(|e| ApiError::core(e.to_string()))
    }

    pub fn is_expired(&self) -> ockam_core::Result<bool> {
        Ok(self.expires_at()? < OffsetDateTime::now_utc())
    }
}

impl Display for LeaseToken {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", color_primary(&self.id))?;

        writeln!(
            f,
            "{}With value {}",
            fmt::INDENTATION,
            color_primary(&self.token)
        )?;

        writeln!(
            f,
            "{}Issued for {}",
            fmt::INDENTATION,
            color_primary(&self.issued_for)
        )?;

        let created_at = self.created_at().map_err(|_| FmtError)?.to_string();
        writeln!(
            f,
            "{}Created at {}",
            fmt::INDENTATION,
            color_primary(created_at)
        )?;

        let status = if self.is_active() {
            color_ok(&self.status)
        } else {
            color_error(&self.status)
        };
        let expires_at = self.expires_at().map_err(|_| FmtError)?.to_string();
        let expiration_time = if self.is_expired().map_err(|_| FmtError)? {
            format!("Expired at {}", color_error(&expires_at))
        } else {
            format!("Expires at {}", color_primary(&expires_at))
        };
        writeln!(f, "{}{expiration_time} ({status})", fmt::INDENTATION)?;

        Ok(())
    }
}

impl Output for LeaseToken {
    fn item(&self) -> crate::Result<String> {
        Ok(self.padded_display())
    }
}

impl Ord for LeaseToken {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.expires_at.cmp(&other.expires_at)
    }
}

impl PartialOrd for LeaseToken {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for LeaseToken {
    fn eq(&self, other: &Self) -> bool {
        self.expires_at == other.expires_at
    }
}

impl Eq for LeaseToken {}

#[derive(
    Encode, Decode, CborLen, Serialize, Deserialize, PartialEq, Debug, Clone, EnumString, Display,
)]
pub enum TokenStatus {
    #[n(0)]
    #[strum(serialize = "active")]
    Active,

    #[n(1)]
    #[strum(serialize = "inactive")]
    Revoked,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lease_token_display() {
        let token = LeaseToken {
            created_at: OffsetDateTime::now_utc().unix_timestamp(),
            expires_at: OffsetDateTime::now_utc().unix_timestamp(),
            ..Default::default()
        };
        assert!(token.expires_at().is_ok());
        assert!(token.is_expired().is_ok());
        assert!(token.item().is_ok());
    }

    #[test]
    fn token_lease_is_expired() {
        let mut token = LeaseToken {
            expires_at: OffsetDateTime::now_utc().unix_timestamp() - 100,
            ..Default::default()
        };
        assert!(token.is_expired().unwrap());

        token.expires_at = OffsetDateTime::now_utc().unix_timestamp() + 100;
        assert!(!token.is_expired().unwrap());
    }

    #[test]
    fn token_lease_ordering() {
        let token1 = LeaseToken {
            expires_at: OffsetDateTime::now_utc().unix_timestamp() + 100,
            ..Default::default()
        };
        let token2 = LeaseToken {
            expires_at: OffsetDateTime::now_utc().unix_timestamp() + 200,
            ..Default::default()
        };
        // token1 expires before token2
        assert!(token1 < token2);
    }
}
