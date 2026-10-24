use crate::feed::Contact;
use chrono::{DateTime, Local, TimeDelta};

/// How long an aircraft can drop out of the feed before it counts as gone.
const LINGER: TimeDelta = TimeDelta::seconds(60);
/// An aircraft that comes back within this window doesn't notify again.
const RENOTIFY_AFTER: TimeDelta = TimeDelta::minutes(30);
const MAX_RECENT: usize = 50;

#[derive(Clone, Debug)]
pub struct Sighting {
    pub contact: Contact,
    pub last_seen: DateTime<Local>,
    pub closest_mi: f64,
}

#[derive(Default)]
pub struct Tracker {
    /// Aircraft in the zone, closest first.
    pub current: Vec<Sighting>,
    /// Aircraft that have left the zone, most recent first.
    pub recent: Vec<Sighting>,
}

impl Tracker {
    /// Applies a fresh set of contacts and returns the ones worth notifying about.
    pub fn update(&mut self, contacts: Vec<Contact>, now: DateTime<Local>) -> Vec<Contact> {
        let mut arrivals = Vec::new();

        for contact in contacts {
            let hex = &contact.aircraft.hex;
            if let Some(sighting) = self
                .current
                .iter_mut()
                .find(|s| &s.contact.aircraft.hex == hex)
            {
                sighting.closest_mi = sighting.closest_mi.min(contact.distance_mi);
                sighting.contact = contact;
                sighting.last_seen = now;
                continue;
            }

            let returning = self
                .recent
                .iter()
                .position(|s| &s.contact.aircraft.hex == hex)
                .map(|i| self.recent.remove(i));
            if !returning
                .as_ref()
                .is_some_and(|s| now - s.last_seen < RENOTIFY_AFTER)
            {
                arrivals.push(contact.clone());
            }
            self.current.push(Sighting {
                closest_mi: contact.distance_mi,
                contact,
                last_seen: now,
            });
        }

        let (gone, stayed): (Vec<_>, Vec<_>) = std::mem::take(&mut self.current)
            .into_iter()
            .partition(|s| now - s.last_seen > LINGER);
        self.current = stayed;
        self.current
            .sort_by(|a, b| a.contact.distance_mi.total_cmp(&b.contact.distance_mi));
        self.add_recent(gone);

        arrivals
    }

    /// Moves aircraft outside a newly shrunk zone straight to recent.
    pub fn shrink_to(&mut self, radius_mi: u32) {
        let (outside, inside): (Vec<_>, Vec<_>) = std::mem::take(&mut self.current)
            .into_iter()
            .partition(|s| s.contact.distance_mi > radius_mi as f64);
        self.current = inside;
        self.add_recent(outside);
    }

    fn add_recent(&mut self, sightings: Vec<Sighting>) {
        self.recent.splice(0..0, sightings);
        self.recent.sort_by_key(|s| std::cmp::Reverse(s.last_seen));
        self.recent.truncate(MAX_RECENT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feed::Aircraft;

    fn contact(hex: &str, distance_mi: f64) -> Contact {
        Contact {
            aircraft: Aircraft {
                hex: hex.into(),
                ..Default::default()
            },
            distance_mi,
            direction: "N",
        }
    }

    #[test]
    fn notifies_once_then_moves_to_recent() {
        let mut tracker = Tracker::default();
        let t0 = Local::now();

        assert_eq!(tracker.update(vec![contact("a", 5.0)], t0).len(), 1);
        assert_eq!(
            tracker
                .update(vec![contact("a", 2.0)], t0 + TimeDelta::seconds(10))
                .len(),
            0
        );
        assert_eq!(tracker.current[0].closest_mi, 2.0);

        // Briefly missing from the feed: still current.
        tracker.update(vec![], t0 + TimeDelta::seconds(40));
        assert_eq!(tracker.current.len(), 1);

        tracker.update(vec![], t0 + TimeDelta::seconds(80));
        assert!(tracker.current.is_empty());
        assert_eq!(tracker.recent.len(), 1);
    }

    #[test]
    fn quick_return_does_not_renotify() {
        let mut tracker = Tracker::default();
        let t0 = Local::now();
        tracker.update(vec![contact("a", 5.0)], t0);
        tracker.update(vec![], t0 + TimeDelta::minutes(2));

        let back = tracker.update(vec![contact("a", 5.0)], t0 + TimeDelta::minutes(5));
        assert!(back.is_empty());
        assert_eq!(tracker.current.len(), 1);
        assert!(tracker.recent.is_empty());

        tracker.update(vec![], t0 + TimeDelta::minutes(7));
        let later = tracker.update(vec![contact("a", 5.0)], t0 + TimeDelta::minutes(60));
        assert_eq!(later.len(), 1);
    }

    #[test]
    fn shrinking_moves_outside_aircraft_to_recent() {
        let mut tracker = Tracker::default();
        tracker.update(
            vec![contact("near", 3.0), contact("far", 8.0)],
            Local::now(),
        );

        tracker.shrink_to(5);
        assert_eq!(tracker.current[0].contact.aircraft.hex, "near");
        assert_eq!(tracker.current.len(), 1);
        assert_eq!(tracker.recent[0].contact.aircraft.hex, "far");
    }
}
