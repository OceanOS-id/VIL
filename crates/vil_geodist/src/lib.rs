use serde_json::{json, Value};

pub fn geo_distance(args: &[Value]) -> Result<Value, String> {
    let lat1 = args
        .get(0)
        .and_then(|v| v.as_f64())
        .ok_or("geo_distance: lat1 required")?;
    let lng1 = args
        .get(1)
        .and_then(|v| v.as_f64())
        .ok_or("geo_distance: lng1 required")?;
    let lat2 = args
        .get(2)
        .and_then(|v| v.as_f64())
        .ok_or("geo_distance: lat2 required")?;
    let lng2 = args
        .get(3)
        .and_then(|v| v.as_f64())
        .ok_or("geo_distance: lng2 required")?;
    let unit = args.get(4).and_then(|v| v.as_str()).unwrap_or("km");

    let r_km = 6371.0; // Earth radius in km
    let d_lat = (lat2 - lat1).to_radians();
    let d_lng = (lng2 - lng1).to_radians();
    let a = (d_lat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (d_lng / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();
    let km = r_km * c;

    let distance = match unit {
        "mi" | "miles" => km * 0.621371,
        "m" | "meters" => km * 1000.0,
        "nm" | "nautical" => km * 0.539957,
        _ => km, // default km
    };

    Ok(json!({"distance": (distance * 100.0).round() / 100.0, "unit": unit}))
}

pub fn register_functions() -> Vec<(&'static str, fn(&[Value]) -> Result<Value, String>)> {
    vec![("geo_distance", geo_distance)]
}
