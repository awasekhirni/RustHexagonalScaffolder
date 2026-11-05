# Generation Report

- **Project:** `generated-api`
- **Schema:** `public`
- **Generator:** `hex-gen v1.0.0`
- **Generated at:** 2026-11-05 18:08:24 UTC
- **Files written:** 123
- **Bytes written:** 259293 (253.22 KiB)
- **Tables:** 3
- **Native enums:** 2
- **Secondary indexes:** 3

## Tables

| table | rust struct | pk | features |
|---|---|---|---|
| `orders` | `Orders` | `id:Uuid` | status, soft-delete, user-ref, tenant, optimistic-concurrency, timestamps |
| `product_categories` | `ProductCategories` | `id:i32` | status, timestamps |
| `users` | `Users` | `id:Uuid` | status, soft-delete, optimistic-concurrency, email, timestamps |

## Secondary indexes

| name | table | columns | unique |
|---|---|---|---|
| `users_email_key` | `users` | `email` | yes |
| `orders_user_id_idx` | `orders` | `user_id` | no |
| `orders_tenant_status_idx` | `orders` | `tenant_id`, `order_status` | no |
