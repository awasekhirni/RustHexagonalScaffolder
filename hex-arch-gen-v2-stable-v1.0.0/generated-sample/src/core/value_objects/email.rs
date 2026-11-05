//! `Email` — validated email address value object.

use std::fmt;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::shared::error::AppError;

/// A validated email address.
///
/// Construct via [`Email::parse`]; the constructor rejects malformed
/// inputs with [`AppError::Validation`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
pub struct Email(String);

impl Email {
    /// Parses and validates an email address.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Validation`] when the input is empty, lacks
    /// an `@`, or exceeds 254 characters (RFC 5321).
    pub fn parse(raw: impl Into<String>) -> Result<Self, AppError> {
        let s = raw.into();
        if s.is_empty() {
            return Err(AppError::Validation("email must not be empty".into()));
        }
        if s.len() > 254 {
            return Err(AppError::Validation("email exceeds 254 chars (RFC 5321)".into()));
        }
        let at_count = s.matches('@').count();
        if at_count != 1 {
            return Err(AppError::Validation(format!(
                "email must contain exactly one '@' (found {at_count})"
            )));
        }
        let (local, domain) = s.split_once('@').unwrap();
        if local.is_empty() || domain.is_empty() {
            return Err(AppError::Validation("email local and domain parts must not be empty".into()));
        }
        if !domain.contains('.') {
            return Err(AppError::Validation("email domain must contain a dot".into()));
        }
        Ok(Self(s))
    }

    /// Returns the inner string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Email {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::str::FromStr for Email {
    type Err = AppError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}
