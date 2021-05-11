#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use darkforest::{mimc, threshold, ChunkFootprint, Coords, Planet};
use itertools::iproduct;
use rayon::prelude::*;
use rocket::config::{Config, ConfigError, Environment};
use rocket::http::Method;
use rocket_contrib::json::Json;
use rocket_cors::{catch_all_options_routes, AllowedHeaders, AllowedOrigins};
use serde::{Deserialize, Serialize};
use std::env;

#[post("/mine", data = "<task>")]
fn mine(task: Json<Task>) -> Json<Response> {
    let x = task.chunkFootprint.bottomLeft.x;
    let y = task.chunkFootprint.bottomLeft.y;
    let size = task.chunkFootprint.sideLength;
    let key = task.planetHashKey;
    let rarity = task.planetRarity;

    let threshold = threshold(rarity);

    let planets = iproduct!(x..(x + size), y..(y + size))
        .par_bridge()
        .filter_map(|(xi, yi)| {
            let hash = mimc(xi, yi, key);
            if hash < threshold {
                Some(Planet {
                    coords: Coords { x: xi, y: yi },
                    hash: hash.to_string(),
                })
            } else {
                None
            }
        })
        .collect::<Vec<Planet>>();

    Json(Response {
        chunkFootprint: task.chunkFootprint.clone(),
        planetLocations: planets,
    })
}

fn main() -> Result<(), ConfigError> {
    let key = "PORT";
    let port: u16 = match env::var(key) {
        Ok(val) => val.parse::<u16>().unwrap(),
        Err(_) => 8000,
    };

    let allowed_origins = AllowedOrigins::all();
    let cors = rocket_cors::CorsOptions {
        allowed_origins,
        allowed_methods: vec![Method::Post].into_iter().map(From::from).collect(),
        allowed_headers: AllowedHeaders::all(),
        allow_credentials: true,
        ..Default::default()
    }
    .to_cors()
    .unwrap();
    let options_routes = catch_all_options_routes();

    let config = Config::build(Environment::Staging)
        .address("0.0.0.0")
        .port(port)
        .finalize()?;

    rocket::custom(config)
        .mount("/", routes![mine])
        .mount("/", options_routes)
        .manage(cors.clone())
        .attach(cors)
        .launch();

    Ok(())
}

#[allow(non_snake_case)]
#[derive(Deserialize)]
pub struct Task {
    pub chunkFootprint: ChunkFootprint,
    pub planetRarity: u32,
    pub planetHashKey: u32,
}

#[allow(non_snake_case)]
#[derive(Serialize)]
pub struct Response {
    pub chunkFootprint: ChunkFootprint,
    pub planetLocations: Vec<Planet>,
}
