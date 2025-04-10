mod listings_data;

use std::collections::HashMap;

use axum::{
    http::StatusCode, routing::{get, post}, Json, Router
};
use listings_data::LISTINGS;
use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/", get(hello))
        .route("/", post(multi_vehicle_search));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3030").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn hello() -> String {
    "Hello!".to_string()
}

#[derive(Deserialize, Serialize, Debug)]
struct VehicleRequest {
    length: u32,
    quantity: u32,
}

#[derive(Deserialize, Serialize)]
struct Listing {
    id: &'static str,
    location_id: &'static str,
    length: u32,
    width: u32,
    price_in_cents: u32,
}

#[derive(Deserialize, Serialize, Debug)]
struct ResponseListing {
    location_id: &'static str,
    listing_ids: Vec<&'static str>,
    total_price_in_cents: u32,
}

async fn multi_vehicle_search(
    Json(vehicles): Json<Vec<VehicleRequest>>,
) -> Result<Json<Vec<ResponseListing>>, StatusCode> {
    tracing::info!("request: {:?}", vehicles);

    let valid_locations = get_valid_locations(&vehicles, &LISTINGS)?;
    Ok(Json(valid_locations))
}

fn get_valid_locations(
    vehicles_request: &[VehicleRequest],
    listings: &phf::Map<&'static str, &'static [Listing]>,
) -> Result<Vec<ResponseListing>, StatusCode> {
    let mut results = Vec::new();

    // split out vehicle requests into just the length for each vehicle
    // multiple vehicles of the same length get multiple entries so they
    // are counted properly
    let mut vehicle_lengths = vec![];
    for vehicle_request in vehicles_request {
        for _ in 0..vehicle_request.quantity {
            vehicle_lengths.push(vehicle_request.length);
        }
    }

    if vehicle_lengths.len() > 5 {
        return Err(StatusCode::BAD_REQUEST);
    }

    for (location_id, listings) in listings.entries() {
        if let Some((listing_ids, total_price)) = search_location(listings, &vehicle_lengths) {
            results.push(ResponseListing {
                location_id,
                listing_ids,
                total_price_in_cents: total_price,
            });
        }
    }

    results.sort_by_key(|response_listing| response_listing.total_price_in_cents);
    Ok(results)
}

