//! Application layer — use-case adapters driving the domain through ports.
//!
//! Services depend only on port traits; the enterprise-bus handlers below
//! delegate to them, and HTTP handlers dispatch through the buses.

pub mod orders_mapper;
pub mod orders_messaging;
pub mod orders_service;
pub mod product_categories_mapper;
pub mod product_categories_messaging;
pub mod product_categories_service;
pub mod users_mapper;
pub mod users_messaging;
pub mod users_service;
