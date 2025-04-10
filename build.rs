use serde::Deserialize;
use serde_json;
use std::{collections::HashMap, fs, path::Path};

#[derive(Deserialize)]
struct Listing {
    id: String,
    location_id: String,
    length: u32,
    width: u32,
    price_in_cents: u32,
}

fn main() {
    let json_path = Path::new("listings.json");
    let data = fs::read_to_string(json_path).expect("Unable to read listings.json");

    let listings: Vec<Listing> = serde_json::from_str(&data).expect("JSON was not well-formatted");

    let mut map: HashMap<String, Vec<Listing>> = HashMap::new();
    for listing in listings {
        map.entry(listing.location_id.clone())
            .or_default()
            .push(listing);
    }

    let mut output = String::new();
    output.push_str("use crate::Listing;\n");
    output.push_str("use phf_macros::phf_map;\n\n");
    output
        .push_str("pub static LISTINGS: phf::Map<&'static str, &'static [Listing]> = phf_map! {\n");

    for (location_id, listings_vec) in map {
        output.push_str(&format!("    {} => &[\n", format!("{:?}", location_id)));
        for listing in listings_vec {
            output.push_str(&format!(
                "        Listing {{ id: {:?}, location_id: {:?}, length: {}, width: {}, price_in_cents: {} }},\n",
                listing.id, listing.location_id, listing.length, listing.width, listing.price_in_cents
            ));
        }
        output.push_str("    ],\n");
    }
    output.push_str("};\n");

    fs::write("src/listings_data.rs", output).expect("Unable to write src/listings_data.rs");
}