fn search_location(
    listings: &[Listing],
    vehicle_lengths: &[u32],
) -> Option<(Vec<&'static str>, u32)> {
    let mut best_listings: Option<(Vec<&'static str>, u32)> = None;
    let mut assignment = vec![0; vehicle_lengths.len()];

    find_best_listings(
        0,
        &mut assignment,
        listings,
        vehicle_lengths,
        &mut best_listings,
    );
    best_listings
}

// this uses a backtracking algorithm to check for each possible assignment
// for each vehicle and tracks the set of listings that have the lowest
// total price
fn find_best_listings(
    vehicle_index: usize,
    listing_assignments: &mut [usize],
    listings: &[Listing],
    vehicles: &[u32],
    best_listings: &mut Option<(Vec<&'static str>, u32)>,
) {
    // the base case is when every vehicle has been mapped to a listing
    if vehicle_index == vehicles.len() {
        // tracks which vehicles are mapped to each listing index
        let mut listing_vehicle_map: HashMap<usize, Vec<u32>> = HashMap::new();
        for (vehicle, &listing_index) in vehicles.iter().zip(listing_assignments.iter()) {
            listing_vehicle_map
                .entry(listing_index)
                .or_default()
                .push(*vehicle);
        }

        // validate each listing assignment
        for (&listing_index, vehicles) in listing_vehicle_map.iter() {
            if !can_store(&listings[listing_index], vehicles) {
                return;
            }
        }

        // track the current best listing
        let total_price: u32 = listing_vehicle_map
            .keys()
            .map(|&idx| listings[idx].price_in_cents)
            .sum();
        let listing_ids: Vec<&'static str> = listing_vehicle_map
            .keys()
            .map(|&idx| listings[idx].id)
            .collect();
        if best_listings.is_none() || total_price < best_listings.as_ref().unwrap().1 {
            *best_listings = Some((listing_ids, total_price));
        }
        return;
    }

    // run the backtracking to try every combination of listing assignments
    for listing_index in 0..listings.len() {
        listing_assignments[vehicle_index] = listing_index;
        find_best_listings(
            vehicle_index + 1,
            listing_assignments,
            listings,
            vehicles,
            best_listings,
        );
    }
}

fn can_store(listing: &Listing, vehicle_lengths: &[u32]) -> bool {
    const VEHICLE_WIDTH: u32 = 10;

    let vehicles_per_row = listing.width / VEHICLE_WIDTH;
    if vehicles_per_row > 0 {
        let mut sorted = vehicle_lengths.to_vec();
        sorted.sort_unstable_by(|a, b| b.cmp(a));
        let mut total_height = 0;
        for row in sorted.chunks(vehicles_per_row as usize) {
            let row_height = *row.iter().max().unwrap();
            total_height += row_height;
        }
        if total_height <= listing.length {
            return true;
        }
    }

    let vehicles_per_column = listing.length / VEHICLE_WIDTH;
    if vehicles_per_column > 0 {
        let mut sorted = vehicle_lengths.to_vec();
        sorted.sort_unstable_by(|a, b| b.cmp(a));
        let mut total_width = 0;
        for column in sorted.chunks(vehicles_per_column as usize) {
            let col_width = *column.iter().max().unwrap();
            total_width += col_width;
        }
        if total_width <= listing.width {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use phf::{Map, phf_map};

    static TEST_LISTINGS: Map<&'static str, &'static [Listing]> = phf_map! {
        "location_id_1" => &[
            Listing {
                id: "listing_id_1",
                location_id: "location_id_1",
                length: 30,
                width: 10,
                price_in_cents: 1000,
            },
            Listing {
                id: "listing_id_2",
                location_id: "location_id_1",
                length: 20,
                width: 10,
                price_in_cents: 800,
            },
        ],
        "location_id_2" => &[
            Listing {
                id: "listing_id_3",
                location_id: "location_id_2",
                length: 10,
                width: 20,
                price_in_cents: 700,
            },
        ],
    };

    #[test]
    fn spec_example() {
        let vehicles = vec![VehicleRequest {
            length: 10,
            quantity: 1,
        }];
        let response = get_valid_locations(&vehicles, &LISTINGS).unwrap();
        assert_eq!(response.len(), 365);

        assert_eq!(
            response[0].location_id,
            "42b8f068-2d13-4ed1-8eec-c98f1eef0850"
        );
        assert_eq!(
            response[0].listing_ids,
            vec!["b9bbe25f-5679-4917-bd7b-1e19c464f3a8"]
        );
        assert_eq!(response[0].total_price_in_cents, 1005);

        assert_eq!(
            response[1].location_id,
            "507628b8-163e-4e22-a6a3-6a16f8188928"
        );
        assert_eq!(
            response[1].listing_ids,
            vec!["e7d59481-b804-4565-b49b-d5beb7aec350"]
        );
        assert_eq!(response[1].total_price_in_cents, 1088);

        assert_eq!(
            response[364].location_id,
            "22ad1ab7-d49b-49d6-8c30-531599934639"
        );
        assert_eq!(
            response[364].listing_ids,
            vec!["20cf6f5e-eb47-4104-b1f9-62527760a4c0"]
        );
        assert_eq!(response[364].total_price_in_cents, 99303);
    }

    #[test]
    fn no_fit() {
        let vehicles = vec![VehicleRequest {
            length: 100,
            quantity: 1,
        }];
        let result = get_valid_locations(&vehicles, &LISTINGS).unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn fits_rotated() {
        let vehicles = vec![VehicleRequest {
            length: 10,
            quantity: 2,
        }];
        let result = get_valid_locations(&vehicles, &TEST_LISTINGS).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].location_id, "location_id_2"); // location 2 is cheaper
        assert_eq!(result[1].location_id, "location_id_1");
    }

    #[test]
    fn one_location_multiple_fit_options() {
        let vehicles = vec![VehicleRequest {
            length: 10,
            quantity: 1,
        }];
        let result = get_valid_locations(&vehicles, &TEST_LISTINGS).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result[0].listing_ids.len() == 1);
    }

    #[test]
    fn multiple_vehicles() {
        let vehicles = vec![
            VehicleRequest {
                length: 10,
                quantity: 1,
            },
            VehicleRequest {
                length: 20,
                quantity: 1,
            },
        ];
        let result = get_valid_locations(&vehicles, &TEST_LISTINGS).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].location_id, "location_id_1");
    }

    #[test]
    fn require_multiple_listings_in_a_location() {
        let vehicles = vec![
            VehicleRequest {
                length: 30,
                quantity: 1,
            },
            VehicleRequest {
                length: 20,
                quantity: 1,
            },
        ];
        let result = get_valid_locations(&vehicles, &TEST_LISTINGS).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].location_id, "location_id_1");
    }
    #[test]
    fn fit_multiple_types_of_vehicles_into_single_listing() {
        let vehicles = vec![
            VehicleRequest {
                length: 10,
                quantity: 1,
            },
            VehicleRequest {
                length: 15,
                quantity: 1,
            },
        ];
        let result = get_valid_locations(&vehicles, &TEST_LISTINGS).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].location_id, "location_id_1");
        assert_eq!(result[0].listing_ids, vec!["listing_id_1"]);
    }

    #[test]
    fn three_vehicles_two_listings_one_location() {
        let vehicles = vec![
            VehicleRequest {
                length: 10,
                quantity: 2,
            },
            VehicleRequest {
                length: 15,
                quantity: 1,
            },
        ];
        let result = get_valid_locations(&vehicles, &TEST_LISTINGS).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].location_id, "location_id_1");
        assert_eq!(result[0].listing_ids.len(), 2);
    }

    #[test]
    fn too_many_vehicles() {
        let vehicles = vec![VehicleRequest {
            length: 5,
            quantity: 6,
        }];
        let result = get_valid_locations(&vehicles, &TEST_LISTINGS);
        assert!(result.is_err());
    }

    #[test]
    fn two_vehicles_same_size_two_listings() {
        let reqs = vec![VehicleRequest {
            length: 20,
            quantity: 2,
        }];
        let result = get_valid_locations(&reqs, &TEST_LISTINGS).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].location_id, "location_id_1");
        assert_eq!(result[0].listing_ids.len(), 2);
    }

    static CHEAP_SECOND_LOCATION_LISTINGS: Map<&'static str, &'static [Listing]> = phf_map! {
        "location_id_1" => &[
            Listing {
                id: "listing_id_1",
                location_id: "location_id_1",
                length: 100,
                width: 100,
                price_in_cents: 1000,
            },
            Listing {
                id: "listing_id_2",
                location_id: "location_id_1",
                length: 100,
                width: 100,
                price_in_cents: 800,
            },
        ],
    };

    #[test]
    fn cheaper_listing_gets_priority() {
        let reqs = vec![VehicleRequest {
            length: 100,
            quantity: 5,
        }];
        let result = get_valid_locations(&reqs, &CHEAP_SECOND_LOCATION_LISTINGS).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].location_id, "location_id_1");
        assert_eq!(result[0].listing_ids.len(), 1);
        assert_eq!(result[0].listing_ids, vec!["listing_id_2"]);
    }
}
