<markdown-accessiblity-table>
<table>
  <tbody>
  <tr>
    <th>name</th>
    <td>senior-tauri-backend</td>
  </tr>
  <tr>
    <th>description</th>
    <td>Designs and implements Offline-First Desktop architectures using Rust, Tauri, SeaORM, and SQLite. Use when building IPC commands, setting up local databases, handling offline sync logic, or bridging Vue.js frontend with Rust backend.</td>
  </tr>
  </tbody>
</table>
</markdown-accessiblity-table>

# Senior Tauri/Rust Backend Engineer

Expert in Desktop-first architectures, strict IPC communication, SeaORM modeling, and Modular (Nest-like) structure in Rust.

---

## 🛑 Rule 0: Analyze Frontend First (Mandatory)

Before writing **any** backend Rust code or SeaORM entities, you MUST analyze the Vue.js frontend structure.

1. Inspect the Frontend `Mock Data` and Pinia Stores.
2. Extract the exact JSON shapes (DTOs) expected by the UI.
3. Understand the frontend UI flow.
4. **Only then** build the Rust Structs, SeaORM Models, and Tauri Commands to match the frontend expectations perfectly.

---

## Backend Development Workflows

### 1. Modular Architecture Workflow

Always structure the Rust backend (`src-tauri/src/`) using a Domain-Driven Design (Nest.js style):

- `core/`: Database connection, global error handling.
- `entities/`: SeaORM models (unified to handle relations easily).
- `modules/{feature}/`: Contains `commands.rs` (Controller), `service.rs` (Logic), and `dto.rs` (Payloads).
- `infrastructure/`: External integrations (Thermal Printers, Typst PDF generation).

### 2. Tauri Command Design (IPC Bridge)

Keep Commands "Dumb". They only receive data, pass it to Services, and return standard Results to Vue.

```rust
// modules/invoice/commands.rs
use tauri::State;
use sea_orm::DatabaseConnection;
use super::dto::CreateInvoiceDto;
use super::service;

#[tauri::command]
pub async fn create_invoice_cmd(
    db: State<'_, DatabaseConnection>,
    payload: CreateInvoiceDto,
) -> Result<String, String> {
    service::create_invoice(&db, payload)
        .await
        .map_err(|e| e.to_string()) // Convert Rust Errors to String for Vue
}

```

### 3. SeaORM Database Workflow

- **Primary Keys:** Always use `UUID` (never auto-increment integers) to prevent sync collisions.
- **Sync Columns:** Every entity must have `sync_status` (0=local, 1=synced, 2=modified) and `updated_at`.
- **Async Queries:** Always use async/await with connection pools.

## Reference Patterns

### Custom Result Type (Error Handling)

```rust
// core/error.rs
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sea_orm::DbErr),
    #[error("Validation error: {0}")]
    Validation(String),
}

pub type AppResult<T> = Result<T, AppError>;

```
