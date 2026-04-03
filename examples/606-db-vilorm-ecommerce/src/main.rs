// +============================================================+
// |  606 -- VilORM E-Commerce (SQLite)                        |
// +============================================================+
// |  Pattern:  VilEntity + VilQuery                           |
// |  Features: Multi-table e-commerce domain                  |
// |  Domain:   Products, Orders, Order Items                  |
// +============================================================+
// |  Demonstrates VilORM patterns:                            |
// |  1. #[derive(VilEntity)] on 3 models                     |
// |  2. insert_columns().value().value_opt_str() -- NULL-safe |
// |  3. select().join().fetch_all() -- JOINed listings        |
// |  4. update().set_expr("stock","stock - ?",qty) -- atomic  |
// |  5. select_expr("SUM(...)").scalar::<f64>() -- aggregates |
// |  6. T::find_by_id() -- PK lookup                         |
// |  7. select().where_eq().order_by_desc().limit() -- catalog|
// |  8. T::delete() -- cancel order                           |
// +============================================================+
//
// Run:   cargo run -p vil-db-vilorm-ecommerce
// Test:
//   # Create product
//   curl -X POST http://localhost:8086/api/shop/products \
//     -H 'Content-Type: application/json' \
//     -d '{"name":"Rust Book","price":29.99,"stock":100,"category":"books"}'
//
//   # List products
//   curl http://localhost:8086/api/shop/products
//
//   # Create order
//   curl -X POST http://localhost:8086/api/shop/orders \
//     -H 'Content-Type: application/json' \
//     -d '{"customer_name":"Alice","items":[{"product_id":"<id>","quantity":2}]}'
//
//   # Order total
//   curl http://localhost:8086/api/shop/orders/<id>/total
//
//   # Cancel order
//   curl -X DELETE http://localhost:8086/api/shop/orders/<id>

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use vil_db_sqlx::SqlxPool;
use vil_orm_derive::VilEntity;
use vil_server::prelude::*;

// -- Models --

#[derive(Debug, Clone, Serialize, Deserialize, VilModel, sqlx::FromRow, VilEntity)]
#[vil_entity(table = "products")]
struct Product {
    #[vil_entity(pk)]
    id: String,
    name: String,
    description: Option<String>,
    price: f64,
    stock: i64,
    category: Option<String>,
    #[vil_entity(auto_now_add)]
    created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, VilModel, sqlx::FromRow, VilEntity)]
#[vil_entity(table = "orders")]
struct Order {
    #[vil_entity(pk)]
    id: String,
    customer_name: String,
    status: String,
    #[vil_entity(auto_now_add)]
    created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, VilModel, sqlx::FromRow, VilEntity)]
#[vil_entity(table = "order_items")]
struct OrderItem {
    #[vil_entity(pk)]
    id: String,
    order_id: String,
    product_id: String,
    quantity: i64,
    unit_price: f64,
}

// -- View types for JOINed queries --

#[derive(Debug, Clone, Serialize, Deserialize, VilModel, sqlx::FromRow)]
struct OrderListItem {
    id: String,
    customer_name: String,
    status: String,
    created_at: String,
    item_count: i64,
}

// -- Request types --

