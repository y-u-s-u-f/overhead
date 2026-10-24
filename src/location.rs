//! Finding home: the Mac's current location, typed coordinates, or an address.

use serde::Deserialize;

pub type Coords = (f64, f64);

#[derive(Clone, Debug)]
pub struct Place {
    pub coords: Coords,
    pub label: String,
}

/// Accepts "51.47, -0.45" or "51.47 -0.45".
pub fn parse_coords(text: &str) -> Option<Coords> {
    let mut parts = text
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|p| !p.is_empty());
    let lat: f64 = parts.next()?.parse().ok()?;
    let lon: f64 = parts.next()?.parse().ok()?;
    let valid =
        parts.next().is_none() && (-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon);
    valid.then_some((lat, lon))
}

pub fn format_coords((lat, lon): Coords) -> String {
    let ns = if lat >= 0.0 { 'N' } else { 'S' };
    let ew = if lon >= 0.0 { 'E' } else { 'W' };
    format!("{:.4}° {ns}, {:.4}° {ew}", lat.abs(), lon.abs())
}

#[derive(Deserialize)]
struct NominatimPlace {
    lat: String,
    lon: String,
    display_name: String,
}

impl NominatimPlace {
    fn into_place(self) -> Option<Place> {
        Some(Place {
            coords: (self.lat.parse().ok()?, self.lon.parse().ok()?),
            label: self.display_name,
        })
    }
}

/// Looks an address up with OpenStreetMap's Nominatim.
pub fn geocode(agent: &ureq::Agent, query: &str) -> Result<Place, String> {
    let results: Vec<NominatimPlace> = agent
        .get("https://nominatim.openstreetmap.org/search")
        .query("q", query)
        .query("format", "jsonv2")
        .query("limit", "1")
        .call()
        .and_then(|mut r| r.body_mut().read_json())
        .map_err(|_| "Couldn't reach the address lookup. Check your connection.".to_string())?;
    results
        .into_iter()
        .next()
        .and_then(NominatimPlace::into_place)
        .ok_or_else(|| "No match. Try adding a city, or enter coordinates.".into())
}

/// A readable name for coordinates, if Nominatim knows one.
pub fn reverse_geocode(agent: &ureq::Agent, (lat, lon): Coords) -> Option<String> {
    agent
        .get("https://nominatim.openstreetmap.org/reverse")
        .query("lat", lat.to_string())
        .query("lon", lon.to_string())
        .query("format", "jsonv2")
        .query("zoom", "16")
        .call()
        .ok()?
        .body_mut()
        .read_json::<NominatimPlace>()
        .ok()
        .map(|p| p.display_name)
}

#[cfg(target_os = "macos")]
pub use core_location::locate;

/// Asks CoreLocation for one fix. The result arrives on the returned channel.
#[cfg(target_os = "macos")]
mod core_location {
    use super::Coords;
    use objc2::rc::Retained;
    use objc2::runtime::{NSObject, ProtocolObject};
    use objc2::{AllocAnyThread, define_class, msg_send};
    use objc2_core_location::{
        CLAuthorizationStatus, CLLocation, CLLocationManager, CLLocationManagerDelegate,
        kCLLocationAccuracyHundredMeters,
    };
    use objc2_foundation::{NSArray, NSError, NSObjectProtocol};
    use std::cell::RefCell;
    use std::sync::mpsc::{Receiver, Sender, channel};

    const DENIED: &str = "Location access is off for Overhead. Turn it on in System Settings → Privacy & Security → Location Services, or type your address.";

    type Reply = Result<Coords, String>;

    thread_local! {
        // The manager must outlive the request; it's replaced by the next one.
        static MANAGER: RefCell<Option<(Retained<CLLocationManager>, Retained<Delegate>)>> =
            const { RefCell::new(None) };
        static REPLY: RefCell<Option<Sender<Reply>>> = const { RefCell::new(None) };
    }

    fn reply(result: Reply) {
        if let Some(tx) = REPLY.with_borrow_mut(Option::take) {
            tx.send(result).ok();
        }
    }

    fn on_authorization(manager: &CLLocationManager) {
        let status = unsafe { manager.authorizationStatus() };
        if status == CLAuthorizationStatus::AuthorizedAlways
            || status == CLAuthorizationStatus::AuthorizedWhenInUse
        {
            unsafe { manager.requestLocation() };
        } else if status == CLAuthorizationStatus::NotDetermined {
            unsafe { manager.requestWhenInUseAuthorization() };
        } else {
            reply(Err(DENIED.into()));
        }
    }

    define_class!(
        #[unsafe(super(NSObject))]
        #[name = "OverheadLocationDelegate"]
        struct Delegate;

        unsafe impl NSObjectProtocol for Delegate {}

        unsafe impl CLLocationManagerDelegate for Delegate {
            #[unsafe(method(locationManagerDidChangeAuthorization:))]
            fn did_change_authorization(&self, manager: &CLLocationManager) {
                on_authorization(manager);
            }

            #[unsafe(method(locationManager:didUpdateLocations:))]
            fn did_update_locations(
                &self,
                _manager: &CLLocationManager,
                locations: &NSArray<CLLocation>,
            ) {
                if let Some(location) = locations.lastObject() {
                    let c = unsafe { location.coordinate() };
                    reply(Ok((c.latitude, c.longitude)));
                }
            }

            #[unsafe(method(locationManager:didFailWithError:))]
            fn did_fail(&self, _manager: &CLLocationManager, error: &NSError) {
                // kCLErrorDenied
                reply(Err(if error.code() == 1 {
                    DENIED.into()
                } else {
                    "Couldn't find your location. Try typing your address instead.".into()
                }));
            }
        }
    );

    pub fn locate() -> Receiver<Reply> {
        let (tx, rx) = channel();
        REPLY.set(Some(tx));
        let manager = unsafe { CLLocationManager::new() };
        let delegate: Retained<Delegate> = unsafe { msg_send![Delegate::alloc(), init] };
        unsafe {
            manager.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
            manager.setDesiredAccuracy(kCLLocationAccuracyHundredMeters);
        }
        on_authorization(&manager);
        MANAGER.set(Some((manager, delegate)));
        rx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_coordinates() {
        assert_eq!(parse_coords("51.47, -0.45"), Some((51.47, -0.45)));
        assert_eq!(parse_coords(" 51.47 -0.45 "), Some((51.47, -0.45)));
        assert_eq!(parse_coords("95, 10"), None);
        assert_eq!(parse_coords("1 Main St"), None);
        assert_eq!(parse_coords("33.8"), None);
    }

    #[test]
    fn formats_coordinates() {
        assert_eq!(format_coords((51.4700, -0.4543)), "51.4700° N, 0.4543° W");
    }
}
