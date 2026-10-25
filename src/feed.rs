use crate::location::Coords;
use serde::Deserialize;
use std::time::Duration;

pub const DEFAULT_RADIUS_MI: u32 = 10;
/// The feeds cap a query at 250 nautical miles.
pub const MAX_RADIUS_MI: u32 = 250;

const MILES_PER_NM: f64 = 1.150_78;
const EARTH_RADIUS_MI: f64 = 3958.8;
const MILITARY_FLAG: u32 = 1;

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Aircraft {
    pub hex: String,
    #[serde(rename = "flight")]
    pub callsign: Option<String>,
    #[serde(rename = "r")]
    pub registration: Option<String>,
    #[serde(rename = "t")]
    pub type_code: Option<String>,
    #[serde(rename = "desc")]
    pub description: Option<String>,
    #[serde(rename = "ownOp")]
    pub operator: Option<String>,
    #[serde(rename = "dbFlags", default)]
    pub db_flags: u32,
    pub alt_baro: Option<Altitude>,
    #[serde(rename = "gs")]
    pub ground_speed: Option<f64>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum Altitude {
    Feet(f64),
    /// The feeds send the string "ground" instead of a number.
    Ground(#[allow(dead_code)] String),
}

/// Barometric altitude bounds in feet. A missing bound is open.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AltitudeRange {
    pub min_ft: Option<u32>,
    pub max_ft: Option<u32>,
}

/// The highest altitude the settings accept.
pub const MAX_ALTITUDE_FT: u32 = 60_000;

impl AltitudeRange {
    /// Puts the bounds in order.
    pub fn sorted(self) -> Self {
        match (self.min_ft, self.max_ft) {
            (Some(min), Some(max)) if min > max => Self {
                min_ft: Some(max),
                max_ft: Some(min),
            },
            _ => self,
        }
    }

    /// Aircraft on the ground count as 0 ft. Ones that don't report altitude always pass.
    pub fn contains(&self, altitude: Option<&Altitude>) -> bool {
        let ft = match altitude {
            Some(Altitude::Feet(ft)) => *ft,
            Some(Altitude::Ground(_)) => 0.0,
            None => return true,
        };
        self.min_ft.is_none_or(|min| ft >= min as f64)
            && self.max_ft.is_none_or(|max| ft <= max as f64)
    }
}

impl Aircraft {
    pub fn callsign(&self) -> Option<&str> {
        self.callsign
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
    }

    pub fn name(&self) -> String {
        self.callsign()
            .or(self.registration.as_deref())
            .unwrap_or(&self.hex)
            .to_uppercase()
    }

    pub fn model(&self) -> Option<&str> {
        self.description.as_deref().or(self.type_code.as_deref())
    }

    pub fn flightradar_url(&self) -> String {
        match (self.callsign(), self.registration.as_deref()) {
            (Some(callsign), _) => format!("https://www.flightradar24.com/{callsign}"),
            (None, Some(reg)) => format!("https://www.flightradar24.com/data/aircraft/{reg}"),
            (None, None) => format!("https://www.flightradar24.com/{}", self.hex),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Contact {
    pub aircraft: Aircraft,
    pub distance_mi: f64,
    pub direction: &'static str,
}

#[derive(Deserialize)]
struct Response {
    #[serde(alias = "aircraft")]
    ac: Vec<Aircraft>,
}

struct Source {
    name: &'static str,
    url: fn(f64, f64, u32) -> String,
}

const SOURCES: [Source; 2] = [
    Source {
        name: "adsb.fi",
        url: |lat, lon, nm| {
            format!("https://opendata.adsb.fi/api/v2/lat/{lat}/lon/{lon}/dist/{nm}")
        },
    },
    Source {
        name: "adsb.lol",
        url: |lat, lon, nm| format!("https://api.adsb.lol/v2/point/{lat}/{lon}/{nm}"),
    },
];

pub fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(10)))
        .user_agent(concat!(
            "Overhead/",
            env!("CARGO_PKG_VERSION"),
            " (https://github.com/y-u-s-u-f/overhead)"
        ))
        .build()
        .into()
}

/// Military aircraft within the radius, closest first. Tries each source in order.
///
/// With `approximate`, the feeds only see home rounded to two decimals (about 1 km).
/// The query grows by a mile to cover the difference, and distances still use exact home.
pub fn fetch(
    agent: &ureq::Agent,
    home: Coords,
    radius_mi: u32,
    approximate: bool,
) -> Result<(&'static str, Vec<Contact>), String> {
    let mut radius_nm = (radius_mi as f64 / MILES_PER_NM).ceil() as u32;
    let (mut lat, mut lon) = home;
    if approximate {
        let round = |x: f64| (x * 100.0).round() / 100.0;
        (lat, lon) = (round(lat), round(lon));
        radius_nm += 1;
    }
    let mut errors = Vec::new();

    for source in &SOURCES {
        let url = (source.url)(lat, lon, radius_nm);
        match agent
            .get(&url)
            .call()
            .and_then(|mut r| r.body_mut().read_json::<Response>())
        {
            Ok(response) => {
                return Ok((source.name, military_contacts(response.ac, home, radius_mi)));
            }
            Err(err) => errors.push(format!("{}: {err}", source.name)),
        }
    }
    Err(errors.join("; "))
}

fn military_contacts(aircraft: Vec<Aircraft>, home: Coords, radius_mi: u32) -> Vec<Contact> {
    let mut contacts: Vec<Contact> = aircraft
        .into_iter()
        .filter(|a| a.db_flags & MILITARY_FLAG != 0)
        .filter_map(|aircraft| {
            let (lat, lon) = (aircraft.lat?, aircraft.lon?);
            let distance_mi = distance_mi(home, (lat, lon));
            (distance_mi <= radius_mi as f64).then(|| Contact {
                direction: compass(bearing(home, (lat, lon))),
                aircraft,
                distance_mi,
            })
        })
        .collect();
    contacts.sort_by(|a, b| a.distance_mi.total_cmp(&b.distance_mi));
    contacts
}

fn distance_mi((lat1, lon1): (f64, f64), (lat2, lon2): (f64, f64)) -> f64 {
    let (lat1, lat2) = (lat1.to_radians(), lat2.to_radians());
    let dlat = lat2 - lat1;
    let dlon = (lon2 - lon1).to_radians();
    let a = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
    2.0 * EARTH_RADIUS_MI * a.sqrt().asin()
}

fn bearing((lat1, lon1): (f64, f64), (lat2, lon2): (f64, f64)) -> f64 {
    let (lat1, lat2) = (lat1.to_radians(), lat2.to_radians());
    let dlon = (lon2 - lon1).to_radians();
    let y = dlon.sin() * lat2.cos();
    let x = lat1.cos() * lat2.sin() - lat1.sin() * lat2.cos() * dlon.cos();
    y.atan2(x).to_degrees().rem_euclid(360.0)
}

fn compass(degrees: f64) -> &'static str {
    const POINTS: [&str; 8] = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];
    POINTS[((degrees + 22.5) / 45.0) as usize % 8]
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOME: Coords = (40.0, -100.0);

    #[test]
    fn distance_and_direction() {
        // Roughly 10 miles due north of home.
        let north = (HOME.0 + 0.1447, HOME.1);
        assert!((distance_mi(HOME, north) - 10.0).abs() < 0.05);
        assert_eq!(compass(bearing(HOME, north)), "N");
        assert_eq!(compass(bearing(HOME, (HOME.0, HOME.1 - 0.1))), "W");
    }

    #[test]
    fn keeps_only_military_inside_radius() {
        let plane = |hex: &str, flags, lat| Aircraft {
            hex: hex.into(),
            db_flags: flags,
            lat: Some(lat),
            lon: Some(HOME.1),
            ..Default::default()
        };
        let contacts = military_contacts(
            vec![
                plane("far", 1, HOME.0 + 0.5),
                plane("civil", 0, HOME.0),
                plane("near", 1, HOME.0 + 0.01),
            ],
            HOME,
            10,
        );
        let hexes: Vec<_> = contacts.iter().map(|c| c.aircraft.hex.as_str()).collect();
        assert_eq!(hexes, ["near"]);
    }

    #[test]
    fn altitude_range() {
        let feet = Altitude::Feet(5000.0);
        let ground = Altitude::Ground("ground".into());
        let below = AltitudeRange {
            min_ft: None,
            max_ft: Some(10_000),
        };
        let above = AltitudeRange {
            min_ft: Some(1000),
            max_ft: None,
        };
        assert!(AltitudeRange::default().contains(Some(&ground)));
        assert!(below.contains(Some(&feet)) && below.contains(Some(&ground)));
        assert!(above.contains(Some(&feet)) && !above.contains(Some(&ground)));
        assert!(!below.contains(Some(&Altitude::Feet(35_000.0))));
        assert!(above.contains(None));
        assert_eq!(
            AltitudeRange {
                min_ft: Some(9000),
                max_ft: Some(2000)
            }
            .sorted(),
            AltitudeRange {
                min_ft: Some(2000),
                max_ft: Some(9000)
            }
        );
    }

    #[test]
    fn parses_ground_altitude() {
        let json =
            r#"{"aircraft":[{"hex":"ae1","alt_baro":"ground"},{"hex":"ae2","alt_baro":1200}]}"#;
        let response: Response = serde_json::from_str(json).unwrap();
        assert!(matches!(response.ac[0].alt_baro, Some(Altitude::Ground(_))));
        assert!(matches!(response.ac[1].alt_baro, Some(Altitude::Feet(f)) if f == 1200.0));
    }
}