#[derive(Debug, Deserialize)]
struct CreateProduct {
    name: String,
    description: Option<String>,
    price: f64,
    stock: Option<i64>,
    category: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OrderItemReq {
    product_id: String,
    quantity: i64,
}

#[derive(Debug, Deserialize)]
struct CreateOrder {
    customer_name: String,
    items: Vec<OrderItemReq>,
}

// -- State --

#[derive(Clone)]
struct AppState {
    pool: Arc<SqlxPool>,
}

// -- Handlers --

/// POST /products -- create product with optional description
async fn create_product(ctx: ServiceCtx, body: ShmSlice) -> HandlerResult<VilResponse<Product>> {
    let state = ctx.state::<AppState>().expect("state");
    let req: CreateProduct = body.json().map_err(|_| VilError::bad_request("invalid JSON"))?;
    let id = uuid::Uuid::new_v4().to_string();
    let stock = req.stock.unwrap_or(0);

    // Pattern: insert with value_opt_str for nullable columns
    Product::q()
        .insert_columns(&["id", "name", "description", "price", "stock", "category"])
        .value(id.clone())
        .value(req.name)
        .value_opt_str(req.description)
        .value(req.price)
        .value(stock)
        .value_opt_str(req.category)
        .execute(state.pool.inner())
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?;

    let product = Product::find_by_id(state.pool.inner(), &id)
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?
        .ok_or_else(|| VilError::internal("created but not found"))?;

    Ok(VilResponse::created(product))
}

/// GET /products -- list with pagination, ordered by created_at desc
async fn list_products(ctx: ServiceCtx) -> HandlerResult<VilResponse<Vec<Product>>> {
    let state = ctx.state::<AppState>().expect("state");

    // Pattern: select().where_raw("1=1").order_by_desc().limit()
    let products = Product::q()
        .select(&["id", "name", "description", "price", "stock", "category", "created_at"])
        .where_raw("1=1")
        .order_by_desc("created_at")
        .limit(50)
        .fetch_all::<Product>(state.pool.inner())
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?;

    Ok(VilResponse::ok(products))
}

/// GET /products/:id
async fn get_product(ctx: ServiceCtx, Path(id): Path<String>) -> HandlerResult<VilResponse<Product>> {
    let state = ctx.state::<AppState>().expect("state");

    // Pattern: T::find_by_id()
    let product = Product::find_by_id(state.pool.inner(), &id)
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?
        .ok_or_else(|| VilError::not_found("Product not found"))?;

    Ok(VilResponse::ok(product))
}

/// POST /orders -- create order with items, atomically decrement stock
async fn create_order(ctx: ServiceCtx, body: ShmSlice) -> HandlerResult<VilResponse<Order>> {
    let state = ctx.state::<AppState>().expect("state");
    let req: CreateOrder = body.json().map_err(|_| VilError::bad_request("invalid JSON"))?;
    let pool = state.pool.inner();

    if req.items.is_empty() {
        return Err(VilError::bad_request("order must have at least one item"));
    }

    let order_id = uuid::Uuid::new_v4().to_string();

    // Create order
    Order::q()
        .insert_columns(&["id", "customer_name", "status"])
        .value(order_id.clone())
        .value(req.customer_name)
        .value("pending".to_string())
        .execute(pool)
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?;

    // Create order items and decrement stock
    for item in &req.items {
        // Lookup product price
        let product = Product::find_by_id(pool, &item.product_id)
            .await
            .map_err(|e| VilError::internal(format!("{e}")))?
            .ok_or_else(|| VilError::bad_request(format!("product {} not found", item.product_id)))?;

        let item_id = uuid::Uuid::new_v4().to_string();

        OrderItem::q()
            .insert_columns(&["id", "order_id", "product_id", "quantity", "unit_price"])
            .value(item_id)
            .value(order_id.clone())
            .value(item.product_id.clone())
            .value(item.quantity)
            .value(product.price)
            .execute(pool)
            .await
            .map_err(|e| VilError::internal(format!("{e}")))?;

        // Pattern: set_expr("stock", "stock - ?", qty) -- atomic stock decrement
        Product::q()
            .update()
            .set_expr("stock", "stock - ?", item.quantity)
            .where_eq("id", &item.product_id)
            .execute(pool)
            .await
            .map_err(|e| VilError::internal(format!("{e}")))?;
    }

    let order = Order::find_by_id(pool, &order_id)
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?
        .ok_or_else(|| VilError::internal("created but not found"))?;

    Ok(VilResponse::created(order))
}

/// GET /orders -- list orders with item count via JOIN
async fn list_orders(ctx: ServiceCtx) -> HandlerResult<VilResponse<Vec<OrderListItem>>> {
    let state = ctx.state::<AppState>().expect("state");

    // Pattern: select().join().group_by().fetch_all() -- JOINed aggregate listing
    let orders = Order::q()
        .select(&[
            "o.id",
            "o.customer_name",
            "o.status",
            "o.created_at",
            "CAST(COALESCE(COUNT(oi.id), 0) AS INTEGER) as item_count",
        ])
        .alias("o")
        .left_join("order_items oi", "oi.order_id = o.id")
        .group_by("o.id")
        .order_by_desc("o.created_at")
        .limit(50)
        .fetch_all::<OrderListItem>(state.pool.inner())
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?;

    Ok(VilResponse::ok(orders))
}

/// GET /orders/:id/total -- scalar aggregate: SUM(quantity * unit_price)
async fn order_total(
    ctx: ServiceCtx,
    Path(id): Path<String>,
) -> HandlerResult<VilResponse<serde_json::Value>> {
    let state = ctx.state::<AppState>().expect("state");
    let pool = state.pool.inner();

    // Verify order exists
    let _order = Order::find_by_id(pool, &id)
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?
        .ok_or_else(|| VilError::not_found("Order not found"))?;

    // Pattern: select_expr("SUM(quantity * unit_price)").scalar::<f64>()
    let total: f64 = OrderItem::q()
        .select_expr("COALESCE(SUM(quantity * unit_price), 0.0)")
        .where_eq("order_id", &id)
        .scalar::<f64>(pool)
        .await
        .unwrap_or(0.0);

    Ok(VilResponse::ok(serde_json::json!({
        "order_id": id,
        "total": total,
    })))
}

/// DELETE /orders/:id -- cancel order
async fn cancel_order(
    ctx: ServiceCtx,
    Path(id): Path<String>,
) -> HandlerResult<VilResponse<serde_json::Value>> {
    let state = ctx.state::<AppState>().expect("state");
    let pool = state.pool.inner();

    // Delete order items first
    OrderItem::delete_where(pool, "order_id = ?", &[&id])
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?;

    // Pattern: T::delete()
    let deleted = Order::delete(pool, &id)
        .await
        .map_err(|e| VilError::internal(format!("{e}")))?;

    if deleted {
        Ok(VilResponse::ok(serde_json::json!({"deleted": true, "id": id})))
    } else {
        Err(VilError::not_found("Order not found"))
    }
}

// -- Main --

#[tokio::main]
async fn main() {
    let pool = SqlxPool::connect("ecommerce", vil_db_sqlx::SqlxConfig::sqlite("sqlite:ecommerce.db?mode=rwc"))
        .await
        .expect("SQLite connect failed");

    pool.execute_raw(
        "CREATE TABLE IF NOT EXISTS products (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            price REAL NOT NULL,
            stock INTEGER DEFAULT 0,
            category TEXT,
            created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
        );
        CREATE TABLE IF NOT EXISTS orders (
            id TEXT PRIMARY KEY,
            customer_name TEXT NOT NULL,
            status TEXT DEFAULT 'pending',
            created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
        );
        CREATE TABLE IF NOT EXISTS order_items (
            id TEXT PRIMARY KEY,
            order_id TEXT NOT NULL REFERENCES orders(id),
            product_id TEXT NOT NULL REFERENCES products(id),
            quantity INTEGER NOT NULL,
            unit_price REAL NOT NULL
        );"
    )
    .await
    .expect("Migration failed");

    let state = AppState { pool: Arc::new(pool) };

    let shop_svc = ServiceProcess::new("shop")
        .endpoint(Method::POST, "/products", post(create_product))
        .endpoint(Method::GET, "/products", get(list_products))
        .endpoint(Method::GET, "/products/:id", get(get_product))
        .endpoint(Method::POST, "/orders", post(create_order))
        .endpoint(Method::GET, "/orders", get(list_orders))
        .endpoint(Method::GET, "/orders/:id/total", get(order_total))
        .endpoint(Method::DELETE, "/orders/:id", delete(cancel_order))
        .state(state);

    VilApp::new("vilorm-ecommerce")
        .port(8086)
        .observer(true)
        .service(shop_svc)
        .run()
        .await;
}
