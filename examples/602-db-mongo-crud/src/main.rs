// =============================================================================
// example-602-db-mongo-crud — MongoDB CRUD with db_log! auto-emit
// =============================================================================
//
// Demonstrates:
//   - MongoClient::new() with a local MongoDB URI
//   - insert_one, find_one, update_one, delete_one
//   - db_log! auto-emitted by vil_db_mongo on every operation
//   - StdoutDrain::resolved() output
//
// Requires: MongoDB running locally.
// Quick start:
//   docker run -p 27017:27017 mongo:7
//
// Without Docker, this example prints config and exits gracefully.
// =============================================================================

use serde::{Deserialize, Serialize};
use vil_db_mongo::{MongoClient, MongoConfig};
use vil_log::drain::{StdoutDrain, StdoutFormat};
use vil_log::runtime::init_logging;
use vil_log::{LogConfig, LogLevel};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Product {
    name:     String,
    price:    u32,
    in_stock: bool,
}

#[tokio::main]
async fn main() {
    // ── Init vil_log with resolved drain ──
    let config = LogConfig {
        ring_slots:        4096,
        level:             LogLevel::Info,
        batch_size:        64,
        flush_interval_ms: 50,
        threads:           None,
    };
    let _task = init_logging(config, StdoutDrain::new(StdoutFormat::Resolved));

    println!();
    println!("  example-602-db-mongo-crud");
    println!("  MongoDB CRUD operations with db_log! auto-emit");
    println!();

    let mongo_cfg = MongoConfig::new("mongodb://localhost:27017", "vil_demo");

    println!("  Connecting to MongoDB: mongodb://localhost:27017");
    println!("  Database: vil_demo");
    println!();
    println!("  NOTE: Requires MongoDB running locally.");
    println!("  Start with:  docker run -p 27017:27017 mongo:7");
    println!();

    let client = match MongoClient::new(mongo_cfg).await {
        Ok(c) => c,
        Err(e) => {
            println!("  [SKIP] Cannot connect to MongoDB: {:?}", e);
            println!("  (All db_log! calls would appear above in resolved format)");
            return;
        }
    };

    let product = Product {
        name:     "Laptop Pro X".into(),
        price:    15_000_000,
        in_stock: true,
    };

    // ── INSERT ──
    let id = match client.insert_one("products", &product).await {
        Ok(id) => {
            println!("  INSERT  products  id={}", id);
            id
        }
        Err(e) => {
            println!("  INSERT  error: {:?}", e);
            return;
        }
    };

    // ── FIND ONE ──
    let filter = bson::doc! { "_id": &id };
    match client.find_one::<Product>("products", filter.clone()).await {
        Ok(Some(p)) => println!("  FIND    products  name={} price={}", p.name, p.price),
        Ok(None)    => println!("  FIND    not found"),
        Err(e)      => println!("  FIND    error: {:?}", e),
    }

    // ── UPDATE ──
    let update = bson::doc! { "$set": { "price": 14_500_000u32 } };
    match client.update_one("products", filter.clone(), update).await {
        Ok(n) => println!("  UPDATE  products  modified={}", n),
        Err(e) => println!("  UPDATE  error: {:?}", e),
    }

    // ── DELETE ──
    match client.delete_one("products", filter).await {
        Ok(n) => println!("  DELETE  products  deleted={}", n),
        Err(e) => println!("  DELETE  error: {:?}", e),
    }

    // Allow drain to flush
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    println!();
    println!("  Done. db_log! entries emitted above in resolved format.");
    println!();
}
